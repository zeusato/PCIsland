//! Runs a helper executable without a console window, with a timeout and an output cap
//! (yt-dlp for the video module, docs/modules/video-player.md §5.3).

use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// `CREATE_NO_WINDOW`: no console flashes up when the app starts the helper.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug)]
pub struct Output {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

/// Blocking: call from `spawn_blocking`. Kills the child on timeout or when stdout
/// exceeds `max_stdout`.
pub fn run(
    exe: &Path,
    args: &[&str],
    timeout: Duration,
    max_stdout: usize,
) -> Result<Output, String> {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("cannot start {}: {e}", exe.display()))?;
    let mut stdout = child.stdout.take().ok_or("no stdout")?;
    let mut stderr = child.stderr.take().ok_or("no stderr")?;
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 64 * 1024];
        loop {
            match stdout.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if buf.len() > max_stdout {
                        return Err(buf.len());
                    }
                }
            }
        }
        Ok(buf)
    });
    let err_reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.by_ref().take(64 * 1024).read_to_string(&mut s);
        s
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        // A reader that gave up on oversized output drops the pipe; the child then exits.
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("timed out".into());
        }
        std::thread::sleep(Duration::from_millis(30));
    };
    let stdout = match reader.join() {
        Ok(Ok(buf)) => buf,
        Ok(Err(_)) => return Err("output too large".into()),
        Err(_) => return Err("output reader failed".into()),
    };
    Ok(Output {
        success: status.success(),
        stdout,
        stderr: err_reader.join().unwrap_or_default(),
    })
}
