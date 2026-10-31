//! Current-source selection (docs/modules/media-hub.md §5). Pure and unit-tested.

use super::dto::PlaybackStatus;

#[derive(Debug, Clone)]
pub struct Candidate<'a> {
    pub id: &'a str,
    pub status: PlaybackStatus,
    /// Last time (epoch ms) this source entered `Playing`; 0 if never.
    pub last_playing_at: u64,
}

/// Picks the source to show:
/// 1. the pinned one if it still exists;
/// 2. the source that most recently started playing, among those playing;
/// 3. the most recently playing paused source, if within `paused_window_ms`.
pub fn select_source<'a>(
    candidates: &[Candidate<'a>],
    pinned: Option<&str>,
    now_ms: u64,
    paused_window_ms: u64,
) -> Option<&'a str> {
    if let Some(pinned) = pinned {
        if let Some(c) = candidates.iter().find(|c| c.id == pinned) {
            return Some(c.id);
        }
    }
    let playing = candidates
        .iter()
        .filter(|c| c.status == PlaybackStatus::Playing)
        .max_by_key(|c| c.last_playing_at);
    if let Some(c) = playing {
        return Some(c.id);
    }
    candidates
        .iter()
        .filter(|c| matches!(c.status, PlaybackStatus::Paused | PlaybackStatus::Changing))
        .filter(|c| {
            c.last_playing_at > 0 && now_ms.saturating_sub(c.last_playing_at) <= paused_window_ms
        })
        .max_by_key(|c| c.last_playing_at)
        .map(|c| c.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use PlaybackStatus::{Paused, Playing, Stopped};

    const WINDOW: u64 = 30 * 60 * 1000;

    fn c(id: &str, status: PlaybackStatus, at: u64) -> Candidate<'_> {
        Candidate {
            id,
            status,
            last_playing_at: at,
        }
    }

    #[test]
    fn nothing_when_empty() {
        assert_eq!(select_source(&[], None, 1_000, WINDOW), None);
    }

    #[test]
    fn most_recent_playing_wins() {
        let list = [c("spotify", Playing, 100), c("chrome", Playing, 200)];
        assert_eq!(select_source(&list, None, 300, WINDOW), Some("chrome"));
    }

    #[test]
    fn playing_beats_more_recent_paused() {
        let list = [c("spotify", Playing, 100), c("chrome", Paused, 900)];
        assert_eq!(select_source(&list, None, 1_000, WINDOW), Some("spotify"));
    }

    #[test]
    fn paused_within_window_is_kept() {
        let list = [c("spotify", Paused, 1_000)];
        assert_eq!(
            select_source(&list, None, 1_000 + WINDOW, WINDOW),
            Some("spotify")
        );
        assert_eq!(select_source(&list, None, 1_001 + WINDOW, WINDOW), None);
    }

    #[test]
    fn never_played_paused_source_is_ignored() {
        let list = [c("app", Paused, 0), c("other", Stopped, 50)];
        assert_eq!(select_source(&list, None, 100, WINDOW), None);
    }

    #[test]
    fn pinned_source_wins_while_it_exists() {
        let list = [c("spotify", Playing, 900), c("web", Paused, 100)];
        assert_eq!(
            select_source(&list, Some("web"), 1_000, WINDOW),
            Some("web")
        );
        assert_eq!(
            select_source(&list, Some("gone"), 1_000, WINDOW),
            Some("spotify")
        );
    }
}
