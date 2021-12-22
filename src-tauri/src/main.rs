#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use serde::{Serialize, Deserialize};
use stracciatella::{schemas::SchemaManager, mods::ModManager, config::{EngineOptions, find_stracciatella_home}};

lazy_static::lazy_static! {
  static ref SCHEMA_MANAGER: SchemaManager = SchemaManager::new();
  static ref ENGINE_OPTIONS: Result<EngineOptions, String> = {
    let stracciatella_home = find_stracciatella_home()?;
    let args = vec!["stracciatella-toolset".to_owned()];

    EngineOptions::from_home_and_args(&stracciatella_home, &args)
  };
  static ref MOD_MANAGER: Result<ModManager, String> = {
    let engine_options = ENGINE_OPTIONS.as_ref()?;
    Ok(ModManager::new_unchecked(&engine_options, &engine_options.assets_dir))
  };
}

#[derive(Debug, Serialize, Deserialize)]
struct Mod {
  id: String,
  name: String,
  description: String,
  version: String,
}

#[tauri::command]
fn get_available_mods() -> Result<Vec<Mod>, String> {
  let mod_manager = MOD_MANAGER.as_ref()?;
  let mods = mod_manager.available_mods().iter().map(|m| Mod {
    id: m.id().to_owned(),
    name: m.name().to_owned(),
    description: m.description().to_owned(),
    version: m.version().to_owned(),
  }).collect();
  Ok(mods)
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![get_available_mods])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
