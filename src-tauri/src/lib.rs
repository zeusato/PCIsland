//! PCisland — Dynamic Island for Windows. Architecture: docs/02-system-design.md.
#![deny(unsafe_op_in_unsafe_fn)]

mod app;
mod core;
mod ipc;
mod modules;
mod platform;
mod shared;

use std::sync::Arc;

use tauri::Manager;

use core::activity::ActivityHub;
use core::artwork::ArtworkCache;
use core::audio::AudioEngine;
use core::settings::{self as settings_mod, LoadOutcome, SettingsHandle};
use core::storage::Db;
use modules::{
    indicators, island, local_player, lyrics, media, notes, reminders, shelf, timer, web_player,
};

pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    // Native UI smoke tests use their own database and never change startup or
    // global shortcuts. This flag is deliberately unavailable in release builds.
    let qa = cfg!(debug_assertions) && args.iter().any(|a| a == "--qa");
    if qa && core::paths::arg_value(&args, "--data-dir").is_none() {
        panic!("--qa requires an isolated --data-dir");
    }
    let paths = core::paths::resolve(&args);
    if args.iter().any(|a| a == "--cleanup") {
        app::cleanup(&paths);
        return;
    }
    let debug = args.iter().any(|a| a == "--debug");
    let _log_guard = app::init_logging(&paths, debug);
    app::install_panic_hook(&paths.logs_dir());
    tracing::info!(data_dir = %paths.data_dir.display(), portable = paths.portable, "starting PCisland {}", env!("CARGO_PKG_VERSION"));

    let settings_path = paths.settings_file();
    let (mut initial_settings, outcome) = if args.iter().any(|a| a == "--reset-settings") {
        let _ = std::fs::rename(
            &settings_path,
            settings_path.with_extension(format!("reset-{}", ipc::now_ms())),
        );
        (
            settings_mod::Settings::default(),
            LoadOutcome::ResetToDefaults,
        )
    } else {
        settings_mod::load(&settings_path)
    };
    initial_settings.validate();
    if matches!(
        outcome,
        LoadOutcome::RecoveredFromBackup | LoadOutcome::ResetToDefaults
    ) {
        tracing::warn!(?outcome, "settings were not loaded normally");
    }

    let artwork = Arc::new(ArtworkCache::default());
    let artwork_for_protocol = artwork.clone();
    let video_proxy = Arc::new(modules::video::proxy::Proxy::default());
    let video_proxy_for_protocol = video_proxy.clone();
    let open_route = core::paths::arg_value(&args, "--open").map(str::to_owned);

    tauri::Builder::default()
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if argv.iter().any(|a| a == "--autostart") {
                return;
            }
            let route = core::paths::arg_value(&argv, "--open");
            if let Err(err) = island::open_main(app, route) {
                tracing::warn!(error = %err, "failed to open main window from second instance");
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("artwork", move |_ctx, request| {
            artwork_for_protocol.respond(&request)
        })
        .register_uri_scheme_protocol("meetingaudio", |ctx, request| {
            modules::meeting::playback::respond(
                &ctx.app_handle().state::<modules::meeting::MeetingHandle>(),
                &request,
            )
        })
        // Media requests of the video window (docs/modules/video-player.md §5.4).
        .register_asynchronous_uri_scheme_protocol("vstream", move |_ctx, request, responder| {
            let proxy = video_proxy_for_protocol.clone();
            tauri::async_runtime::spawn(async move {
                responder.respond(proxy.handle(request).await);
            });
        })
        .setup(move |app| {
            let handle = app.handle();
            let db = Db::open(&paths.db_file())?;
            let settings = SettingsHandle::start(handle, settings_path.clone(), initial_settings);
            let sup = core::supervisor::Supervisor::new(handle);
            let bus = core::events::EventBus::new();
            core::events::spawn_resume_detector(bus.clone());
            let hub = ActivityHub::start(handle, &sup);
            let music_events = modules::local_player::event_bridge();
            let audio = AudioEngine::start(music_events.sink());

            let media = media::setup(
                handle,
                artwork.clone(),
                hub.clone(),
                settings.subscribe(),
                bus.clone(),
                &sup,
            );
            let web = web_player::setup(
                handle,
                db.clone(),
                media.clone(),
                artwork.clone(),
                settings.subscribe(),
            );
            let timers = timer::setup(
                handle,
                db.clone(),
                hub.clone(),
                audio.clone(),
                settings.subscribe(),
                bus.clone(),
                &sup,
            );
            let reminders = reminders::setup(
                handle,
                db.clone(),
                hub.clone(),
                audio.clone(),
                settings.subscribe(),
                bus.clone(),
                &sup,
            );
            indicators::setup(hub.clone(), settings.subscribe(), bus.clone());
            let player = local_player::setup(
                handle,
                db.clone(),
                audio.clone(),
                &music_events,
                media.clone(),
                artwork.clone(),
                settings.subscribe(),
                bus.clone(),
                &sup,
            );
            let notes = notes::setup(
                handle,
                db.clone(),
                hub.clone(),
                settings.get().notes.trash_days,
                bus.clone(),
                audio.clone(),
                settings.subscribe(),
            );
            let shelf = shelf::setup(
                handle,
                db.clone(),
                hub.clone(),
                settings.subscribe(),
                artwork.clone(),
            );
            let lyrics = lyrics::setup(
                handle,
                db.clone(),
                bus.clone(),
                settings.subscribe(),
                player.clone(),
            );
            let visualizer = modules::visualizer::setup(bus.clone(), settings.subscribe());
            let video = modules::video::setup(
                handle,
                db.clone(),
                media.clone(),
                video_proxy.clone(),
                artwork.clone(),
                settings.subscribe(),
                paths.local_dir.clone(),
            );
            let island = island::setup(handle, hub.clone(), &settings, bus.clone())?;
            let meeting = modules::meeting::setup(
                handle,
                db.clone(),
                paths.data_dir.join("meetings"),
                hub.clone(),
            );

            media.attach_smtc(island.hwnd());
            app::update::spawn(db.clone(), hub.clone(), settings.subscribe());
            if !qa {
                if let Err(error) = app::sync_autostart(handle, settings.get().app.autostart) {
                    tracing::warn!(%error, "autostart registration failed");
                }
                app::shortcuts::register(handle, &settings.get().app.shortcuts);
                app::tray::setup(handle)?;
            }
            if paths.fell_back {
                tracing::warn!("portable data folder is read-only; using %APPDATA%");
            }
            if outcome == LoadOutcome::RecoveredFromBackup
                || outcome == LoadOutcome::ResetToDefaults && settings_path.exists()
            {
                hub.alert(
                    core::activity::Activity::new(
                        "settings",
                        "settings:recovered",
                        "settings.recovered",
                        core::priority::ALERT,
                        serde_json::Value::Null,
                    )
                    .transient(10_000),
                );
            }

            // Audio follows the default output device.
            let audio_for_devices = audio.clone();
            let mut device_events = bus.subscribe();
            tauri::async_runtime::spawn(async move {
                while let Ok(event) = device_events.recv().await {
                    if let core::events::AppEvent::AudioDeviceChanged = event {
                        audio_for_devices.reopen();
                    }
                }
            });
            app.manage(sup);
            app.manage(bus);
            app.manage(paths.clone());
            app.manage(app::maintenance::MaintenanceState::default());
            app.manage(db);
            app.manage(settings);
            app.manage(hub);
            app.manage(audio);
            app.manage(music_events);
            app.manage(media);
            app.manage(web);
            app.manage(timers);
            app.manage(reminders);
            app.manage(player);
            app.manage(notes);
            app.manage(shelf);
            app.manage(shelf::operations::ExportState::default());
            app.manage(lyrics);
            app.manage(visualizer);
            app.manage(video);
            app.manage(meeting);
            app.manage(island);
            if let Some(route) = open_route.as_deref() {
                let _ = island::open_main(app.handle(), Some(route));
            }
            tracing::info!("PCisland started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::maintenance::maintenance_resources,
            app::maintenance::maintenance_logs,
            app::maintenance::maintenance_clean,
            app::maintenance::maintenance_processes,
            app::maintenance::maintenance_reclaim,
            app::maintenance::maintenance_measure,
            app::maintenance::maintenance_open,
            app::commands::app_get_info,
            app::commands::app_quit,
            app::commands::app_show_toast,
            app::commands::app_open_data_folder,
            app::commands::app_play_sound,
            app::commands::app_pause_alerts,
            app::commands::app_check_update_now,
            app::commands::app_open_url,
            app::commands::app_shortcut_errors,
            app::commands::app_modules,
            app::commands::settings_get,
            app::commands::settings_update,
            app::commands::activity_get_state,
            app::commands::activity_invoke,
            app::commands::activity_dismiss,
            app::commands::activity_pin,
            island::commands::island_set_hit_regions,
            island::commands::island_get_state,
            island::commands::island_open_main,
            island::commands::island_toggle_hidden,
            island::commands::island_request_focus,
            island::commands::island_release_focus,
            island::commands::island_idle_seconds,
            media::commands::media_get_state,
            media::commands::media_play_pause,
            media::commands::media_next,
            media::commands::media_previous,
            media::commands::media_select_source,
            timer::commands::timer_get_state,
            timer::commands::timer_start,
            timer::commands::timer_control,
            timer::commands::stopwatch_control,
            timer::commands::pomodoro_control,
            timer::commands::pomodoro_stats,
            timer::commands::focus_set,
            local_player::commands::local_player_get_state,
            local_player::commands::local_player_play_tracks,
            local_player::commands::local_player_play_files,
            local_player::commands::local_player_enqueue,
            local_player::commands::local_player_control,
            local_player::commands::local_player_set_volume,
            local_player::commands::local_player_folders,
            local_player::commands::local_player_add_folder,
            local_player::commands::local_player_remove_folder,
            local_player::commands::local_player_rescan,
            local_player::commands::local_player_query,
            local_player::commands::local_player_playlists,
            local_player::commands::local_player_playlist_create,
            local_player::commands::local_player_playlist_delete,
            local_player::commands::local_player_playlist_add,
            local_player::commands::local_player_playlist_play,
            local_player::commands::local_player_stations,
            local_player::commands::local_player_station_add,
            local_player::commands::local_player_station_remove,
            local_player::commands::local_player_play_station,
            local_player::commands::local_player_playlist_import,
            local_player::commands::local_player_playlist_export,
            lyrics::lyrics_get_current,
            lyrics::lyrics_adjust_offset,
            modules::visualizer::visualizer_subscribe,
            notes::notes_list,
            notes::notes_export,
            notes::notes_save,
            notes::notes_delete,
            notes::notes_restore,
            notes::notes_pin,
            notes::notes_toggle_check,
            notes::notes_tasks,
            notes::notes_set_task_alarm,
            notes::desktop::notes_desktop_open,
            notes::desktop::notes_desktop_state,
            notes::desktop::notes_desktop_options,
            shelf::shelf_list,
            shelf::shelf_add,
            shelf::shelf_remove,
            shelf::shelf_clear,
            shelf::shelf_open,
            shelf::shelf_drag_icon,
            shelf::operations::shelf_copy,
            shelf::operations::shelf_remove_many,
            shelf::operations::shelf_zip,
            shelf::operations::shelf_convert,
            shelf::operations::shelf_cancel,
            reminders::commands::reminders_list,
            reminders::commands::reminders_create,
            reminders::commands::reminders_update,
            reminders::commands::reminders_delete,
            reminders::commands::reminders_complete,
            reminders::commands::reminders_snooze,
            reminders::commands::reminders_parse,
            web_player::commands::web_player_get_state,
            web_player::commands::web_player_list_sites,
            web_player::commands::web_player_open,
            web_player::commands::web_player_resume,
            web_player::commands::web_player_select_queue,
            web_player::commands::web_player_request_queue,
            web_player::commands::web_player_set_repeat,
            web_player::commands::web_player_search,
            web_player::commands::web_player_search_report,
            web_player::commands::web_player_play_result,
            web_player::commands::web_player_show,
            web_player::commands::web_player_hide,
            web_player::commands::web_player_close,
            web_player::commands::web_player_report,
            web_player::commands::web_player_navigate,
            web_player::commands::web_player_clear_data,
            modules::video::commands::video_classify,
            modules::video::commands::video_clipboard,
            modules::video::commands::video_open,
            modules::video::commands::video_open_stream,
            modules::video::commands::video_get_state,
            modules::video::commands::video_tools_status,
            modules::video::commands::video_tools_install,
            modules::video::commands::video_control,
            modules::video::commands::video_window,
            modules::video::commands::video_open_in_browser,
            modules::video::commands::video_current,
            modules::video::commands::video_report,
            modules::video::commands::video_refresh,
            modules::video::commands::video_set_quality,
            modules::video::commands::video_sniff_report,
            modules::meeting::meeting_state,
            modules::meeting::meeting_devices,
            modules::meeting::meeting_start,
            modules::meeting::meeting_stop,
            modules::meeting::meeting_mark,
            modules::meeting::meeting_notes,
            modules::meeting::meeting_retry,
            modules::meeting::meeting_transcript_edit,
            modules::meeting::meeting_export,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PCisland");
}
