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
use config::ToolsetConfig;
use futures_lite::io::AsyncReadExt;
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use simplelog::*;
use stracciatella::{
    file_formats::stci::{Stci, StciRgb888},
    fs::resolve_existing_components,
    mods::{Mod as StracciatellaMod, ModManager, ModPath},
    unicode::Nfc,
    vfs::{Vfs, VfsLayer},
};

mod config;
mod error;
mod state;

use crate::error::{Error, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Mod {
    id: String,
    name: String,
    description: String,
    version: String,
    path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct ModSettings {
    required_mods: Vec<String>,
}

impl ModSettings {
    fn filename() -> &'static str {
        "toolset.json"
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableToolsetConfig {
    partial: bool,
    config: config::PartialToolsetConfig,
}

#[tauri::command]
async fn get_toolset_config(
    state: tauri::State<'_, state::TauriState>,
) -> Result<SerializableToolsetConfig> {
    let state = state.read().await;
    match *state {
        state::ToolsetState::Configured { ref config, .. } => Ok(SerializableToolsetConfig {
            partial: false,
            config: config.clone().into(),
        }),
        state::ToolsetState::NotConfigured { ref config } => Ok(SerializableToolsetConfig {
            partial: true,
            config: config.clone(),
        }),
    }
}

#[tauri::command]
async fn set_toolset_config(
    state: tauri::State<'_, state::TauriState>,
    config: SerializableToolsetConfig,
) -> Result<SerializableToolsetConfig> {
    {
        let mut state = state.write().await;

        if config.partial {
            *state = state::ToolsetState::not_configured(config.config.clone());
        } else {
            let config = config.config.to_full_config().ok_or_else(|| {
                Error::new("partial flag set to false, but incomplete config".to_owned())
            })?;

            let engine_options = config.to_engine_options();
            let mod_manager = ModManager::new_unchecked(&engine_options);
            let mut vfs = Vfs::new();
            // Test toolset config for errors
            vfs.init(&config.to_engine_options(), &mod_manager)
                .map_err(|e| {
                    Error::new(format!(
                        "failed to test config: could not initialize vfs: {}",
                        e
                    ))
                })?;

            config.write()?;
            *state = state::ToolsetState::configured(config);
        }
    }

    get_toolset_config(state).await
}

#[derive(Debug, Serialize, Deserialize)]
struct EditableMod {
    #[serde(flatten)]
    m: Mod,
    settings: Option<ModSettings>,
}

#[tauri::command]
async fn get_available_mods(state: tauri::State<'_, state::TauriState>) -> Result<Vec<Mod>> {
    let state = state.read().await;
    match *state {
        state::ToolsetState::Configured {
            ref mod_manager, ..
        } => {
            let mods = mod_manager
                .available_mods()
                .iter()
                .map(|m| Mod {
                    id: m.id().to_owned(),
                    name: m.name().to_owned(),
                    description: m.description().to_owned(),
                    version: m.version().to_owned(),
                    path: match m.path() {
                        ModPath::Path(p) => p.to_owned(),
                    },
                })
                .collect();
            Ok(mods)
        }
        _ => Err(Error::new(
            "get_available_mods mods not available in this editor state".to_owned(),
        )),
    }
}

#[tauri::command]
async fn get_editable_mods(state: tauri::State<'_, state::TauriState>) -> Result<Vec<EditableMod>> {
    let available_mods = get_available_mods(state.clone()).await?;
    let state = state.read().await;
    match *state {
        state::ToolsetState::Configured { ref config, .. } => {
            let mut editable_mods = vec![];

            for m in available_mods {
                if m.path.starts_with(&config.stracciatella_install_dir) {
                    continue;
                }
                let toolset_file = resolve_existing_components(
                    Path::new(ModSettings::filename()),
                    Some(&m.path),
                    true,
                );
                let settings = read_to_string(&toolset_file)
                    .await
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok());
                editable_mods.push(EditableMod {
                    m: m.clone(),
                    settings,
                });
            }

            Ok(editable_mods)
        }
        _ => Err(Error::new(
            "get_editable_mods mods not available in this editor state".to_owned(),
        )),
    }
}

#[tauri::command]
async fn set_selected_mod(
    state: tauri::State<'_, state::TauriState>,
    mod_id: String,
) -> Result<()> {
    let mut state = state.write().await;
    match *state {
        state::ToolsetState::Configured {
            ref config,
            ref mod_manager,
            ref mut opened_mod,
            ..
        } => {
            let mod_to_open = state::OpenedMod::new(config, mod_manager, &mod_id)?;
            *opened_mod = Some(mod_to_open);

            Ok(())
        }
        _ => Err(Error::new(
            "set_selected_mod mods not available in this editor state".to_owned(),
        )),
    }
}

fn get_selected_mod(state: &state::ToolsetState) -> Result<&state::OpenedMod> {
    match state {
        state::ToolsetState::Configured { opened_mod, .. } => {
            if let Some(opened_mod) = opened_mod.as_ref() {
                Ok(opened_mod)
            } else {
                Err(Error::new("no mod selected".to_owned()))
            }
        }
        _ => Err(Error::new(
            "get_selected_mod mods not available in this editor state".to_owned(),
        )),
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
    state: tauri::State<'_, state::TauriState>,
    file: String,
) -> Result<JsonFileWithSchema> {
    let state = state.read().await;
    match *state {
        state::ToolsetState::Configured {
            ref schema_manager, ..
        } => {
            let schema = schema_manager
                .get(Path::new(&file))
                .ok_or_else(|| Error::new(format!("schema for `{}` not found", file)))?;
            let schema = Value::from_str(schema.as_str())
                .map_err(|e| Error::new(format!("error decoding schema for `{}`: {}", file, e)))?;
            let selected_mod = get_selected_mod(&state)?;

            if file.contains("..") {
                return Err(Error::new("file path cannot contain `..`".to_owned()));
            }

            let path = get_mod_data_path(&selected_mod.m, &file);
            let content: Value = if path.exists() {
                let json = read_to_string(&path)
                    .await
                    .map_err(|e| Error::new(format!("error reading `{:?}`: {}", path, e)))?;
                stracciatella::json::de::from_string(&json).map_err(|e| {
                    Error::new(format!("error decoding data for `{:?}`: {}", path, e))
                })?
            } else {
                let mut f = selected_mod
                    .vfs
                    .open(&Nfc::caseless(&file))
                    .map_err(|e| Error::new(format!("error opening `{:?}`: {}", file, e)))?;
                let mut json = String::new();
                f.read_to_string(&mut json)
                    .map_err(|e| Error::new(format!("error reading `{:?}`: {}", file, e)))?;
                stracciatella::json::de::from_string(&json).map_err(|e| {
                    Error::new(format!("error decoding data for `{:?}`: {}", file, e))
                })?
            };

            Ok(JsonFileWithSchema { content, schema })
        }
        _ => Err(Error::new(
            "open_json_file_with_schema mods not available in this editor state".to_owned(),
        )),
    }
}

#[tauri::command]
async fn persist_text_file(
    state: tauri::State<'_, state::TauriState>,
    file_path: String,
    content: String,
) -> Result<()> {
    let state = state.read().await;

    if file_path.contains("..") {
        return Err(Error::new("file path cannot contain `..`".to_owned()));
    }

    match *state {
        state::ToolsetState::Configured { .. } => {
            let selected_mod = get_selected_mod(&state)?.clone();
            let path = get_mod_data_path(&selected_mod.m, &file_path);
            Ok(write(&path, &content).await?)
        }
        _ => Err(Error::new(
            "persist_text_file mods not available in this editor state".to_owned(),
        )),
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

    pub fn to_png_data(&self) -> core::result::Result<Vec<u8>, image::ImageError> {
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
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
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
    state: tauri::State<'_, state::TauriState>,
    file: String,
) -> Result<Base64Image> {
    if file.contains("..") {
        return Err(Error::new("file path cannot contain `..`".to_owned()));
    }

    let state = state.read().await;
    let selected_mod = get_selected_mod(&state)?.clone();
    let path = get_mod_data_path(&selected_mod.m, &file);
    let content: Vec<u8> = if path.exists() {
        let mut f = OpenOptions::new().open(&path).await?;
        let mut result = vec![];

        f.read_to_end(&mut result).await?;

        result
    } else {
        let mut f = selected_mod.vfs.open(&Nfc::caseless(&file))?;
        let mut result = vec![];

        f.read_to_end(&mut result)?;

        result
    };
    let stci = Stci::from_input(&mut content.as_slice())?;
    let size = match &stci {
        Stci::Indexed { sub_images, .. } => sub_images
            .get(0)
            .ok_or_else(|| {
                Error::new("indexed stci does not have at least one subimage".to_owned())
            })
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
            get_toolset_config,
            set_toolset_config,
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
