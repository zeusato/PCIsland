// Every app command is listed here so that each window must be granted it explicitly
// through a capability file. This is what keeps remote web pages limited to
// `web_player_report` (docs/02-system-design.md §12).
const COMMANDS: &[&str] = &[
    "island_set_hit_regions",
    "island_get_state",
    "island_open_main",
    "island_toggle_hidden",
    "media_get_state",
    "media_play_pause",
    "media_next",
    "media_previous",
    "web_player_get_state",
    "web_player_list_sites",
    "web_player_open",
    "web_player_show",
    "web_player_hide",
    "web_player_close",
    "web_player_report",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
