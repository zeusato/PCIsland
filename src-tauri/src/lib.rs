//! PCisland — Dynamic Island for Windows. Architecture: docs/02-system-design.md.
#![deny(unsafe_op_in_unsafe_fn)]

mod app;
mod core;
mod ipc;
mod modules;
mod platform;

use std::sync::Arc;

use tauri::Manager;

use core::artwork::ArtworkCache;
use modules::{island, media, web_player};

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let artwork = Arc::new(ArtworkCache::default());
    let artwork_for_protocol = artwork.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Err(err) = island::open_main(app, None) {
                tracing::warn!(error = %err, "failed to open main window from second instance");
            }
        }))
        .register_uri_scheme_protocol("artwork", move |_ctx, request| {
            artwork_for_protocol.respond(&request)
        })
        .setup(move |app| {
            let handle = app.handle();
            // P0 stand-in for ActivityHub: highest activity priority (core::priority).
            let (top_tx, top_rx) = tokio::sync::watch::channel(core::priority::IDLE);
            let media = media::setup(handle, artwork.clone(), top_tx);
            let web = web_player::setup(handle, media.clone());
            let island = island::setup(handle, top_rx)?;
            app.manage(media);
            app.manage(web);
            app.manage(island);
            app::tray::setup(handle)?;
            tracing::info!("PCisland started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            island::commands::island_set_hit_regions,
            island::commands::island_get_state,
            island::commands::island_open_main,
            island::commands::island_toggle_hidden,
            media::commands::media_get_state,
            media::commands::media_play_pause,
            media::commands::media_next,
            media::commands::media_previous,
            web_player::commands::web_player_get_state,
            web_player::commands::web_player_list_sites,
            web_player::commands::web_player_open,
            web_player::commands::web_player_show,
            web_player::commands::web_player_hide,
            web_player::commands::web_player_close,
            web_player::commands::web_player_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PCisland");
}
