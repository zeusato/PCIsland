//! `manifest.json` of the sounds release (docs/modules/focus-sounds.md §4). Pure parsing and
//! validation: one bad entry rejects the whole manifest.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const REPO: &str = "zeusato/pcisland-sounds";
pub const MANIFEST_URL: &str =
    "https://github.com/zeusato/pcisland-sounds/releases/latest/download/manifest.json";
pub const MAX_MANIFEST: usize = 256 * 1024;
pub const MAX_FILE: u64 = 50 * 1024 * 1024;
const MAX_TOTAL: u64 = 1024 * 1024 * 1024;
const MAX_PACKS: usize = 32;
const MAX_TRACKS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum SoundRole {
    Focus,
    Break,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PackName {
    pub vi: String,
    pub en: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SoundTrack {
    pub file: String,
    pub title: String,
    #[ts(type = "number")]
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SoundPack {
    pub id: String,
    pub role: SoundRole,
    pub name: PackName,
    pub tracks: Vec<SoundTrack>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub release: String,
    #[serde(default)]
    pub license: String,
    pub packs: Vec<SoundPack>,
}

/// `[a-z0-9._-]`, not starting with a dot, 1–96 characters.
pub fn safe_name(value: &str) -> bool {
    (1..=96).contains(&value.len())
        && !value.starts_with('.')
        && value.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
}

fn hex64(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn parse(bytes: &[u8]) -> Result<Manifest, String> {
    if bytes.len() > MAX_MANIFEST {
        return Err("manifest too large".into());
    }
    let mut m: Manifest = serde_json::from_slice(bytes).map_err(|e| format!("manifest: {e}"))?;
    validate(&mut m)?;
    Ok(m)
}

fn validate(m: &mut Manifest) -> Result<(), String> {
    if m.schema != 1 {
        return Err(format!("unsupported manifest schema {}", m.schema));
    }
    if !safe_name(&m.release) {
        return Err("invalid release tag".into());
    }
    if m.packs.is_empty() || m.packs.len() > MAX_PACKS {
        return Err("invalid pack count".into());
    }
    m.license = m.license.chars().take(500).collect();
    let mut files = std::collections::HashSet::new();
    let mut ids = std::collections::HashSet::new();
    let mut total = 0u64;
    for pack in &mut m.packs {
        if !safe_name(&pack.id) || !ids.insert(pack.id.clone()) {
            return Err(format!("invalid pack id {:?}", pack.id));
        }
        if pack.tracks.is_empty() || pack.tracks.len() > MAX_TRACKS {
            return Err(format!("invalid track count in {}", pack.id));
        }
        pack.name.vi = pack.name.vi.chars().take(60).collect();
        pack.name.en = pack.name.en.chars().take(60).collect();
        for track in &mut pack.tracks {
            if !safe_name(&track.file)
                || !track.file.ends_with(".mp3")
                || !files.insert(track.file.clone())
            {
                return Err(format!("invalid file {:?}", track.file));
            }
            if !hex64(&track.sha256) || track.size == 0 || track.size > MAX_FILE {
                return Err(format!("invalid hash or size for {}", track.file));
            }
            track.sha256.make_ascii_lowercase();
            track.title = track.title.chars().take(120).collect();
            total += track.size;
        }
    }
    if total > MAX_TOTAL {
        return Err("release too large".into());
    }
    Ok(())
}

/// Every file URL is built here from the fixed repository, never taken from the manifest.
pub fn file_url(release: &str, file: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/{release}/{file}")
}

#[cfg(test)]
pub(crate) fn sample(release: &str, sha: &str) -> String {
    format!(
        r#"{{"schema":1,"release":"{release}","license":"test","packs":[
        {{"id":"lofi","role":"focus","name":{{"vi":"Lo-fi","en":"Lo-fi"}},"tracks":[{{"file":"lofi-01-a.mp3","title":"A","size":3,"sha256":"{sha}"}}]}},
        {{"id":"rest","role":"break","name":{{"vi":"Nghỉ","en":"Rest"}},"tracks":[{{"file":"rest-01-b.mp3","title":"B","size":3,"sha256":"{sha}"}}]}}]}}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    const SHA: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn accepts_a_well_formed_manifest() {
        let m = parse(sample("focus-sounds-v1", SHA).as_bytes()).unwrap();
        assert_eq!(m.packs.len(), 2);
        assert_eq!(m.packs[1].role, SoundRole::Break);
        assert_eq!(
            file_url(&m.release, "lofi-01-a.mp3"),
            "https://github.com/zeusato/pcisland-sounds/releases/download/focus-sounds-v1/lofi-01-a.mp3"
        );
    }

    #[test]
    fn rejects_paths_hosts_bad_hashes_and_duplicates() {
        for bad in [
            sample("../v1", SHA),
            sample("focus-sounds-v1", "abc"),
            sample("focus-sounds-v1", SHA).replace("lofi-01-a.mp3", "../evil.mp3"),
            sample("focus-sounds-v1", SHA).replace("lofi-01-a.mp3", "a.exe"),
            sample("focus-sounds-v1", SHA).replace("rest-01-b.mp3", "lofi-01-a.mp3"),
            sample("focus-sounds-v1", SHA).replace(r#""size":3"#, r#""size":999999999999"#),
            sample("focus-sounds-v1", SHA).replace(r#""role":"focus""#, r#""role":"party""#),
            sample("focus-sounds-v1", SHA).replace(r#""schema":1"#, r#""schema":2"#),
        ] {
            assert!(parse(bad.as_bytes()).is_err(), "{bad}");
        }
    }
}
