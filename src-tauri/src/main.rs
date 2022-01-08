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

use async_fs::{read_to_string, write, OpenOptions};
use futures_lite::io::AsyncReadExt;
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use stracciatella::{
  config::{find_stracciatella_home, EngineOptions},
  file_formats::stci::{Stci, StciRgb888},
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

  static ref SELECTED_MOD: Arc<Mutex<Option<StracciatellaMod>>> = Arc::new(Mutex::new(None));
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
async fn get_editable_mods() -> Result<Vec<EditableMod>, String> {
  let engine_options = ENGINE_OPTIONS.as_ref()?;
  let mod_manager = MOD_MANAGER.as_ref()?;
  let mods = mod_manager.available_mods();
  let mut editable_mods = vec![];

  for m in mods {
    match m.path() {
      // We dont support Android, so ModPath::Path is the only variant
      ModPath::Path(p) => {
        if p.starts_with(&engine_options.assets_dir) {
          continue;
        }
        let toolset_file =
          resolve_existing_components(&Path::new(ModSettings::filename()), Some(p), true);
        let settings = read_to_string(&toolset_file)
          .await
          .ok()
          .and_then(|s| serde_json::from_str(&s).ok());
        editable_mods.push(EditableMod {
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
  }

  Ok(editable_mods)
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
  vfs
    .init(&engine_options, &mod_manager)
    .map_err(|e| format!("error initializing vfs: {}", e))?;
  let mod_manager = MOD_MANAGER.as_ref()?;
  let m = mod_manager
    .get_mod_by_id(&mod_id)
    .ok_or_else(|| format!("unknown mod: {}", mod_id))?;
  *selected_mod = Some(m.clone());
  Ok(())
}

fn get_selected_mod() -> Result<StracciatellaMod, String> {
  let guard = SELECTED_MOD
  .lock()
  .map_err(|e| format!("error locking SELECTED_MOD mutex: {}", e))?;
  Ok(guard
    .as_ref()
    .ok_or_else(|| "no mod selected".to_owned())?
    .clone())
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
async fn open_json_file_with_schema(file: String) -> Result<JsonFileWithSchema, String> {
  let schema = SCHEMA_MANAGER
    .get(&Path::new(&file))
    .ok_or_else(|| format!("schema for `{}` not found", file))?;
  let schema = Value::from_str(schema.as_str())
    .map_err(|e| format!("error decoding schema for `{}`: {}", file, e))?;
  let selected_mod = get_selected_mod()?;

  if file.contains("..") {
    return Err("file path cannot contain `..`".to_owned());
  }

  let path = get_mod_data_path(&selected_mod, &file);
  let content: Value = if path.exists() {
    let json = read_to_string(&path)
      .await
      .map_err(|e| format!("error reading `{:?}`: {}", path, e))?;
    stracciatella::json::de::from_string(&json)
      .map_err(|e| format!("error decoding data for `{:?}`: {}", path, e))?
  } else {
    let guard = VFS
      .lock()
      .map_err(|e| format!("error locking VFS mutex: {}", e))?;
    let mut f = guard
      .open(&Nfc::caseless(&file))
      .map_err(|e| format!("error opening `{:?}`: {}", file, e))?;
    let mut json = String::new();
    f.read_to_string(&mut json)
      .map_err(|e| format!("error reading `{:?}`: {}", file, e))?;
    stracciatella::json::de::from_string(&json)
      .map_err(|e| format!("error decoding data for `{:?}`: {}", file, e))?
  };

  Ok(JsonFileWithSchema { content, schema })
}

#[tauri::command]
async fn persist_text_file(file_path: String, content: String) -> Result<(), String> {
  let selected_mod = get_selected_mod()?;

  if file_path.contains("..") {
    return Err("file path cannot contain `..`".to_owned());
  }

  let path = get_mod_data_path(&selected_mod, &file_path);
  write(&path, &content)
    .await
    .map_err(|e| format!("Error saving to file {:?}: {}", path, e))
}

#[derive(Debug)]
struct Base64Image {
  image: image::RgbaImage,
}

impl Base64Image {
  pub fn new(image: image::RgbaImage) -> Self {
    Self { image }
  }

  pub fn to_png_data(&self) -> Result<Vec<u8>, image::ImageError> {
    let mut png_data = vec![];
    let png_encoder = image::png::PngEncoder::new(&mut png_data);
    png_encoder.encode(
      self.image.as_raw(),
      self.image.width(),
      self.image.height(),
      image::ColorType::Rgba8,
    )?;
    Ok(png_data)
  }
}

impl Serialize for Base64Image {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let data = self.to_png_data().map_err(serde::ser::Error::custom)?;
    let base64_data = base64::encode(&data);
    let mime_type = "image/png";
    serializer.serialize_str(&format!("data:{};base64,{}", mime_type, base64_data))
  }
}

#[tauri::command]
async fn read_image_file(file: String) -> Result<Base64Image, String> {
  let selected_mod = {
    let guard = SELECTED_MOD
      .lock()
      .map_err(|e| format!("error locking SELECTED_MOD mutex: {}", e))?;
    guard
      .as_ref()
      .ok_or_else(|| "no mod selected".to_owned())?
      .clone()
  };

  if file.contains("..") {
    return Err("file path cannot contain `..`".to_owned());
  }

  let path = get_mod_data_path(&selected_mod, &file);
  let content: Vec<u8> = if path.exists() {
    let mut f = OpenOptions::new()
      .open(&path)
      .await
      .map_err(|e| format!("error reading `{:?}`: {}", path, e))?;
    let mut result = vec![];

    f.read_to_end(&mut result)
      .await
      .map_err(|e| format!("error reading `{:?}`: {}", file, e))?;

    result
  } else {
    let guard = VFS
      .lock()
      .map_err(|e| format!("error locking VFS mutex: {}", e))?;
    let mut f = guard
      .open(&Nfc::caseless(&file))
      .map_err(|e| format!("error opening `{:?}`: {}", file, e))?;
    let mut result = vec![];

    f.read_to_end(&mut result)
      .map_err(|e| format!("error reading `{:?}`: {}", file, e))?;

    result
  };
  let stci = Stci::from_input(&mut content.as_slice())
    .map_err(|e| format!("error decoding `{:?}`: {}", file, e))?;
  let size = match &stci {
    Stci::Indexed { sub_images, .. } => sub_images
      .get(0)
      .ok_or_else(|| "indexed stci does not have at least one subimage".to_owned())
      .map(|s| (u32::from(s.dimensions.0), u32::from(s.dimensions.1))),
    Stci::Rgb { width, height, .. } => Ok((u32::from(*width), u32::from(*height))),
  }?;
  let mut image = RgbaImage::new(size.0, size.1);

  match &stci {
    Stci::Indexed {
      sub_images,
      palette,
    } => {
      let sub_image = &sub_images[0];
      let width = usize::from(sub_image.dimensions.0);
      let height = usize::from(sub_image.dimensions.1);
      for y in 0..height {
        for x in 0..width {
          let index = usize::from((y * width) + x);
          let stci_pixel = palette.colors[usize::from(sub_image.data[index])];
          let pixel = image.get_pixel_mut(x as u32, y as u32);

          if sub_image.data[index] == 0 {
            pixel[3] = 0;
          } else {
            pixel[0] = stci_pixel.0;
            pixel[1] = stci_pixel.1;
            pixel[2] = stci_pixel.2;
            pixel[3] = 255;
          }
        }
      }
    }
    Stci::Rgb {
      width,
      height,
      data,
    } => {
      let width = usize::from(*width);
      let height = usize::from(*height);
      for y in 0..height {
        for x in 0..width {
          let index = (y * width) + x;
          let stci_pixel = StciRgb888::from(data[index]);
          let pixel = image.get_pixel_mut(x as u32, y as u32);

          pixel[0] = stci_pixel.0;
          pixel[1] = stci_pixel.1;
          pixel[2] = stci_pixel.2;
          pixel[3] = 255;
        }
      }
    }
  }

  Ok(Base64Image::new(image))
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      get_available_mods,
      get_editable_mods,
      set_selected_mod,
      open_json_file_with_schema,
      persist_text_file,
      read_image_file
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
