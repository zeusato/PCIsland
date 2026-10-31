//! Default render endpoint volume notifications (docs/modules/system-indicators.md §5).

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    AUDIO_VOLUME_NOTIFICATION_DATA, IMMDeviceEnumerator, MMDeviceEnumerator, eMultimedia, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
};
use windows::core::implement;

/// How often to check whether the default output device changed.
const DEVICE_CHECK: Duration = Duration::from_secs(3);

#[implement(IAudioEndpointVolumeCallback)]
struct Callback(Sender<(f32, bool)>);

impl IAudioEndpointVolumeCallback_Impl for Callback_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        // SAFETY: the OS passes a valid pointer for the duration of the call.
        if let Some(d) = unsafe { data.as_ref() } {
            let _ = self.0.send((d.fMasterVolume, d.bMuted.as_bool()));
        }
        Ok(())
    }
}

/// Starts the `audio-endpoint` thread. `on_change(level 0..1, muted, initial)`.
pub fn spawn<F>(on_change: F)
where
    F: FnMut(f32, bool, bool) + Send + 'static,
{
    std::thread::Builder::new()
        .name("audio-endpoint".into())
        .spawn(move || {
            let mut on_change = on_change;
            if let Err(err) = run(&mut on_change) {
                tracing::warn!(module = "indicators", error = %err, "volume watcher stopped");
            }
        })
        .expect("spawn audio-endpoint thread");
}

fn run<F: FnMut(f32, bool, bool)>(on_change: &mut F) -> windows::core::Result<()> {
    // SAFETY: COM init for this dedicated thread; all objects below live on it.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok()?;
    // SAFETY: standard COM activation of the MMDevice enumerator.
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
    let (tx, rx) = mpsc::channel();
    loop {
        // SAFETY: COM calls on interfaces owned by this thread; the id string is freed below.
        let (endpoint, device_id) = unsafe {
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let id_ptr = device.GetId()?;
            let id = id_ptr.to_string().unwrap_or_default();
            CoTaskMemFree(Some(id_ptr.0 as *const _));
            let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
            (endpoint, id)
        };
        let callback: IAudioEndpointVolumeCallback = Callback(tx.clone()).into();
        // SAFETY: registering/unregistering our callback on the endpoint we hold.
        unsafe { endpoint.RegisterControlChangeNotify(&callback)? };
        // SAFETY: plain getters.
        let (level, muted) = unsafe {
            (
                endpoint.GetMasterVolumeLevelScalar()?,
                endpoint.GetMute()?.as_bool(),
            )
        };
        on_change(level, muted, true);

        loop {
            match rx.recv_timeout(DEVICE_CHECK) {
                Ok((level, muted)) => on_change(level, muted, false),
                Err(RecvTimeoutError::Timeout) => {
                    // SAFETY: as above; the returned string is freed immediately.
                    let current = unsafe {
                        enumerator
                            .GetDefaultAudioEndpoint(eRender, eMultimedia)
                            .ok()
                            .and_then(|d| {
                                let p = d.GetId().ok()?;
                                let s = p.to_string().ok();
                                CoTaskMemFree(Some(p.0 as *const _));
                                s
                            })
                    };
                    if current.as_deref() != Some(device_id.as_str()) {
                        break;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
        // SAFETY: same endpoint/callback pair registered above.
        unsafe {
            let _ = endpoint.UnregisterControlChangeNotify(&callback);
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

/// Battery and AC state (`None` when the machine has no battery).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerStatus {
    pub on_ac: bool,
    pub percent: Option<u8>,
    pub seconds_left: Option<u32>,
}

pub fn power_status() -> Option<PowerStatus> {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut s = SYSTEM_POWER_STATUS::default();
    // SAFETY: `s` is a valid out-struct.
    unsafe { GetSystemPowerStatus(&mut s) }.ok()?;
    let no_battery = s.BatteryFlag & 128 != 0 || s.BatteryFlag == 255;
    Some(PowerStatus {
        on_ac: s.ACLineStatus == 1,
        percent: (!no_battery && s.BatteryLifePercent <= 100).then_some(s.BatteryLifePercent),
        seconds_left: (s.BatteryLifeTime != u32::MAX).then_some(s.BatteryLifeTime),
    })
}
