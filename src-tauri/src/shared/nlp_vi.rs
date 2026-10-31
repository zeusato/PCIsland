//! Parses Vietnamese (and basic English) reminder sentences such as
//! "họp team 3h chiều mai" or "uống nước mỗi 2 tiếng" (docs/modules/reminders.md §5, F25).
//! Pure library: no app dependencies.

use std::sync::LazyLock;

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use regex::Regex;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Repeat {
    None,
    Daily,
    Weekdays,
    /// Days as 0 = Monday … 6 = Sunday.
    Weekly {
        days: Vec<u8>,
    },
    Monthly {
        day: u8,
    },
    Every {
        minutes: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub title: String,
    pub due: Option<NaiveDateTime>,
    pub repeat: Repeat,
}

/// Lowercases and strips Vietnamese diacritics, keeping a 1:1 char mapping with the input.
pub fn normalize(s: &str) -> String {
    s.chars()
        .map(|c| fold(c.to_lowercase().next().unwrap_or(c)))
        .collect()
}

fn fold(c: char) -> char {
    const GROUPS: &[(&str, char)] = &[
        ("àáạảãâầấậẩẫăằắặẳẵ", 'a'),
        ("èéẹẻẽêềếệểễ", 'e'),
        ("ìíịỉĩ", 'i'),
        ("òóọỏõôồốộổỗơờớợởỡ", 'o'),
        ("ùúụủũưừứựửữ", 'u'),
        ("ỳýỵỷỹ", 'y'),
        ("đ", 'd'),
    ];
    GROUPS
        .iter()
        .find(|(set, _)| set.contains(c))
        .map_or(c, |(_, base)| *base)
}

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($pat).expect("valid regex"));
    };
}

const UNIT_H: &str = r"(?:gio|tieng|h|hours?|hrs?)";
const UNIT_M: &str = r"(?:phut|ph|p|minutes?|mins?|m)";

re!(
    EVERY,
    &format!(r"\b(?:moi|every)\s+(\d+)\s*({UNIT_H}|{UNIT_M})\b")
);
re!(
    DAILY,
    r"\b(?:moi ngay|hang ngay|hang ngay|ngay nao cung|every day|daily)\b"
);
re!(
    WEEKDAYS,
    r"\b(?:cac ngay trong tuan|ngay lam viec|cac ngay lam viec|thu 2 den thu 6|thu hai den thu sau|weekdays)\b"
);
re!(
    WEEKLY,
    r"\b(?:moi\s+)?(thu\s*(?:[2-7]|hai|ba|tu|nam|sau|bay)|chu nhat|cn)\s+(?:hang|moi)\s+tuan\b|\bmoi\s+(thu\s*(?:[2-7]|hai|ba|tu|nam|sau|bay)|chu nhat)\b"
);
re!(MONTHLY, r"\bngay\s+(\d{1,2})\s+(?:hang|moi)\s+thang\b");
re!(
    RELATIVE,
    &format!(
        r"\b(?:(?:sau|trong|in)\s+)?(?:(nua)\s+{UNIT_H}|(\d+)\s*{UNIT_H}\s*(?:(\d+)\s*{UNIT_M}?)?|(\d+)\s*{UNIT_M})\s*(nua|later)?\b"
    )
);
re!(
    CLOCK,
    r"\b(?:luc\s+|vao\s+|at\s+)?(\d{1,2})\s*(?::|h|g|gio)\s*(\d{1,2})?\s*(?:phut|p)?\s*(kem\s*(?:15|muoi lam))?\s*(ruoi)?\s*(am|pm|sang|trua|chieu|toi|dem)?\b|\b(\d{1,2})\s*(am|pm)\b"
);
re!(
    PERIOD_ONLY,
    r"\b(sang|trua|chieu|toi|dem)\s+(nay|mai)\b|\b(toi nay|trua nay|chieu nay|sang nay|nua dem)\b"
);
re!(
    DAY_WORD,
    r"\b(hom nay|today|ngay mai|tomorrow|mai|ngay kia|ngay mot|mot)\b"
);
re!(
    WEEKDAY,
    r"\b(thu\s*(?:[2-7]|hai|ba|tu|nam|sau|bay)|chu nhat|cn)(\s+tuan\s+(?:sau|toi))?\b"
);
re!(
    DATE,
    r"\b(\d{1,2})/(\d{1,2})(?:/(\d{2,4}))?\b|\bngay\s+(\d{1,2})\b"
);
re!(
    FILLER,
    r"^(?:nhac (?:toi|minh|em|anh)|nhac|nho|remind me to|remind me|hay)\s+|\s+(?:luc|vao|at|nhe)$"
);

