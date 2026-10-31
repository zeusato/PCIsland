//! Play queue with shuffle history and repeat modes (docs/modules/local-player.md §5). Pure.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Queue {
    pub items: Vec<String>,
    /// Index into `items` of the current track.
    pub current: Option<usize>,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    /// Indices still to play in shuffle mode.
    #[serde(default)]
    upcoming: Vec<usize>,
    /// Indices already played (for "previous").
    #[serde(default)]
    history: Vec<usize>,
    #[serde(default)]
    seed: u64,
}

impl Queue {
    pub fn set(&mut self, items: Vec<String>, start: usize) {
        self.items = items;
        self.history.clear();
        self.current = (!self.items.is_empty()).then(|| start.min(self.items.len() - 1));
        self.reshuffle();
    }

    pub fn current_item(&self) -> Option<&str> {
        self.current
            .and_then(|i| self.items.get(i))
            .map(String::as_str)
    }

    pub fn set_shuffle(&mut self, on: bool) {
        self.shuffle = on;
        self.reshuffle();
    }

    /// Inserts after the current track ("play next") or appends.
    pub fn enqueue(&mut self, ids: Vec<String>, next: bool) {
        let at = if next {
            self.current.map_or(self.items.len(), |c| c + 1)
        } else {
            self.items.len()
        };
        let n = ids.len();
        for (k, id) in ids.into_iter().enumerate() {
            self.items.insert(at + k, id);
        }
        let shift = |i: &mut usize| {
            if *i >= at {
                *i += n;
            }
        };
        self.upcoming.iter_mut().for_each(shift);
        self.history.iter_mut().for_each(shift);
        if next && self.shuffle {
            self.upcoming.extend((at..at + n).rev());
        } else if self.shuffle {
            self.upcoming.splice(0..0, at..at + n);
        }
        if self.current.is_none() && !self.items.is_empty() {
            self.current = Some(0);
        }
    }

    /// Advances after a track ended (`auto`) or on "next" (`!auto`).
    pub fn next(&mut self, auto: bool) -> Option<&str> {
        let cur = self.current?;
        if auto && self.repeat == RepeatMode::One {
            return self.current_item();
        }
        let next = if self.shuffle {
            if self.upcoming.is_empty() && self.repeat == RepeatMode::All {
                self.reshuffle_all();
            }
            self.upcoming.pop()
        } else if cur + 1 < self.items.len() {
            Some(cur + 1)
        } else if self.repeat == RepeatMode::All {
            Some(0)
        } else {
            None
        };
        match next {
            Some(n) => {
                self.history.push(cur);
                self.current = Some(n);
                self.current_item()
            }
            None => None,
        }
    }

    /// "Previous": back through shuffle history or one step in order.
    pub fn previous(&mut self) -> Option<&str> {
        let cur = self.current?;
        let prev = if self.shuffle {
            let p = self.history.pop();
            if p.is_some() {
                self.upcoming.push(cur);
            }
            p
        } else if cur > 0 {
            Some(cur - 1)
        } else if self.repeat == RepeatMode::All {
            Some(self.items.len() - 1)
        } else {
            None
        };
        self.current = Some(prev.unwrap_or(cur));
        self.current_item()
    }

    fn reshuffle(&mut self) {
        self.upcoming.clear();
        if !self.shuffle {
            return;
        }
        let cur = self.current;
        self.upcoming = (0..self.items.len()).filter(|i| Some(*i) != cur).collect();
        self.shuffle_upcoming();
    }

    fn reshuffle_all(&mut self) {
        self.upcoming = (0..self.items.len()).collect();
        self.shuffle_upcoming();
    }

    /// Fisher–Yates with a tiny xorshift (no rand dependency).
    fn shuffle_upcoming(&mut self) {
        if self.seed == 0 {
            self.seed = crate::ipc::now_ms() | 1;
        }
        for i in (1..self.upcoming.len()).rev() {
            self.seed ^= self.seed << 13;
            self.seed ^= self.seed >> 7;
            self.seed ^= self.seed << 17;
            let j = (self.seed % (i as u64 + 1)) as usize;
            self.upcoming.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: usize) -> Queue {
        let mut q = Queue::default();
        q.set((0..n).map(|i| i.to_string()).collect(), 0);
        q
    }

    #[test]
    fn in_order_stops_at_end_unless_repeat_all() {
        let mut a = q(3);
        assert_eq!(a.next(true), Some("1"));
        assert_eq!(a.next(true), Some("2"));
        assert_eq!(a.next(true), None);
        a.repeat = RepeatMode::All;
        assert_eq!(a.next(true), Some("0"));
    }

    #[test]
    fn repeat_one_only_repeats_on_auto_advance() {
        let mut a = q(3);
        a.repeat = RepeatMode::One;
        assert_eq!(a.next(true), Some("0"));
        assert_eq!(a.next(false), Some("1"));
    }

    #[test]
    fn shuffle_visits_every_track_once_and_previous_goes_back() {
        let mut a = q(6);
        a.set_shuffle(true);
        let mut seen = vec![a.current_item().map(str::to_owned).expect("current")];
        while let Some(id) = a.next(true) {
            seen.push(id.to_owned());
        }
        seen.sort();
        assert_eq!(seen, vec!["0", "1", "2", "3", "4", "5"]);
        let last = a.current_item().map(str::to_owned);
        let before = a.previous().map(str::to_owned);
        assert_ne!(before, last);
    }

    #[test]
    fn play_next_inserts_after_current() {
        let mut a = q(3);
        a.enqueue(vec!["x".into()], true);
        assert_eq!(a.next(false), Some("x"));
        assert_eq!(a.next(false), Some("1"));
    }
}
