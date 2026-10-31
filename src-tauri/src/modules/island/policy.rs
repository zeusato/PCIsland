//! Visibility policy: a pure function from context to `hidden | minimal | normal`
//! (docs/modules/island-window.md §5, rules in priority order).

use super::dto::VisibilityMode;
use crate::core::priority;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code, reason = "Never/All come from settings (roadmap P1.1)")]
pub enum AlertOverFullscreen {
    Never,
    Critical,
    All,
}

#[derive(Debug, Clone, Copy)]
pub struct PolicySettings {
    pub hide_in_fullscreen: bool,
    pub minimal_when_maximized: bool,
    /// `island.idle_display == "hidden"`.
    pub hide_when_idle: bool,
    pub alert_over_fullscreen: AlertOverFullscreen,
}

impl Default for PolicySettings {
    fn default() -> Self {
        Self {
            hide_in_fullscreen: true,
            minimal_when_maximized: true,
            hide_when_idle: false,
            alert_over_fullscreen: AlertOverFullscreen::Critical,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PolicyInput {
    pub session_locked: bool,
    pub user_hidden: bool,
    pub fullscreen: bool,
    pub maximized: bool,
    /// Highest priority among current activities (`core::priority`).
    pub top_priority: u8,
    /// An Alert (transient) is currently queued.
    pub has_alert: bool,
}

pub fn decide(input: PolicyInput, settings: PolicySettings) -> VisibilityMode {
    if input.session_locked || input.user_hidden {
        return VisibilityMode::Hidden;
    }
    if input.fullscreen && settings.hide_in_fullscreen {
        let critical = input.top_priority >= priority::CRITICAL;
        let show = match settings.alert_over_fullscreen {
            AlertOverFullscreen::Never => false,
            AlertOverFullscreen::Critical => critical,
            AlertOverFullscreen::All => critical || input.has_alert,
        };
        return if show {
            VisibilityMode::Normal
        } else {
            VisibilityMode::Hidden
        };
    }
    if settings.hide_when_idle && input.top_priority < priority::AMBIENT && !input.has_alert {
        return VisibilityMode::Hidden;
    }
    if input.maximized && settings.minimal_when_maximized && input.top_priority < priority::LIVE {
        return VisibilityMode::Minimal;
    }
    VisibilityMode::Normal
}

#[cfg(test)]
mod tests {
    use super::*;
    use VisibilityMode::{Hidden, Minimal, Normal};

    fn base() -> PolicyInput {
        PolicyInput::default()
    }

    #[test]
    fn idle_desktop_is_normal() {
        assert_eq!(decide(base(), PolicySettings::default()), Normal);
    }

    #[test]
    fn locked_or_user_hidden_always_wins() {
        let s = PolicySettings::default();
        let critical = PolicyInput {
            top_priority: priority::CRITICAL,
            ..base()
        };
        assert_eq!(
            decide(
                PolicyInput {
                    session_locked: true,
                    ..critical
                },
                s
            ),
            Hidden
        );
        assert_eq!(
            decide(
                PolicyInput {
                    user_hidden: true,
                    ..critical
                },
                s
            ),
            Hidden
        );
    }

    #[test]
    fn fullscreen_hides_unless_critical_is_allowed() {
        let s = PolicySettings::default();
        let fs = PolicyInput {
            fullscreen: true,
            ..base()
        };
        assert_eq!(decide(fs, s), Hidden);
        assert_eq!(
            decide(
                PolicyInput {
                    top_priority: priority::MEDIA,
                    ..fs
                },
                s
            ),
            Hidden
        );
        assert_eq!(
            decide(
                PolicyInput {
                    has_alert: true,
                    ..fs
                },
                s
            ),
            Hidden
        );
        assert_eq!(
            decide(
                PolicyInput {
                    top_priority: priority::CRITICAL,
                    ..fs
                },
                s
            ),
            Normal
        );
    }

    #[test]
    fn fullscreen_alert_settings() {
        let fs = PolicyInput {
            fullscreen: true,
            top_priority: priority::CRITICAL,
            ..base()
        };
        let never = PolicySettings {
            alert_over_fullscreen: AlertOverFullscreen::Never,
            ..Default::default()
        };
        assert_eq!(decide(fs, never), Hidden);
        let all = PolicySettings {
            alert_over_fullscreen: AlertOverFullscreen::All,
            ..Default::default()
        };
        let alert = PolicyInput {
            fullscreen: true,
            has_alert: true,
            ..base()
        };
        assert_eq!(decide(alert, all), Normal);
    }

    #[test]
    fn fullscreen_ignored_when_setting_off() {
        let s = PolicySettings {
            hide_in_fullscreen: false,
            ..Default::default()
        };
        assert_eq!(
            decide(
                PolicyInput {
                    fullscreen: true,
                    ..base()
                },
                s
            ),
            Normal
        );
    }

    #[test]
    fn maximized_window_makes_idle_island_minimal() {
        let s = PolicySettings::default();
        let max = PolicyInput {
            maximized: true,
            ..base()
        };
        assert_eq!(decide(max, s), Minimal);
        assert_eq!(
            decide(
                PolicyInput {
                    top_priority: priority::MEDIA,
                    ..max
                },
                s
            ),
            Minimal
        );
        assert_eq!(
            decide(
                PolicyInput {
                    top_priority: priority::LIVE,
                    ..max
                },
                s
            ),
            Normal
        );
        let off = PolicySettings {
            minimal_when_maximized: false,
            ..s
        };
        assert_eq!(decide(max, off), Normal);
    }

    #[test]
    fn hide_when_idle_keeps_ambient_and_alerts_visible() {
        let s = PolicySettings {
            hide_when_idle: true,
            ..Default::default()
        };
        assert_eq!(decide(base(), s), Hidden);
        assert_eq!(
            decide(
                PolicyInput {
                    top_priority: priority::AMBIENT,
                    ..base()
                },
                s
            ),
            Normal
        );
        assert_eq!(
            decide(
                PolicyInput {
                    has_alert: true,
                    ..base()
                },
                s
            ),
            Normal
        );
    }

    #[test]
    fn fullscreen_beats_maximized() {
        let s = PolicySettings::default();
        let both = PolicyInput {
            fullscreen: true,
            maximized: true,
            ..base()
        };
        assert_eq!(decide(both, s), Hidden);
    }
}