fn weekday_from(word: &str) -> Option<Weekday> {
    let w: String = word.split_whitespace().collect::<Vec<_>>().join(" ");
    Some(match w.trim_start_matches("thu").trim() {
        "2" | "hai" => Weekday::Mon,
        "3" | "ba" => Weekday::Tue,
        "4" | "tu" => Weekday::Wed,
        "5" | "nam" => Weekday::Thu,
        "6" | "sau" => Weekday::Fri,
        "7" | "bay" => Weekday::Sat,
        _ if w == "chu nhat" || w == "cn" => Weekday::Sun,
        _ => return None,
    })
}

fn period_hour(period: &str, hour: u32) -> u32 {
    match period {
        "pm" | "chieu" | "toi" if hour < 12 => hour + 12,
        "trua" if hour < 11 => hour + 12,
        "dem" if (6..12).contains(&hour) => hour + 12,
        "am" | "sang" if hour == 12 => 0,
        _ => hour,
    }
}

fn default_time_for(period: &str) -> NaiveTime {
    let h = match period {
        "sang" => 9,
        "trua" => 12,
        "chieu" => 15,
        "toi" => 20,
        "dem" | "nua dem" => 23,
        _ => 9,
    };
    NaiveTime::from_hms_opt(h, 0, 0).unwrap_or_default()
}

struct Spans(Vec<(usize, usize)>);

impl Spans {
    fn add(&mut self, m: &regex::Match<'_>) {
        self.0.push((m.start(), m.end()));
    }
}

