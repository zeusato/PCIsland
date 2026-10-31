//! Windows toast used as a fallback when the island is hidden (fullscreen, user-hidden)
//! and a reminder fires (docs/modules/reminders.md §4).

use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::HSTRING;

/// AUMID that is always registered; used when PCisland runs portable or from `tauri dev`,
/// where no Start-menu shortcut carries our own AppUserModelID.
const POWERSHELL_AUMID: &str =
    r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe";

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The NSIS installer drops `uninstall.exe` next to the app and registers a shortcut
/// whose AUMID is the bundle identifier.
fn app_id(identifier: &str) -> String {
    let installed = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("uninstall.exe").exists()))
        .unwrap_or(false);
    if installed {
        identifier.to_owned()
    } else {
        POWERSHELL_AUMID.to_owned()
    }
}

/// Shows a toast on a short-lived thread; failures are logged, never surfaced.
pub fn show(identifier: &str, title: &str, body: &str) {
    let xml = format!(
        "<toast scenario=\"reminder\"><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual><actions><action content=\"OK\" arguments=\"dismiss\" activationType=\"system\"/></actions></toast>",
        escape(title),
        escape(body)
    );
    let aumid = app_id(identifier);
    std::thread::spawn(move || {
        // SAFETY: initialises COM for this short-lived thread only.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let result = (|| -> windows::core::Result<()> {
            let doc = XmlDocument::new()?;
            doc.LoadXml(&HSTRING::from(xml))?;
            let toast = ToastNotification::CreateToastNotification(&doc)?;
            ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(aumid))?.Show(&toast)
        })();
        if let Err(e) = result {
            tracing::warn!(module = "platform", error = %e, "toast failed");
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn escapes_xml() {
        assert_eq!(
            super::escape("a<b & \"c\">"),
            "a&lt;b &amp; &quot;c&quot;&gt;"
        );
    }
}
