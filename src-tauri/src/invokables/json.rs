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
    if file.contains("..") {
        return Err(Error::new("file path cannot contain `..`"));
    }

    let state = state.read().await;
    let schema_manager = state.try_schema_manager()?;
    let selected_mod = state.try_selected_mod()?;
    let schema = schema_manager
        .get(Path::new(&file))
        .ok_or_else(|| Error::new(format!("schema for `{}` not found", file)))?;
    let schema = Value::from_str(schema.as_str())?;
    let path = selected_mod.data_path(&file);

    let content: Value = if path.exists() {
        let json = read_to_string(&path).await?;
        stracciatella::json::de::from_string(&json)
            .map_err(|e| Error::new(format!("error decoding data for `{:?}`: {}", file, e)))?
    } else {
        let mut f = selected_mod.vfs.open(&Nfc::caseless(&file))?;
        let mut json = String::new();
        f.read_to_string(&mut json)?;
        stracciatella::json::de::from_string(&json)
            .map_err(|e| Error::new(format!("error decoding data for `{:?}`: {}", file, e)))?
    };

    Ok(JsonFileWithSchema { content, schema })
}

#[tauri::command]
pub async fn persist_json_file(
    state: tauri::State<'_, state::TauriState>,
    path: String,
    content: Value,
) -> Result<()> {
    if path.contains("..") {
        return Err(Error::new("file path cannot contain `..`"));
    }

    let state = state.read().await;
    let selected_mod = state.try_selected_mod()?;
    let path = selected_mod.data_path(&path);
    let content = serde_json::to_string(&content)?;

    write(&path, &content).await?;

    Ok(())
}
