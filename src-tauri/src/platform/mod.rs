//! The only place allowed to call Win32 / WinRT directly or use `unsafe`
//! (docs/03-coding-principles.md §1, rule 6).

pub mod audio_endpoint;
pub mod foreground;
pub mod gsmtc;
pub mod loopback;
pub mod shell;
pub mod smtc;
pub mod toast;
pub mod window;
