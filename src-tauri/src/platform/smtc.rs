//! PCisland's own SystemMediaTransportControls, so keyboard media keys and the Windows
//! media flyout work for the local and web players (docs/modules/media-hub.md §5).

use windows::Foundation::TypedEventHandler;
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
    SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
};
use windows::Win32::System::WinRT::ISystemMediaTransportControlsInterop;
use windows::core::HSTRING;

use super::window::RawHwnd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    PlayPause,
    Next,
    Previous,
}

pub struct AppSmtc {
    controls: SystemMediaTransportControls,
}

// SAFETY: SystemMediaTransportControls is an agile WinRT object; its methods may be
// called from any thread.
unsafe impl Send for AppSmtc {}
unsafe impl Sync for AppSmtc {}

impl AppSmtc {
    /// Binds SMTC to `window` and forwards button presses to `on_button`.
    pub fn new<F>(window: RawHwnd, on_button: F) -> windows::core::Result<Self>
    where
        F: Fn(Button) + Send + Sync + 'static,
    {
        let interop = windows::core::factory::<
            SystemMediaTransportControls,
            ISystemMediaTransportControlsInterop,
        >()?;
        // SAFETY: `window` is the island HWND owned by this process for the app's lifetime.
        let controls: SystemMediaTransportControls =
            unsafe { interop.GetForWindow(window.hwnd())? };
        controls.SetIsPlayEnabled(true)?;
        controls.SetIsPauseEnabled(true)?;
        controls.SetIsNextEnabled(true)?;
        controls.SetIsPreviousEnabled(true)?;
        controls.ButtonPressed(&TypedEventHandler::<
            SystemMediaTransportControls,
            SystemMediaTransportControlsButtonPressedEventArgs,
        >::new(move |_, args| {
            if let Some(args) = args.as_ref() {
                match args.Button()? {
                    SystemMediaTransportControlsButton::Play
                    | SystemMediaTransportControlsButton::Pause => on_button(Button::PlayPause),
                    SystemMediaTransportControlsButton::Next => on_button(Button::Next),
                    SystemMediaTransportControlsButton::Previous => on_button(Button::Previous),
                    _ => {}
                }
            }
            Ok(())
        }))?;
        controls.SetIsEnabled(false)?;
        Ok(Self { controls })
    }

    /// Shows `title`/`artist` with the given state, or hides PCisland from the flyout.
    pub fn update(&self, info: Option<(&str, Option<&str>, bool)>) -> windows::core::Result<()> {
        match info {
            None => {
                self.controls
                    .SetPlaybackStatus(MediaPlaybackStatus::Stopped)?;
                self.controls.SetIsEnabled(false)
            }
            Some((title, artist, playing)) => {
                self.controls.SetIsEnabled(true)?;
                self.controls.SetPlaybackStatus(if playing {
                    MediaPlaybackStatus::Playing
                } else {
                    MediaPlaybackStatus::Paused
                })?;
                let updater = self.controls.DisplayUpdater()?;
                updater.SetType(MediaPlaybackType::Music)?;
                let music = updater.MusicProperties()?;
                music.SetTitle(&HSTRING::from(title))?;
                music.SetArtist(&HSTRING::from(artist.unwrap_or_default()))?;
                updater.Update()
            }
        }
    }
}
