//! Optional, serial Groq jobs. Only marked clips leave the computer.
#![forbid(unsafe_code)]
use super::MeetingHandle;
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path, time::Duration};
/// Read the user's runtime credential without exposing it to the WebView.
pub(super) fn api_key() -> Option<String> {
    std::env::var("GROQ_API_KEY")
        .ok()
        .map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty())
}

pub fn start(handle: MeetingHandle) {
    tauri::async_runtime::spawn(async move {
        // A crash can happen after the provider accepted a request. Keep the
        // job visible and retryable, rather than claiming exactly-once delivery.
        for m in handle.snapshot().meetings {
            let _ = handle.update(&m.id, |m| {
                for c in &mut m.highlights {
                    if c.transcript_status == "running" {
                        c.transcript_status = "failed".into();
                        c.error = "Previous transcription interrupted; retry manually".into();
                    }
                }
            });
        }
        loop {
            let now = crate::ipc::now_ms();
            let mut wait = 3_600_000;
            let mut job = None;
            for m in handle.snapshot().meetings {
                for c in m.highlights {
                    if c.status == "ready" && c.transcript_status == "queued" {
                        if c.retry_at <= now && job.is_none() {
                            job = Some((m.id.clone(), c));
                        } else {
                            wait = wait.min(c.retry_at.saturating_sub(now).max(1000));
                        }
                    }
                }
            }
            let Some((id, clip)) = job else {
                tokio::select! {()=handle.wake.notified()=>{},()=tokio::time::sleep(Duration::from_millis(wait))=>{}};
                continue;
            };
            let Some(key) = api_key() else {
                let _ = handle.update(&id, |m| {
                    if let Some(c) = m.highlights.iter_mut().find(|c| c.id == clip.id) {
                        c.transcript_status = "failed".into();
                        c.error = "Set GROQ_API_KEY and restart PCisland to transcribe".into();
                    }
                });
                continue;
            };
            let _ = handle.update(&id, |m| {
                if let Some(c) = m.highlights.iter_mut().find(|c| c.id == clip.id) {
                    c.transcript_status = "running".into();
                    c.attempts += 1;
                }
            });
            let path = clip.file.clone();
            let db = handle.db.clone();
            let result = tauri::async_runtime::spawn_blocking(move || {
                transcribe(Path::new(&path), &db, &key)
            })
            .await;
            let result =
                result.unwrap_or_else(|_| Err(("Transcription worker stopped".into(), None)));
            let _ = handle.update(&id, |m| {
                if let Some(c) = m.highlights.iter_mut().find(|c| c.id == clip.id) {
                    match result {
                        Ok(text) => {
                            c.transcript = text;
                            c.transcript_status = "ready".into();
                            c.error.clear();
                        }
                        Err((message, retry)) => {
                            c.error = message;
                            if let Some(delay) = retry.filter(|_| c.attempts < 5) {
                                c.transcript_status = "queued".into();
                                c.retry_at = crate::ipc::now_ms() + delay;
                            } else {
                                c.transcript_status = "failed".into();
                            }
                        }
                    }
                }
            });
            // Shared account: keep bursts below the provider's request/minute limit.
            tokio::time::sleep(Duration::from_secs(4)).await;
        }
    });
}
fn transcribe(
    path: &Path,
    db: &crate::core::storage::Db,
    key: &str,
) -> Result<String, (String, Option<u64>)> {
    let fail = |e: std::io::Error| (e.to_string(), None);
    let file = std::fs::File::open(path).map_err(fail)?;
    if file.metadata().map_err(fail)?.len() > 24 * 1024 * 1024 {
        return Err((
            "Marked clip exceeds upload limit; audio is preserved locally".into(),
            None,
        ));
    }
    let mut hash = Sha256::new();
    let mut reader = std::io::BufReader::new(file);
    let mut buf = [0_u8; 65536];
    loop {
        let n = reader.read(&mut buf).map_err(fail)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    let cache = format!(
        "meeting.transcript.whisper-large-v3.vi.{:x}",
        hash.finalize()
    );
    if let Some(text) = db.kv_get(&cache) {
        return Ok(text);
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|_| ("Cannot create transcription client".into(), None))?;
    let opus = path.extension().is_some_and(|s| s == "opus");
    let part = reqwest::blocking::multipart::Part::file(path)
        .map_err(fail)?
        .file_name(if opus { "highlight.ogg" } else { "sample.wav" })
        .mime_str(if opus { "audio/ogg" } else { "audio/wav" })
        .map_err(|_| ("Invalid audio MIME".into(), None))?;
    let form = reqwest::blocking::multipart::Form::new()
        .text("model", "whisper-large-v3")
        .text("language", "vi")
        .text("temperature", "0")
        .text("response_format", "verbose_json")
        .part("file", part);
    let response = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .bearer_auth(key)
        .multipart(form)
        .send()
        .map_err(|_| ("Cannot reach Groq; clip remains local".into(), Some(60_000)))?;
    let status = response.status();
    if !status.is_success() {
        let retry = if status.as_u16() == 429 || status.is_server_error() {
            Some(
                response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(60)
                    .clamp(10, 3600)
                    * 1000,
            )
        } else {
            None
        };
        return Err((format!("Groq HTTP {}", status.as_u16()), retry));
    }
    let value: serde_json::Value = response
        .json()
        .map_err(|_| ("Invalid transcription response".into(), None))?;
    let text = value
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or(("Missing transcript".into(), None))?
        .to_owned();
    db.kv_set(&cache, &text)
        .map_err(|_| ("Cannot save transcript cache".into(), None))?;
    Ok(text)
}
#[cfg(test)]
mod tests {
    #[test]
    fn reads_key_from_the_running_process() {
        const CASE: &str = "PCISLAND_TEST_GROQ_CASE";
        if let Ok(case) = std::env::var(CASE) {
            assert_eq!(
                super::api_key().as_deref(),
                (case == "set").then_some("test-only-key")
            );
            return;
        }
        // Child processes isolate credentials from other tests and from the developer's machine.
        for (case, value) in [
            ("missing", None),
            ("blank", Some("   ")),
            ("set", Some(" test-only-key ")),
        ] {
            let mut child = std::process::Command::new(std::env::current_exe().unwrap());
            child
                .arg("reads_key_from_the_running_process")
                .env(CASE, case)
                .env_remove("GROQ_API_KEY");
            if let Some(value) = value {
                child.env("GROQ_API_KEY", value);
            }
            let output = child.output().unwrap();
            assert!(output.status.success());
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        }
    }
    #[test]
    #[ignore = "live Groq API, public FLEURS fixture only; never microphone audio"]
    fn live_public_vietnamese_fixture() {
        let response=reqwest::blocking::get("https://huggingface.co/datasets/FluidInference/fleurs-full/resolve/main/vi_vn/vi_vn_0000.wav").expect("public fixture download");
        assert!(response.status().is_success());
        let bytes = response.bytes().unwrap();
        assert!(bytes.len() < 1_000_000);
        let path =
            std::env::temp_dir().join(format!("pcisland-public-{}.wav", crate::core::id::new_id()));
        std::fs::write(&path, &bytes).unwrap();
        // Exercise the actual Ogg Opus upload format, not only WAV.
        let mut cursor = 12;
        let mut pcm = &bytes[0..0];
        while cursor + 8 <= bytes.len() {
            let size =
                u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
            if &bytes[cursor..cursor + 4] == b"data" {
                pcm = &bytes[cursor + 8..cursor + 8 + size];
                break;
            }
            cursor += 8 + size + (size % 2);
        }
        assert!(!pcm.is_empty());
        let samples: Vec<f32> = pcm
            .chunks_exact(2)
            .flat_map(|v| {
                let sample = i16::from_le_bytes([v[0], v[1]]) as f32 / 32768.0;
                [sample; 3]
            })
            .collect();
        let journal = path.with_extension("packets");
        let opus = path.with_extension("opus");
        let mut recorder = crate::modules::meeting::codec::Recorder::new(&journal).unwrap();
        for chunk in samples.chunks(960) {
            let mut frame = [0.0; 960];
            frame[..chunk.len()].copy_from_slice(chunk);
            recorder.push(&frame).unwrap();
        }
        recorder.push(&[0.0; 960]).unwrap();
        recorder.flush().unwrap();
        drop(recorder);
        crate::modules::meeting::codec::export(&journal, &opus, 0, samples.len() as u64 / 48)
            .unwrap();
        let db = crate::core::storage::Db::open_in_memory().unwrap();
        let start = std::time::Instant::now();
        let key = super::api_key().expect("set GROQ_API_KEY for the live API test");
        let result = super::transcribe(&path, &db, &key);
        let wav_elapsed = start.elapsed();
        let opus_start = std::time::Instant::now();
        let opus_result = super::transcribe(&opus, &db, &key);
        let _ = std::fs::remove_file(&opus);
        let _ = std::fs::remove_file(&journal);
        let _ = std::fs::remove_file(&path);
        let opus_text = opus_result.expect("Groq accepts marked Opus clips");
        assert!(opus_text.to_lowercase().contains("văn hóa"));
        println!(
            "Opus pipeline: {} ms; {}",
            opus_start.elapsed().as_millis(),
            opus_text
        );
        let text = result.expect("Groq should accept public Vietnamese sample");
        assert!(text.to_lowercase().contains("văn hóa"));
        println!(
            "Public Vietnamese fixture: {} ms; transcript: {}",
            wav_elapsed.as_millis(),
            text
        );
    }
}
