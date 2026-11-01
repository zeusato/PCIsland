//! Local-only range responses for files belonging to saved meetings.
use super::MeetingHandle;
use std::io::{Read, Seek, SeekFrom};
use tauri::http::{Request, Response};
pub fn respond(handle: &MeetingHandle, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let empty = |status| {
        Response::builder()
            .status(status)
            .body(Vec::new())
            .expect("response")
    };
    let parts: Vec<_> = request.uri().path().trim_matches('/').split('/').collect();
    if parts.len() != 2 {
        return empty(404);
    }
    let Ok(m) = handle.get(parts[0]) else {
        return empty(404);
    };
    let path = if parts[1] == "full" {
        m.full_file
    } else {
        m.highlights
            .into_iter()
            .find(|c| c.id == parts[1] && c.status == "ready")
            .map(|c| c.file)
            .unwrap_or_default()
    };
    let Ok(mut file) = std::fs::File::open(path) else {
        return empty(404);
    };
    let Ok(meta) = file.metadata() else {
        return empty(500);
    };
    let total = meta.len();
    if total == 0 {
        return empty(404);
    }
    let range = request
        .headers()
        .get("range")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("bytes="))
        .and_then(|v| v.split_once('-'));
    let start = range.and_then(|(s, _)| s.parse::<u64>().ok()).unwrap_or(0);
    if start >= total {
        return empty(416);
    }
    let end = range
        .and_then(|(_, s)| s.parse::<u64>().ok())
        .unwrap_or(total - 1)
        .min(total - 1)
        .min(start + 1024 * 1024 - 1);
    if end < start {
        return empty(416);
    }
    let mut data = vec![0; (end - start + 1) as usize];
    if file
        .seek(SeekFrom::Start(start))
        .and_then(|_| file.read_exact(&mut data))
        .is_err()
    {
        return empty(500);
    }
    Response::builder()
        .status(206)
        .header("Content-Type", "audio/ogg")
        .header("Accept-Ranges", "bytes")
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Range", format!("bytes {start}-{end}/{total}"))
        .header("Content-Length", data.len())
        .body(data)
        .expect("response")
}
