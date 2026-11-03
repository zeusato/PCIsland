//! Stage only the expected release asset. Portable updates replace the executable alone.
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const LIMIT: u64 = 128 * 1024 * 1024;

#[derive(Deserialize)]
pub struct Asset {
    pub name: String,
    pub size: u64,
    pub digest: Option<String>,
}

pub struct Package {
    dir: TempDir,
    target: PathBuf,
    file: PathBuf,
    digest: String,
}

impl Package {
    pub fn apply(self, restart: bool) -> Result<(), String> {
        crate::platform::update::handoff(&self.target, &self.file, &self.digest, restart)
            .map_err(|e| e.to_string())?;
        // The helper owns the staged file until Windows releases the running binary.
        let _ = self.dir.keep();
        Ok(())
    }
}

fn verify(bytes: &[u8], asset: &Asset) -> Result<String, String> {
    let digest = format!("{:x}", Sha256::digest(bytes));
    if bytes.len() as u64 != asset.size
        || asset
            .digest
            .as_deref()
            .and_then(|d| d.strip_prefix("sha256:"))
            != Some(&digest)
    {
        return Err("Release asset checksum or size mismatch".into());
    }
    Ok(digest)
}

fn portable_exe(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    archive
        .by_name("PCisland.exe")
        .map_err(|e| e.to_string())?
        .take(LIMIT + 1)
        .read_to_end(&mut result)
        .map_err(|e| e.to_string())?;
    if !result.starts_with(b"MZ") || result.len() as u64 > LIMIT {
        return Err("Invalid portable executable".into());
    }
    Ok(result)
}

pub async fn download(
    client: &reqwest::Client,
    version: &str,
    assets: &[Asset],
) -> Result<Package, String> {
    let target = std::env::current_exe().map_err(|e| e.to_string())?;
    let installed = crate::platform::update::installed(&target);
    let suffix = if installed {
        "setup.exe"
    } else {
        "portable.zip"
    };
    let name = format!("PCisland-{version}-{suffix}");
    let asset = assets
        .iter()
        .find(|a| a.name == name && a.size > 0 && a.size <= LIMIT)
        .ok_or("No matching release asset")?;
    let url = format!("https://github.com/zeusato/PCIsland/releases/download/v{version}/{name}");
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if (bytes.len() + chunk.len()) as u64 > asset.size {
            return Err("Asset too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let digest = verify(&bytes, asset)?;
    tauri::async_runtime::spawn_blocking(move || stage(target, installed, bytes, digest))
        .await
        .map_err(|e| e.to_string())?
}

fn stage(
    target: PathBuf,
    installed: bool,
    bytes: Vec<u8>,
    digest: String,
) -> Result<Package, String> {
    let (root, file_name, payload, digest) = if installed {
        (std::env::temp_dir(), "setup.exe", bytes, digest)
    } else {
        let exe = portable_exe(&bytes)?;
        let hash = format!("{:x}", Sha256::digest(&exe));
        let parent = target.parent().unwrap_or(Path::new(".")).to_owned();
        (parent, "portable.exe", exe, hash)
    };
    let dir = tempfile::Builder::new()
        .prefix("PCisland-update-")
        .tempdir_in(root)
        .map_err(|e| e.to_string())?;
    let file = dir.path().join(file_name);
    std::fs::write(&file, payload).map_err(|e| e.to_string())?;
    Ok(Package {
        dir,
        target,
        file,
        digest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn portable_package_stages_only_its_executable_and_preserves_data() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        std::fs::create_dir(&data).unwrap();
        std::fs::write(data.join("settings.json"), "original").unwrap();
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, content) in [
            ("PCisland.exe", "MZnew"),
            ("data/settings.json", "replacement"),
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        let bytes = zip.finish().unwrap().into_inner();
        let target = root.path().join("Original name.exe");
        std::fs::write(&target, b"MZold").unwrap();
        let package = stage(target.clone(), false, bytes, String::new()).unwrap();
        assert_eq!(std::fs::read(&package.file).unwrap(), b"MZnew");
        assert_eq!(std::fs::read(target).unwrap(), b"MZold");
        assert_eq!(
            std::fs::read(data.join("settings.json")).unwrap(),
            b"original"
        );
    }
    #[test]
    fn asset_digest_and_size_must_both_match() {
        let asset = Asset {
            name: "setup.exe".into(),
            size: 3,
            digest: Some(format!("sha256:{:x}", Sha256::digest(b"new"))),
        };
        assert!(verify(b"new", &asset).is_ok());
        assert!(verify(b"old", &asset).is_err());
        assert!(verify(b"", &asset).is_err());
        assert!(portable_exe(b"not a ZIP").is_err());
    }
}