pub fn parse(input: &str, now: NaiveDateTime) -> Parsed {
    let norm = normalize(input);
    let mut spans = Spans(Vec::new());
    let mut repeat = Repeat::None;
    let mut date: Option<NaiveDate> = None;
    let mut time: Option<NaiveTime> = None;
    let mut period: Option<String> = None;
    let mut relative: Option<Duration> = None;
    let mut weekday: Option<(Weekday, bool)> = None;

    if let Some(c) = EVERY.captures(&norm) {
        let n: u32 = c[1].parse().unwrap_or(1);
        let minutes = if c[2].starts_with('p') || c[2].starts_with('m') {
            n
        } else {
            n * 60
        };
        repeat = Repeat::Every {
            minutes: minutes.max(1),
        };
        spans.add(&c.get(0).expect("match"));
    } else if let Some(m) = DAILY.find(&norm) {
        repeat = Repeat::Daily;
        spans.add(&m);
    } else if let Some(m) = WEEKDAYS.find(&norm) {
        repeat = Repeat::Weekdays;
        spans.add(&m);
    } else if let Some(c) = WEEKLY.captures(&norm) {
        let word = c
            .get(1)
            .or_else(|| c.get(2))
            .map(|m| m.as_str())
            .unwrap_or_default();
        if let Some(day) = weekday_from(word) {
            repeat = Repeat::Weekly {
                days: vec![day.num_days_from_monday() as u8],
            };
            weekday = Some((day, false));
        }
        spans.add(&c.get(0).expect("match"));
    } else if let Some(c) = MONTHLY.captures(&norm) {
        let day: u8 = c[1].parse().unwrap_or(1);
        repeat = Repeat::Monthly {
            day: day.clamp(1, 31),
        };
        spans.add(&c.get(0).expect("match"));
    }

    // Relative offsets only count when marked ("15p nữa", "sau 1 tiếng", "in 10 min").
    for c in RELATIVE.captures_iter(&norm) {
        let whole = c.get(0).expect("match");
        let marked = whole.as_str().starts_with("sau")
            || whole.as_str().starts_with("trong")
            || whole.as_str().starts_with("in")
            || c.get(5).is_some();
        if !marked
            || spans
                .0
                .iter()
                .any(|&(s, e)| whole.start() < e && s < whole.end())
        {
            continue;
        }
        let mins = if c.get(1).is_some() {
            30
        } else if let Some(h) = c.get(2) {
            h.as_str().parse::<i64>().unwrap_or(0) * 60
                + c.get(3).map_or(0, |m| m.as_str().parse().unwrap_or(0))
        } else {
            c.get(4).map_or(0, |m| m.as_str().parse().unwrap_or(0))
        };
        relative = Some(Duration::minutes(mins));
        spans.add(&whole);
        break;
    }

    if relative.is_none() {
        for c in CLOCK.captures_iter(&norm) {
            let whole = c.get(0).expect("match");
            if spans
                .0
                .iter()
                .any(|&(s, e)| whole.start() < e && s < whole.end())
            {
                continue;
            }
            let (h, m, kem, ruoi, p) = if let Some(h) = c.get(1) {
                (
                    h.as_str(),
                    c.get(2).map(|m| m.as_str()),
                    c.get(3).is_some(),
                    c.get(4).is_some(),
                    c.get(5).map(|m| m.as_str()),
                )
            } else {
                (
                    c.get(6).map(|m| m.as_str()).unwrap_or("0"),
                    None,
                    false,
                    false,
                    c.get(7).map(|m| m.as_str()),
                )
            };
            let mut hour: u32 = h.parse().unwrap_or(99);
            let mut minute: u32 =
                m.and_then(|m| m.parse().ok())
                    .unwrap_or(if ruoi { 30 } else { 0 });
            if kem {
                hour = hour.saturating_sub(1);
                minute = 45;
            }
            if hour > 23 || minute > 59 {
                continue;
            }
            if let Some(p) = p {
                hour = period_hour(p, hour);
                period = Some(p.to_owned());
            }
            time = NaiveTime::from_hms_opt(hour, minute, 0);
            spans.add(&whole);
            break;
        }
        if let Some(c) = PERIOD_ONLY.captures(&norm) {
            let whole = c.get(0).expect("match");
            let (p, day) = match (c.get(1), c.get(2), c.get(3)) {
                (Some(p), Some(d), _) => (p.as_str().to_owned(), d.as_str()),
                (_, _, Some(w)) => {
                    let w = w.as_str();
                    (w.split(' ').next().unwrap_or("").to_owned(), "nay")
                }
                _ => (String::new(), "nay"),
            };
            if day == "mai" {
                date = Some(now.date() + Duration::days(1));
            } else {
                date = Some(now.date());
            }
            if let Some(t) = time {
                let adjusted = period_hour(&p, t.hour_of_day());
                time = NaiveTime::from_hms_opt(adjusted, t.minute_of_hour(), 0);
            } else {
                time = Some(default_time_for(if whole.as_str() == "nua dem" {
                    "nua dem"
                } else {
                    &p
                }));
            }
            period = Some(p);
            spans.add(&whole);
        }
        if date.is_none() {
            if let Some(c) = DAY_WORD.captures(&norm) {
                let w = c.get(1).expect("group");
                date = Some(match w.as_str() {
                    "hom nay" | "today" => now.date(),
                    "ngay kia" | "ngay mot" | "mot" => now.date() + Duration::days(2),
                    _ => now.date() + Duration::days(1),
                });
                spans.add(&w);
            }
        }
        if date.is_none() {
            if let Some(c) = WEEKDAY.captures(&norm).filter(|c| {
                !spans
                    .0
                    .iter()
                    .any(|&(s, e)| c.get(0).is_some_and(|m| m.start() < e && s < m.end()))
            }) {
                if let Some(day) = weekday_from(c.get(1).expect("group").as_str()) {
                    weekday = Some((day, c.get(2).is_some()));
                }
                spans.add(&c.get(0).expect("match"));
            }
        }
        if date.is_none() {
            if let Some(c) = DATE.captures(&norm) {
                let today = now.date();
                if let (Some(d), Some(m)) = (c.get(1), c.get(2)) {
                    let d: u32 = d.as_str().parse().unwrap_or(1);
                    let m: u32 = m.as_str().parse().unwrap_or(1);
                    let y = c
                        .get(3)
                        .and_then(|y| y.as_str().parse::<i32>().ok())
                        .map(|y| if y < 100 { 2000 + y } else { y });
                    let mut candidate = NaiveDate::from_ymd_opt(y.unwrap_or(today.year()), m, d);
                    if y.is_none() && candidate.is_some_and(|c| c < today) {
                        candidate = NaiveDate::from_ymd_opt(today.year() + 1, m, d);
                    }
                    date = candidate;
                } else if let Some(d) = c.get(4) {
                    let d: u32 = d.as_str().parse().unwrap_or(1);
                    date = NaiveDate::from_ymd_opt(today.year(), today.month(), d)
                        .filter(|c| *c >= today)
                        .or_else(|| next_month_day(today, d));
                }
                spans.add(&c.get(0).expect("match"));
            }
        }
    }

    if let Some((day, next_week)) = weekday {
        if date.is_none() {
            let mut ahead = (7 + day.num_days_from_monday() as i64
                - now.weekday().num_days_from_monday() as i64)
                % 7;
            if next_week {
                ahead = (7 - now.weekday().num_days_from_monday() as i64)
                    + day.num_days_from_monday() as i64;
            }
            date = Some(now.date() + Duration::days(ahead));
        }
    }

    let due = if let Some(rel) = relative {
        Some(now + rel)
    } else {
        resolve(now, date, time, period.is_some(), &repeat)
    };

    Parsed {
        title: title(input, &spans.0),
        due,
        repeat,
    }
}

