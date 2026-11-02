//! The only place allowed to call Win32 / WinRT directly or use `unsafe`
//! (docs/03-coding-principles.md §1, rule 6).

pub mod audio_endpoint;
pub mod autostart;
pub mod clipboard;
pub mod files;
pub mod foreground;
pub mod gsmtc;
pub mod loopback;
pub mod maintenance;
pub mod meeting_audio;
pub mod memory;
pub mod process;
pub mod shell;
pub mod smtc;
pub mod toast;
pub mod webview_drop;
pub mod window;
