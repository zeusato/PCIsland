//! Windows installation detection and post-exit update handoff.
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

/// A standalone executable must use the portable package, even outside a data folder.
pub fn installed(exe: &Path) -> bool {
    let location = windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\PCisland")
        .and_then(|key| key.get_string("InstallLocation"));
    location.is_ok_and(|dir| {
        let target = Path::new(dir.trim_matches('"')).join("pcisland.exe");
        target
            .to_string_lossy()
            .eq_ignore_ascii_case(&exe.to_string_lossy())
    })
}

pub fn handoff(exe: &Path, asset: &Path, digest: &str, restart: bool) -> std::io::Result<()> {
    helper(exe, asset, digest, restart, std::process::id())?.spawn()?;
    Ok(())
}

fn helper(
    exe: &Path,
    asset: &Path,
    digest: &str,
    restart: bool,
    parent: u32,
) -> std::io::Result<Command> {
    let system = std::env::var_os("SystemRoot").ok_or(std::io::ErrorKind::NotFound)?;
    let shell = Path::new(&system).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let args: Vec<_> = std::env::args().collect();
    let mut cmd = Command::new(shell);
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        include_str!("apply-update.ps1"),
    ])
    .env("PCI_PARENT", parent.to_string())
    .env("PCI_TARGET", exe)
    .env("PCI_ASSET", asset)
    .env("PCI_DIGEST", digest)
    .env("PCI_RESTART", if restart { "1" } else { "0" })
    .env(
        "PCI_DATA_DIR",
        crate::core::paths::arg_value(&args, "--data-dir").unwrap_or(""),
    )
    .creation_flags(0x0800_0000);
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn portable_handoff_verifies_again_and_only_replaces_the_executable() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("PC island's app.exe");
        let data = root.path().join("data");
        std::fs::create_dir(&data).unwrap();
        std::fs::write(data.join("settings.json"), b"user settings").unwrap();
        std::fs::write(&target, b"old").unwrap();
        for valid in [false, true] {
            let stage = tempfile::tempdir_in(root.path()).unwrap();
            let asset = stage.path().join("portable.exe");
            std::fs::write(&asset, b"new").unwrap();
            let digest = format!("{:x}", Sha256::digest(if valid { b"new" } else { b"bad" }));
            let result = helper(&target, &asset, &digest, false, i32::MAX as u32)
                .unwrap()
                .status()
                .unwrap();
            assert_eq!(
                result.success(),
                valid,
                "{:?}",
                std::fs::read_to_string(stage.path().join("update-error.txt"))
            );
            assert_eq!(
                std::fs::read(&target).unwrap(),
                if valid { b"new" } else { b"old" }
            );
            assert_eq!(
                std::fs::read(data.join("settings.json")).unwrap(),
                b"user settings"
            );
        }
    }
}
