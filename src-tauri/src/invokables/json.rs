use std::{path::Path, str::FromStr};

use async_fs::{read_to_string, write};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use stracciatella::{unicode::Nfc, vfs::VfsLayer};

use crate::{
    error::{Error, Result},
    state,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonFileWithSchema {
    content: Value,
    schema: Value,
}

#[tauri::command]
pub async fn open_json_file_with_schema(
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
            let selected_mod = super::mods::get_selected_mod(&state)?;

            if file.contains("..") {
                return Err(Error::new("file path cannot contain `..`".to_owned()));
            }

            let path = super::mods::get_mod_data_path(&selected_mod.m, &file);
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
pub async fn persist_json_file(
    state: tauri::State<'_, state::TauriState>,
    path: String,
    content: Value,
) -> Result<()> {
    let state = state.read().await;

    if path.contains("..") {
        return Err(Error::new("file path cannot contain `..`".to_owned()));
    }

    let selected_mod = super::mods::get_selected_mod(&state)?.clone();
    let path = super::mods::get_mod_data_path(&selected_mod.m, &path);
    let content = serde_json::to_string(&content)?;

    write(&path, &content).await?;

    Ok(())
}
