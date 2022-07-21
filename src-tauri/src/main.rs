#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use config::ToolsetConfig;
use simplelog::*;

mod config;
mod error;
mod invokables;
mod state;

use crate::error::{Error, Result};

fn main() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("failed to initialize logger");

    if let Ok(toolset_config_path) = config::ToolsetConfig::path() {
        log::info!(
            "Toolset config path: `{}`",
            toolset_config_path.to_string_lossy()
        );
    }
    let initial_state = match ToolsetConfig::read() {
        Ok(config) => state::ToolsetState::configured(config),
        Err((e, partial_config)) => {
            log::error!("failed to read toolset config: {}", e);
            state::ToolsetState::not_configured(partial_config)
        }
    };
    let initial_state = state::TauriState::new(initial_state);

    tauri::Builder::default()
        .manage(initial_state)
        .invoke_handler(tauri::generate_handler![
            invokables::toolset_config::get_toolset_config,
            invokables::toolset_config::set_toolset_config,
            invokables::mods::get_available_mods,
            invokables::mods::get_editable_mods,
            invokables::mods::set_selected_mod,
            invokables::json::open_json_file_with_schema,
            invokables::json::persist_json_file,
            invokables::images::read_image_file,
            invokables::resources::list_resources,
            invokables::sounds::read_sound,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
