#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::{
  io::Read,
  path::{Path, PathBuf},
  str::FromStr,
  sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use stracciatella::{
  config::{find_stracciatella_home, EngineOptions},
  fs::resolve_existing_components,
  mods::{Mod as StracciatellaMod, ModManager, ModPath},
  schemas::SchemaManager,
  unicode::Nfc,
  vfs::{Vfs, VfsLayer},
};

lazy_static::lazy_static! {
  static ref SCHEMA_MANAGER: SchemaManager = SchemaManager::default();
  static ref ENGINE_OPTIONS: Result<EngineOptions, String> = {
    let stracciatella_home = find_stracciatella_home()?;
    let args = vec!["stracciatella-toolset".to_owned()];

    EngineOptions::from_home_and_args(&stracciatella_home, &args)
  };
  static ref MOD_MANAGER: Result<ModManager, String> = {
    let engine_options = ENGINE_OPTIONS.as_ref()?;
    Ok(ModManager::new_unchecked(&engine_options, &engine_options.assets_dir))
  };

  static ref SELECTED_MOD: Mutex<Option<StracciatellaMod>> = Mutex::new(None);
  static ref VFS: Arc<Mutex<Vfs>> = Arc::new(Mutex::new(Vfs::new()));
}

#[derive(Debug, Serialize, Deserialize)]
struct Mod {
  id: String,
  name: String,
  description: String,
  version: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ModSettings {
  required_mods: Vec<String>,
}

impl ModSettings {
  fn filename() -> &'static str {
    "toolset.json"
  }
}

#[derive(Debug, Serialize, Deserialize)]
struct EditableMod {
  #[serde(flatten)]
  m: Mod,
  path: String,
  settings: Option<ModSettings>,
}

#[tauri::command]
fn get_editable_mods() -> Result<Vec<EditableMod>, String> {
  let engine_options = ENGINE_OPTIONS.as_ref()?;
  let mod_manager = MOD_MANAGER.as_ref()?;
  let mods = mod_manager
    .available_mods()
    .iter()
    .flat_map(|m| match m.path() {
      // We dont support Android, so ModPath::Path is the only variant
      ModPath::Path(p) => {
        if p.starts_with(&engine_options.assets_dir) {
          None
        } else {
          let toolset_file =
            resolve_existing_components(&Path::new(ModSettings::filename()), Some(p), true);
          let settings = std::fs::read_to_string(&toolset_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());
          Some(EditableMod {
            m: Mod {
              id: m.id().to_owned(),
              name: m.name().to_owned(),
              description: m.description().to_owned(),
              version: m.version().to_owned(),
            },
            path: p.to_string_lossy().to_string(),
            settings,
          })
        }
      }
    })
    .collect();
  Ok(mods)
}

#[tauri::command]
fn get_available_mods() -> Result<Vec<Mod>, String> {
  let mod_manager = MOD_MANAGER.as_ref()?;
  let mods = mod_manager
    .available_mods()
    .iter()
    .map(|m| Mod {
      id: m.id().to_owned(),
      name: m.name().to_owned(),
      description: m.description().to_owned(),
      version: m.version().to_owned(),
    })
    .collect();
  Ok(mods)
}

#[tauri::command]
fn set_selected_mod(mod_id: String) -> Result<(), String> {
  let mut selected_mod = SELECTED_MOD
    .lock()
    .map_err(|e| format!("error locking SELECTED_MOD mutex: {}", e))?;
  let engine_options = ENGINE_OPTIONS.as_ref()?;
  let mod_manager = MOD_MANAGER.as_ref()?;
  let mut vfs = VFS
    .lock()
    .map_err(|e| format!("error locking VFS mutex: {}", e))?;
  vfs.init(&engine_options, &mod_manager).map_err(|e| format!("error initializing vfs: {}", e))?;
  let mod_manager = MOD_MANAGER.as_ref()?;
  let m = mod_manager
    .get_mod_by_id(&mod_id)
    .ok_or_else(|| format!("unknown mod: {}", mod_id))?;
  *selected_mod = Some(m.clone());
  Ok(())
}

fn get_mod_data_path(m: &StracciatellaMod, file_path: &str) -> PathBuf {
  let file_path = PathBuf::from("data").join(file_path);
  match m.path() {
    ModPath::Path(p) => resolve_existing_components(&Path::new(&file_path), Some(p), true),
  }
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonFileWithSchema {
  content: Value,
  schema: Value,
}

#[tauri::command]
fn open_json_file_with_schema(file: String) -> Result<JsonFileWithSchema, String> {
  let schema = SCHEMA_MANAGER
    .get(&Path::new(&file))
    .ok_or_else(|| format!("schema for `{}` not found", file))?;
  let schema = Value::from_str(schema.as_str())
    .map_err(|e| format!("error decoding schema for `{}`: {}", file, e))?;
  let guard = SELECTED_MOD
    .lock()
    .map_err(|e| format!("error locking SELECTED_MOD mutex: {}", e))?;
  let selected_mod = guard.as_ref().ok_or_else(|| "no mod selected".to_owned())?;

  if file.contains("..") {
    return Err("file path cannot contain `..`".to_owned());
  }

  let path = get_mod_data_path(&selected_mod, &file);
  let content: Value = if path.exists() {
    let json =
      std::fs::read_to_string(&path).map_err(|e| format!("error reading `{:?}`: {}", path, e))?;
    stracciatella::json::de::from_string(&json)
      .map_err(|e| format!("error decoding schema for `{:?}`: {}", path, e))?
  } else {
    let guard = VFS
      .lock()
      .map_err(|e| format!("error locking VFS mutex: {}", e))?;
    let mut f = guard
      .open(&Nfc::caseless(&file))
      .map_err(|e| format!("error opening `{:?}`: {}", file, e))?;
    let mut json = String::new();
    f.read_to_string(&mut json)
      .map_err(|e| format!("error reading `{:?}`: {}", path, e))?;
    stracciatella::json::de::from_string(&json)
      .map_err(|e| format!("error decoding schema for `{:?}`: {}", path, e))?
  };

  Ok(JsonFileWithSchema { content, schema })
}

#[tauri::command]
fn persist_text_file(file_path: String, content: String) -> Result<(), String> {
  let guard = SELECTED_MOD
    .lock()
    .map_err(|e| format!("error locking SELECTED_MOD mutex: {}", e))?;
  let selected_mod = guard.as_ref().ok_or_else(|| "no mod selected".to_owned())?;

  if file_path.contains("..") {
    return Err("file path cannot contain `..`".to_owned());
  }

  let path = get_mod_data_path(&selected_mod, &file_path);
  std::fs::write(&path, &content).map_err(|e| format!("Error saving to file {:?}: {}", path, e))
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      get_available_mods,
      get_editable_mods,
      set_selected_mod,
      open_json_file_with_schema,
      persist_text_file
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