fn next_month_day(today: NaiveDate, d: u32) -> Option<NaiveDate> {
    let (y, m) = if today.month() == 12 {
        (today.year() + 1, 1)
    } else {
        (today.year(), today.month() + 1)
    };
    NaiveDate::from_ymd_opt(y, m, d)
}

trait HourMinute {
    fn hour_of_day(&self) -> u32;
    fn minute_of_hour(&self) -> u32;
}
impl HourMinute for NaiveTime {
    fn hour_of_day(&self) -> u32 {
        chrono::Timelike::hour(self)
    }
    fn minute_of_hour(&self) -> u32 {
        chrono::Timelike::minute(self)
    }
}

/// Combines date/time with the ambiguity rules of docs/modules/reminders.md §5.
fn resolve(
    now: NaiveDateTime,
    date: Option<NaiveDate>,
    time: Option<NaiveTime>,
    has_period: bool,
    repeat: &Repeat,
) -> Option<NaiveDateTime> {
    match (date, time) {
        (None, None) => match repeat {
            Repeat::Every { minutes } => Some(now + Duration::minutes(i64::from(*minutes))),
            Repeat::None => None,
            _ => Some(next_at(now, default_time_for(""))),
        },
        (Some(d), None) => Some(d.and_time(default_time_for(""))),
        (Some(d), Some(t)) => {
            let h = t.hour_of_day();
            // "mai 3h": 1–6 without a period means afternoon.
            let t = if !has_period && (1..=6).contains(&h) {
                t + Duration::hours(12)
            } else {
                t
            };
            Some(d.and_time(t))
        }
        (None, Some(t)) => {
            let h = t.hour_of_day();
            if !has_period && (1..=11).contains(&h) {
                // Nearest future among h and h+12, today or tomorrow.
                let am = now.date().and_time(t);
                let pm = am + Duration::hours(12);
                return Some(
                    [am, pm, am + Duration::days(1)]
                        .into_iter()
                        .find(|c| *c > now)
                        .unwrap_or(pm),
                );
            }
            Some(next_at(now, t))
        }
    }
}

fn next_at(now: NaiveDateTime, t: NaiveTime) -> NaiveDateTime {
    let today = now.date().and_time(t);
    if today > now {
        today
    } else {
        today + Duration::days(1)
    }
}

