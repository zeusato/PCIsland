//! WASAPI loopback capture of the default render device, mixed to mono f32
//! (docs/modules/lyrics-visualizer.md §B5). Runs on its own thread only while needed.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use windows::Win32::Media::Audio::{
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
    IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
};

/// Handle to a running capture; dropping it stops the thread.
pub struct Capture {
    stop: Arc<AtomicBool>,
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Starts capturing. `on_samples(mono, sample_rate)` is called ~every 15 ms.
/// `on_error` is called once if the device cannot be captured (exclusive mode, driver).
pub fn start<F, E>(mut on_samples: F, on_error: E) -> Capture
where
    F: FnMut(&[f32], u32) + Send + 'static,
    E: FnOnce(String) + Send + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    std::thread::Builder::new()
        .name("loopback".into())
        .spawn(move || {
            if let Err(err) = run(&flag, &mut on_samples) {
                on_error(err.to_string());
            }
        })
        .expect("spawn loopback thread");
    Capture { stop }
}

fn run<F: FnMut(&[f32], u32)>(stop: &AtomicBool, on_samples: &mut F) -> windows::core::Result<()> {
    // SAFETY: COM objects are created, used and released on this thread only; the mix
    // format pointer is freed with CoTaskMemFree; buffers are read within the
    // GetBuffer/ReleaseBuffer window and their length comes from WASAPI.
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        let format = client.GetMixFormat()?;
        let channels = usize::from((*format).nChannels.max(1));
        let rate = (*format).nSamplesPerSec;
        let float = (*format).wBitsPerSample == 32;
        let init = client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            1_000_000,
            0,
            format,
            None,
        );
        CoTaskMemFree(Some(format as *const _));
        init?;
        let capture: IAudioCaptureClient = client.GetService()?;
        client.Start()?;
        let mut mono = Vec::with_capacity(4096);
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(15));
            mono.clear();
            while capture.GetNextPacketSize()? > 0 {
                let mut data = std::ptr::null_mut();
                let mut frames = 0u32;
                let mut flags = 0u32;
                capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None)?;
                let n = frames as usize;
                if flags & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0 || data.is_null() {
                    mono.extend(std::iter::repeat_n(0.0, n));
                } else if float {
                    let s = std::slice::from_raw_parts(data.cast::<f32>(), n * channels);
                    mono.extend(
                        s.chunks_exact(channels)
                            .map(|c| c.iter().sum::<f32>() / channels as f32),
                    );
                } else {
                    let s = std::slice::from_raw_parts(data.cast::<i16>(), n * channels);
                    mono.extend(s.chunks_exact(channels).map(|c| {
                        c.iter().map(|v| f32::from(*v) / 32768.0).sum::<f32>() / channels as f32
                    }));
                }
                capture.ReleaseBuffer(frames)?;
            }
            if !mono.is_empty() {
                on_samples(&mono, rate);
            }
        }
        let _ = client.Stop();
    }
    Ok(())
}
