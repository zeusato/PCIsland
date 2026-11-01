//! Streaming exports. Originals are never modified; outputs belong to the user.
#![forbid(unsafe_code)]
use std::{
    collections::HashSet,
    fs::File,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{State, ipc::Channel};

use super::ShelfHandle;
use crate::ipc::{IpcError, IpcResult};

#[derive(Default)]
pub struct ExportState(pub Mutex<Option<Arc<AtomicBool>>>);

pub use crate::shared::archive::{ShelfProgress, zip_files};

fn error(e: impl std::fmt::Display) -> IpcError {
    IpcError::new("shelf.export", e.to_string())
}

fn selected(shelf: &ShelfHandle, ids: &[String]) -> IpcResult<Vec<PathBuf>> {
    let items = shelf.raw_items().map_err(error)?;
    let paths: Vec<_> = items
        .into_iter()
        .filter(|i| ids.contains(&i.id))
        .map(|i| PathBuf::from(i.path))
        .collect();
    if paths.is_empty() || paths.len() != ids.iter().collect::<HashSet<_>>().len() {
        return Err(error("Selection changed; select files again"));
    }
    for p in &paths {
        if !p.exists() {
            return Err(error(format!("Missing: {}", p.display())));
        }
    }
    Ok(paths)
}

#[tauri::command]
pub fn shelf_copy(
    window: tauri::WebviewWindow,
    shelf: State<'_, ShelfHandle>,
    ids: Vec<String>,
) -> IpcResult<()> {
    let owner = crate::platform::window::RawHwnd::from_hwnd(window.hwnd().map_err(error)?);
    crate::platform::clipboard::write_files(&selected(&shelf, &ids)?, owner).map_err(error)
}

#[tauri::command]
pub fn shelf_remove_many(shelf: State<'_, ShelfHandle>, ids: Vec<String>) -> IpcResult<()> {
    shelf
        .db
        .call(|c| {
            let tx = c.unchecked_transaction()?;
            for id in ids {
                tx.execute("DELETE FROM shelf_items WHERE id=?1", [id])?;
            }
            tx.commit()
        })
        .map_err(error)?;
    shelf.publish();
    Ok(())
}

#[tauri::command]
pub fn shelf_cancel(state: State<'_, ExportState>) {
    if let Some(cancel) = state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
    {
        cancel.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub async fn shelf_zip(
    shelf: State<'_, ShelfHandle>,
    state: State<'_, ExportState>,
    ids: Vec<String>,
    destination: String,
    progress: Channel<ShelfProgress>,
) -> IpcResult<String> {
    let paths = selected(&shelf, &ids)?;
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut active = state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if active.is_some() {
            return Err(error("Another export is running"));
        }
        *active = Some(cancel.clone());
    }
    let output = destination.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        zip_files(&paths, Path::new(&output), &cancel, |p| {
            let _ = progress.send(p);
        })
    })
    .await;
    *state
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    result.map_err(error)?.map_err(error)?;
    shelf
        .add(std::slice::from_ref(&destination))
        .map_err(error)?;
    Ok(destination)
}

#[tauri::command]
pub async fn shelf_convert(
    shelf: State<'_, ShelfHandle>,
    id: String,
    destination: String,
    format: String,
    max_edge: Option<u32>,
) -> IpcResult<String> {
    let paths = selected(&shelf, &[id])?;
    let output = destination.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let format = match format.as_str() {
            "png" => image::ImageFormat::Png,
            "jpeg" => image::ImageFormat::Jpeg,
            "webp" => image::ImageFormat::WebP,
            _ => return Err("Unsupported image format".into()),
        };
        let mut reader = image::ImageReader::open(&paths[0])
            .map_err(|e| e.to_string())?
            .with_guessed_format()
            .map_err(|e| e.to_string())?;
        // Animated sources are rejected instead of silently losing frames.
        if !matches!(
            reader.format(),
            Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
        ) {
            return Err("Only static PNG/JPEG inputs are supported".into());
        }
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        if reader.format() == Some(image::ImageFormat::Png) {
            let decoder = image::codecs::png::PngDecoder::new(std::io::BufReader::new(
                File::open(&paths[0]).map_err(|e| e.to_string())?,
            ))
            .map_err(|e| e.to_string())?;
            if decoder.is_apng().map_err(|e| e.to_string())? {
                return Err("Animated PNG is not converted; original kept".into());
            }
        }
        let mut image = reader.decode().map_err(|e| e.to_string())?;
        if format == image::ImageFormat::Jpeg && image.color().has_alpha() {
            return Err("Choose PNG/WebP to preserve transparency".into());
        }
        if let Some(edge) = max_edge {
            if !(64..=8192).contains(&edge) {
                return Err("Size must be 64–8192".into());
            }
            if image.width() > edge || image.height() > edge {
                image = image.thumbnail(edge, edge);
            }
        }
        let temp =
            Path::new(&output).with_extension(format!("{}.partial", crate::core::id::new_id()));
        let result = (|| {
            let mut file = File::options()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|e| e.to_string())?;
            image
                .write_to(&mut file, format)
                .map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            crate::platform::files::publish(&temp, Path::new(&output))
                .map_err(|e| e.to_string())?;
            Ok(())
        })();
        let _ = std::fs::remove_file(temp);
        result
    })
    .await
    .map_err(error)?
    .map_err(error)?;
    shelf
        .add(std::slice::from_ref(&destination))
        .map_err(error)?;
    Ok(destination)
}