/// Removes recognised spans and filler words from the original text.
fn title(input: &str, spans: &[(usize, usize)]) -> String {
    // Byte offsets come from the normalized string, which maps 1:1 per char;
    // convert via char indices.
    let norm = normalize(input);
    let byte_to_char = |b: usize| norm[..b].chars().count();
    let chars: Vec<char> = input.chars().collect();
    let mut keep = vec![true; chars.len()];
    for &(s, e) in spans {
        for k in keep.iter_mut().take(byte_to_char(e)).skip(byte_to_char(s)) {
            *k = false;
        }
    }
    let raw: String = chars
        .iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(c, _)| *c)
        .collect();
    let mut text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    for _ in 0..3 {
        let n = normalize(&text);
        let Some(m) = FILLER.find(&n) else { break };
        let (s, e) = (n[..m.start()].chars().count(), n[..m.end()].chars().count());
        text = text
            .chars()
            .enumerate()
            .filter(|(i, _)| *i < s || *i >= e)
            .map(|(_, c)| c)
            .collect::<String>()
            .trim()
            .to_owned();
    }
    let text = text
        .trim_matches(|c: char| c == ',' || c == '-' || c.is_whitespace())
        .to_owned();
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Friday 2026-10-30 14:00.
    fn now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 30)
            .and_then(|d| d.and_hms_opt(14, 0, 0))
            .expect("date")
    }

    fn at(d: u32, h: u32, m: u32) -> Option<NaiveDateTime> {
        let (month, day) = if d > 31 { (11, d - 31) } else { (10, d) };
        NaiveDate::from_ymd_opt(2026, month, day).and_then(|x| x.and_hms_opt(h, m, 0))
    }

    #[test]
    fn table() {
        let cases: &[(&str, &str, Option<NaiveDateTime>)] = &[
            // Relative
            ("uống nước sau 15 phút", "Uống nước", at(30, 14, 15)),
            ("gọi mẹ 15p nữa", "Gọi mẹ", at(30, 14, 15)),
            ("goi me 15p nua", "Goi me", at(30, 14, 15)),
            ("tắt bếp 1 tiếng nữa", "Tắt bếp", at(30, 15, 0)),
            ("nộp bài nửa tiếng nữa", "Nộp bài", at(30, 14, 30)),
            ("họp 2h30p nữa", "Họp", at(30, 16, 30)),
            ("check mail in 10 min", "Check mail", at(30, 14, 10)),
            ("sau 45 phút kiểm tra lò", "Kiểm tra lò", at(30, 14, 45)),
            // Clock time today/tomorrow
            ("nộp báo cáo 17h", "Nộp báo cáo", at(30, 17, 0)),
            ("nộp báo cáo lúc 17:30", "Nộp báo cáo", at(30, 17, 30)),
            ("họp 3h chiều", "Họp", at(30, 15, 0)),
            ("họp lúc 3 giờ chiều", "Họp", at(30, 15, 0)),
            ("gọi khách 4h", "Gọi khách", at(30, 16, 0)),
            ("đi ngủ 11h", "Đi ngủ", at(30, 23, 0)),
            ("đón con 9h sáng", "Đón con", at(31, 9, 0)),
            ("chạy bộ 6h sáng", "Chạy bộ", at(31, 6, 0)),
            ("ăn trưa 12h trưa", "Ăn trưa", at(31, 12, 0)),
            ("xem phim 8h tối", "Xem phim", at(30, 20, 0)),
            ("uống thuốc 8 giờ tối", "Uống thuốc", at(30, 20, 0)),
            ("họp 3h15", "Họp", at(30, 15, 15)),
            ("họp 3 giờ 15 phút chiều", "Họp", at(30, 15, 15)),
            ("họp 3h rưỡi chiều", "Họp", at(30, 15, 30)),
            ("call 5pm", "Call", at(30, 17, 0)),
            ("call at 10am", "Call", at(31, 10, 0)),
            ("ăn tối 19h", "Ăn tối", at(30, 19, 0)),
            ("dọn nhà 13h", "Dọn nhà", at(31, 13, 0)),
            // Dates
            ("họp team 3h chiều mai", "Họp team", at(31, 15, 0)),
            ("họp team mai 3h", "Họp team", at(31, 15, 0)),
            ("mai 9h đi khám", "Đi khám", at(31, 9, 0)),
            ("ngày mai nộp hồ sơ", "Nộp hồ sơ", at(31, 9, 0)),
            ("sáng mai đi chợ", "Đi chợ", at(31, 9, 0)),
            ("tối nay gọi bố", "Gọi bố", at(30, 20, 0)),
            ("tối nay 9h gọi bố", "Gọi bố", at(30, 21, 0)),
            ("chiều nay họp", "Họp", at(30, 15, 0)),
            ("trưa mai ăn với An", "Ăn với An", at(31, 12, 0)),
            ("hôm nay 16h nộp bài", "Nộp bài", at(30, 16, 0)),
            (
                "ngày kia trả sách",
                "Trả sách",
                Some(
                    NaiveDate::from_ymd_opt(2026, 11, 1)
                        .and_then(|d| d.and_hms_opt(9, 0, 0))
                        .expect("d"),
                ),
            ),
            (
                "mốt 10h đi phỏng vấn",
                "Đi phỏng vấn",
                Some(
                    NaiveDate::from_ymd_opt(2026, 11, 1)
                        .and_then(|d| d.and_hms_opt(10, 0, 0))
                        .expect("d"),
                ),
            ),
            (
                "thứ 2 họp giao ban 8h sáng",
                "Họp giao ban",
                Some(
                    NaiveDate::from_ymd_opt(2026, 11, 2)
                        .and_then(|d| d.and_hms_opt(8, 0, 0))
                        .expect("d"),
                ),
            ),
            (
                "thứ hai họp",
                "Họp",
                Some(
                    NaiveDate::from_ymd_opt(2026, 11, 2)
                        .and_then(|d| d.and_hms_opt(9, 0, 0))
                        .expect("d"),
                ),
            ),
            (
                "chủ nhật đi cafe 9h",
                "Đi cafe",
                Some(
                    NaiveDate::from_ymd_opt(2026, 11, 1)
                        .and_then(|d| d.and_hms_opt(9, 0, 0))
                        .expect("d"),
                ),
            ),
            (
                "thứ 6 tuần sau nộp",
                "Nộp",
                Some(
                    NaiveDate::from_ymd_opt(2026, 11, 6)
                        .and_then(|d| d.and_hms_opt(9, 0, 0))
                        .expect("d"),
                ),
            ),
            (
                "sinh nhật vợ 25/12",
                "Sinh nhật vợ",
                Some(
                    NaiveDate::from_ymd_opt(2026, 12, 25)
                        .and_then(|d| d.and_hms_opt(9, 0, 0))
                        .expect("d"),
                ),
            ),
            (
                "gia hạn hộ chiếu 15/3",
                "Gia hạn hộ chiếu",
                Some(
                    NaiveDate::from_ymd_opt(2027, 3, 15)
                        .and_then(|d| d.and_hms_opt(9, 0, 0))
                        .expect("d"),
                ),
            ),
            (
                "đóng tiền nhà ngày 5",
                "Đóng tiền nhà",
                Some(
                    NaiveDate::from_ymd_opt(2026, 11, 5)
                        .and_then(|d| d.and_hms_opt(9, 0, 0))
                        .expect("d"),
                ),
            ),
            ("tomorrow 9am standup", "Standup", at(31, 9, 0)),
            // No time
            ("mua sữa", "Mua sữa", None),
            ("nhắc tôi mua sữa", "Mua sữa", None),
        ];
        let mut failures = Vec::new();
        for (input, title, due) in cases {
            let p = parse(input, now());
            if p.title != *title || p.due != *due {
                failures.push(format!(
                    "{input:?} → title {:?} due {:?} (want {title:?} {due:?})",
                    p.title, p.due
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "{} failures:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn repeats() {
        let cases: &[(&str, Repeat, &str)] = &[
            (
                "uống nước mỗi 2 tiếng",
                Repeat::Every { minutes: 120 },
                "Uống nước",
            ),
            (
                "đứng dậy mỗi 45 phút",
                Repeat::Every { minutes: 45 },
                "Đứng dậy",
            ),
            ("uống thuốc 8h sáng mỗi ngày", Repeat::Daily, "Uống thuốc"),
            (
                "tập thể dục hằng ngày 6h sáng",
                Repeat::Daily,
                "Tập thể dục",
            ),
            (
                "chấm công 8h các ngày trong tuần",
                Repeat::Weekdays,
                "Chấm công",
            ),
            (
                "họp team thứ 2 hàng tuần 9h",
                Repeat::Weekly { days: vec![0] },
                "Họp team",
            ),
            (
                "đóng tiền điện ngày 10 hàng tháng",
                Repeat::Monthly { day: 10 },
                "Đóng tiền điện",
            ),
        ];
        for (input, repeat, title) in cases {
            let p = parse(input, now());
            assert_eq!(&p.repeat, repeat, "{input}");
            assert_eq!(&p.title, title, "{input}");
            assert!(p.due.is_some(), "{input} should have a due time");
        }
    }

    #[test]
    fn normalize_is_char_aligned() {
        let s = "Họp Đội lúc 3 giờ chiều";
        assert_eq!(normalize(s), "hop doi luc 3 gio chieu");
        assert_eq!(normalize(s).chars().count(), s.chars().count());
    }
}
