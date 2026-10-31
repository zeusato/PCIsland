//! LRC lyrics parser: `[mm:ss.xx] text`, several timestamps per line, `[offset:±ms]`.

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    #[ts(type = "number")]
    pub time_ms: u64,
    pub text: String,
}

pub fn parse(lrc: &str) -> Vec<LyricLine> {
    let mut offset: i64 = 0;
    let mut lines = Vec::new();
    for raw in lrc.lines() {
        let mut rest = raw.trim();
        let mut stamps = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(end) = stripped.find(']') else { break };
            let tag = &stripped[..end];
            rest = stripped[end + 1..].trim_start();
            if let Some(v) = tag.strip_prefix("offset:") {
                offset = v.trim().parse().unwrap_or(0);
            } else if let Some(ms) = timestamp(tag) {
                stamps.push(ms);
            }
        }
        for ms in stamps {
            lines.push(LyricLine {
                time_ms: ms,
                text: rest.to_owned(),
            });
        }
    }
    // Positive offset means lyrics appear sooner.
    for l in &mut lines {
        l.time_ms = (l.time_ms as i64 - offset).max(0) as u64;
    }
    lines.sort_by_key(|l| l.time_ms);
    lines
}

fn timestamp(tag: &str) -> Option<u64> {
    let (m, s) = tag.split_once(':')?;
    let minutes: u64 = m.trim().parse().ok()?;
    let seconds: f64 = s.trim().parse().ok()?;
    Some(minutes * 60_000 + (seconds * 1000.0).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multi_stamps_offset_and_meta() {
        let lrc = "[ar:Someone]\n[offset:500]\n[00:12.50][01:02.00]Chorus line\n[00:05.00]First\n[00:30.00]";
        let lines = parse(lrc);
        assert_eq!(
            lines.iter().map(|l| l.time_ms).collect::<Vec<_>>(),
            vec![4_500, 12_000, 29_500, 61_500]
        );
        assert_eq!(lines[0].text, "First");
        assert_eq!(lines[1].text, "Chorus line");
        assert_eq!(lines[2].text, "");
    }
}
