#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::{
  io::Read,
  path::{Path, PathBuf},
  str::FromStr,
};

use async_fs::{read_to_string, write, OpenOptions};
use futures_lite::io::AsyncReadExt;
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use simplelog::*;
use stracciatella::{
  file_formats::stci::{Stci, StciRgb888},
  fs::resolve_existing_components,
  mods::{Mod as StracciatellaMod, ModPath},
  unicode::Nfc,
  vfs::VfsLayer,
};

mod config;
mod state;

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
async fn get_editable_mods(
  state: tauri::State<'_, state::ToolsetState>,
) -> Result<Vec<EditableMod>, String> {
  match state.inner() {
    state::ToolsetState::Configured {
      config,
      mod_manager,
      ..
    } => {
      let mut editable_mods = vec![];

      for m in mod_manager.available_mods() {
        match m.path() {
          // We dont support Android, so ModPath::Path is the only variant
          ModPath::Path(p) => {
            if p.starts_with(&config.stracciatella_install_dir) {
              continue;
            }
            let toolset_file =
              resolve_existing_components(Path::new(ModSettings::filename()), Some(p), true);
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
    _ => Err("get_editable_mods mods not available in this editor state".to_owned()),
  }
}

#[tauri::command]
fn get_available_mods(state: tauri::State<'_, state::ToolsetState>) -> Result<Vec<Mod>, String> {
  match state.inner() {
    state::ToolsetState::Configured { mod_manager, .. } => {
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
    _ => Err("get_available_mods mods not available in this editor state".to_owned()),
  }
}

#[tauri::command]
fn set_selected_mod(
  state: tauri::State<'_, state::ToolsetState>,
  mod_id: String,
) -> Result<(), String> {
  match state.inner() {
    state::ToolsetState::Configured {
      config,
      mod_manager,
      opened_mod,
      ..
    } => {
      let mod_to_open = state::OpenedMod::new(config, mod_manager, &mod_id)?;
      let mut opened_mod = opened_mod
        .lock()
        .map_err(|e| format!("error locking VFS mutex: {}", e))?;
      *opened_mod = Some(mod_to_open);

      Ok(())
    }
    _ => Err("set_selected_mod mods not available in this editor state".to_owned()),
  }
}

fn get_selected_mod(state: &state::ToolsetState) -> Result<state::OpenedMod, String> {
  match state {
    state::ToolsetState::Configured { opened_mod, .. } => {
      let opened_mod = opened_mod
        .lock()
        .map_err(|e| format!("error locking VFS mutex: {}", e))?;

      if let Some(opened_mod) = opened_mod.as_ref() {
        Ok(opened_mod.clone())
      } else {
        Err("no mod selected".to_owned())
      }
    }
    _ => Err("get_selected_mod mods not available in this editor state".to_owned()),
  }
}

fn get_mod_data_path(m: &StracciatellaMod, file_path: &str) -> PathBuf {
  let file_path = PathBuf::from("data").join(file_path);
  match m.path() {
    ModPath::Path(p) => resolve_existing_components(Path::new(&file_path), Some(p), true),
  }
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonFileWithSchema {
  content: Value,
  schema: Value,
}

#[tauri::command]
async fn open_json_file_with_schema(
  state: tauri::State<'_, state::ToolsetState>,
  file: String,
) -> Result<JsonFileWithSchema, String> {
  match state.inner() {
    state::ToolsetState::Configured { schema_manager, .. } => {
      let schema = schema_manager
        .get(Path::new(&file))
        .ok_or_else(|| format!("schema for `{}` not found", file))?;
      let schema = Value::from_str(schema.as_str())
        .map_err(|e| format!("error decoding schema for `{}`: {}", file, e))?;
      let selected_mod = get_selected_mod(&state)?;

      if file.contains("..") {
        return Err("file path cannot contain `..`".to_owned());
      }

      let path = get_mod_data_path(&selected_mod.m, &file);
      let content: Value = if path.exists() {
        let json = read_to_string(&path)
          .await
          .map_err(|e| format!("error reading `{:?}`: {}", path, e))?;
        stracciatella::json::de::from_string(&json)
          .map_err(|e| format!("error decoding data for `{:?}`: {}", path, e))?
      } else {
        let mut f = selected_mod
          .vfs
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
    _ => Err("open_json_file_with_schema mods not available in this editor state".to_owned()),
  }
}

#[tauri::command]
async fn persist_text_file(
  state: tauri::State<'_, state::ToolsetState>,
  file_path: String,
  content: String,
) -> Result<(), String> {
  if file_path.contains("..") {
    return Err("file path cannot contain `..`".to_owned());
  }

  match state.inner() {
    state::ToolsetState::Configured { .. } => {
      let selected_mod = get_selected_mod(&state)?;
      let path = get_mod_data_path(&selected_mod.m, &file_path);
      write(&path, &content)
        .await
        .map_err(|e| format!("Error saving to file {:?}: {}", path, e))
    }
    _ => Err("persist_text_file mods not available in this editor state".to_owned()),
  }
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
async fn read_image_file(
  state: tauri::State<'_, state::ToolsetState>,
  file: String,
) -> Result<Base64Image, String> {
  if file.contains("..") {
    return Err("file path cannot contain `..`".to_owned());
  }

  match state.inner() {
    state::ToolsetState::Configured { .. } => {
      let selected_mod = get_selected_mod(&state)?;
      let path = get_mod_data_path(&selected_mod.m, &file);
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
        let mut f = selected_mod
          .vfs
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
              let index = (y * width) + x;
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
    _ => Err("persist_text_file mods not available in this editor state".to_owned()),
  }
}

fn main() {
  TermLogger::init(
    LevelFilter::Info,
    Config::default(),
    TerminalMode::Mixed,
    ColorChoice::Auto,
  )
  .expect("could not start logger");

  let toolset_config_path = config::ToolsetConfig::path().expect("could not find user directory");
  log::info!(
    "Toolset config path: `{}`",
    toolset_config_path.to_string_lossy()
  );
  let config = if toolset_config_path.exists() {
    let c = std::fs::read_to_string(&toolset_config_path).expect("reading toolset config failed");
    serde_json::from_str(&c).expect("parsing toolset config failed")
  } else {
    config::PartialToolsetConfig::guess()
  };
  let initial_state = if let Some(config) = config.to_config() {
    state::ToolsetState::configure(config)
  } else {
    state::ToolsetState::NotConfigured { config }
  };

  tauri::Builder::default()
    .manage(initial_state)
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
