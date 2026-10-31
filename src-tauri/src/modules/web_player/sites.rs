//! Site profiles tested at Gate G0 (docs/06-decision-log.md ADR-008).
//! Keep in sync with `capabilities/web-player-remote.json`.

#[derive(Debug, Clone, Copy)]
pub struct SiteProfile {
    pub id: &'static str,
    pub name: &'static str,
    pub home_url: &'static str,
    /// Exact hosts or `*.domain` suffixes.
    pub domains: &'static [&'static str],
}

pub const SITES: &[SiteProfile] = &[
    SiteProfile {
        id: "zing_mp3",
        name: "Zing MP3",
        home_url: "https://zingmp3.vn/",
        domains: &["zingmp3.vn", "*.zingmp3.vn"],
    },
    SiteProfile {
        id: "soundcloud",
        name: "SoundCloud",
        home_url: "https://soundcloud.com/",
        domains: &["soundcloud.com", "*.soundcloud.com"],
    },
    SiteProfile {
        id: "youtube",
        name: "YouTube",
        home_url: "https://www.youtube.com/",
        domains: &["www.youtube.com"],
    },
    SiteProfile {
        id: "youtube_music",
        name: "YouTube Music",
        home_url: "https://music.youtube.com/",
        domains: &["music.youtube.com"],
    },
    SiteProfile {
        id: "spotify",
        name: "Spotify",
        home_url: "https://open.spotify.com/",
        domains: &["open.spotify.com"],
    },
    SiteProfile {
        id: "nhaccuatui",
        name: "NhacCuaTui",
        home_url: "https://www.nhaccuatui.com/",
        domains: &["www.nhaccuatui.com", "nhaccuatui.com"],
    },
];

pub fn find(id: &str) -> Option<&'static SiteProfile> {
    SITES.iter().find(|s| s.id == id)
}

pub fn for_host(host: &str) -> Option<&'static SiteProfile> {
    SITES
        .iter()
        .find(|s| s.domains.iter().any(|d| domain_matches(d, host)))
}

pub fn host_allowed(host: &str) -> bool {
    for_host(host).is_some()
}

fn domain_matches(pattern: &str, host: &str) -> bool {
    match pattern.strip_prefix("*.") {
        Some(suffix) => {
            host.len() > suffix.len()
                && host.ends_with(suffix)
                && host[..host.len() - suffix.len()].ends_with('.')
        }
        None => pattern.eq_ignore_ascii_case(host),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_matches_subdomains_only() {
        assert!(domain_matches("*.zingmp3.vn", "mp3.zingmp3.vn"));
        assert!(!domain_matches("*.zingmp3.vn", "zingmp3.vn"));
        assert!(!domain_matches("*.zingmp3.vn", "evilzingmp3.vn"));
    }

    #[test]
    fn hosts_map_to_sites() {
        assert_eq!(for_host("zingmp3.vn").map(|s| s.id), Some("zing_mp3"));
        assert_eq!(
            for_host("music.youtube.com").map(|s| s.id),
            Some("youtube_music")
        );
        assert!(!host_allowed("example.com"));
    }
}
