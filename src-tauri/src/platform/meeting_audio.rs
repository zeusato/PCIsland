//! Dedicated WASAPI capture threads; no hooks into music/video playback.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use windows::Win32::{Media::Audio::*, System::Com::*};

pub struct Capture {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Capture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub fn start(
    system: bool,
    device_id: Option<String>,
    mut samples: impl FnMut(&[f32]) + Send + 'static,
    error: impl FnOnce(String) + Send + 'static,
) -> Result<Capture, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let thread = std::thread::Builder::new()
        .name(
            if system {
                "meeting-system"
            } else {
                "meeting-mic"
            }
            .into(),
        )
        .spawn(move || {
            // SAFETY: all COM objects and capture buffers stay on this thread. COM is
            // uninitialized only after run() drops all interfaces.
            let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok() }
                .map_err(|e| e.to_string());
            if let Err(e) = result {
                let _ = tx.send(Err(e));
                return;
            }
            let result = run(system, device_id.as_deref(), &flag, &mut samples, &tx);
            unsafe {
                CoUninitialize();
            }
            if let Err(e) = result {
                let message = e.to_string();
                let _ = tx.try_send(Err(message.clone()));
                error(message);
            }
        })
        .map_err(|e| e.to_string())?;
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(Capture {
            stop,
            thread: Some(thread),
        }),
        other => {
            stop.store(true, Ordering::Relaxed);
            let _ = thread.join();
            Err(match other {
                Ok(Err(e)) => e,
                _ => "Capture startup timed out".into(),
            })
        }
    }
}
fn run(
    system: bool,
    device_id: Option<&str>,
    stop: &AtomicBool,
    samples: &mut impl FnMut(&[f32]),
    ready: &std::sync::mpsc::SyncSender<Result<(), String>>,
) -> windows::core::Result<()> {
    // SAFETY: COM interfaces belong to this initialized thread. WASAPI converts
    // shared-mode audio to the declared mono 48 kHz float format. Buffers are
    // consumed strictly between GetBuffer and ReleaseBuffer.
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = if let Some(id) = device_id {
            let id: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
            enumerator.GetDevice(windows::core::PCWSTR(id.as_ptr()))?
        } else {
            enumerator.GetDefaultAudioEndpoint(if system { eRender } else { eCapture }, eConsole)?
        };
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        let format = WAVEFORMATEX {
            wFormatTag: 3,
            nChannels: 1,
            nSamplesPerSec: 48000,
            nAvgBytesPerSec: 192000,
            nBlockAlign: 4,
            wBitsPerSample: 32,
            cbSize: 0,
        };
        let flags = AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
            | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY
            | if system {
                AUDCLNT_STREAMFLAGS_LOOPBACK
            } else {
                0
            };
        client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 1_000_000, 0, &format, None)?;
        let capture: IAudioCaptureClient = client.GetService()?;
        client.Start()?;
        let _ = ready.send(Ok(()));
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(10));
            while capture.GetNextPacketSize()? > 0 {
                let mut data = std::ptr::null_mut();
                let mut frames = 0;
                let mut flags = 0;
                capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None)?;
                if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || data.is_null() {
                    samples(&vec![0.0; frames as usize]);
                } else {
                    samples(std::slice::from_raw_parts(
                        data.cast::<f32>(),
                        frames as usize,
                    ));
                }
                capture.ReleaseBuffer(frames)?;
            }
        }
        client.Stop()?;
    }
    Ok(())
}

#[derive(Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct MeetingDevice {
    pub id: String,
    pub name: String,
    pub system: bool,
}
pub fn devices() -> Result<Vec<MeetingDevice>, String> {
    std::thread::spawn(|| {
        // SAFETY: property values and COM strings are released on the creating thread.
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|e| e.to_string())?;
            let result = (|| -> windows::core::Result<Vec<MeetingDevice>> {
                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
                let key = windows::Win32::Foundation::PROPERTYKEY {
                    fmtid: windows::core::GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
                    pid: 14,
                };
                let mut result = Vec::new();
                for system in [false, true] {
                    let collection = enumerator.EnumAudioEndpoints(
                        if system { eRender } else { eCapture },
                        DEVICE_STATE_ACTIVE,
                    )?;
                    for index in 0..collection.GetCount()? {
                        let device = collection.Item(index)?;
                        let id_ptr = device.GetId()?;
                        let id = id_ptr.to_string()?;
                        CoTaskMemFree(Some(id_ptr.0.cast()));
                        let store = device.OpenPropertyStore(STGM_READ)?;
                        let mut prop = store.GetValue(&key)?;
                        let converted = StructuredStorage::PropVariantToStringAlloc(&prop);
                        let _ = StructuredStorage::PropVariantClear(&mut prop);
                        let name = match converted {
                            Ok(ptr) => {
                                let name = ptr.to_string().unwrap_or_else(|_| id.clone());
                                CoTaskMemFree(Some(ptr.0.cast()));
                                name
                            }
                            Err(_) => id.clone(),
                        };
                        result.push(MeetingDevice { id, name, system });
                    }
                }
                Ok(result)
            })();
            CoUninitialize();
            result.map_err(|e| e.to_string())
        }
    })
    .join()
    .map_err(|_| "Device enumeration stopped".to_owned())?
}
#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "native loopback initialization; no audio saved or uploaded"]
    fn system_endpoint_initialize() {
        let (tx, rx) = std::sync::mpsc::channel();
        let capture = super::start(
            true,
            None,
            |_| {},
            move |e| {
                let _ = tx.send(e);
            },
        )
        .expect("default loopback");
        std::thread::sleep(std::time::Duration::from_millis(300));
        drop(capture);
        assert!(rx.try_recv().is_err());
        let devices = super::devices().expect("enumerate devices");
        assert!(devices.iter().any(|d| d.system));
        println!("{} active audio devices", devices.len());
    }
    #[test]
    #[ignore = "native microphone permission required; no audio saved or uploaded"]
    fn microphone_endpoint_initialize() {
        let capture = super::start(false, None, |_| {}, |_| {}).expect("default microphone");
        drop(capture);
    }
}
