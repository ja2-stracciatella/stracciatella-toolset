use std::fs::{read_to_string, write};
use std::{path::Path, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use stracciatella::{unicode::Nfc, vfs::VfsLayer};

use crate::{
    error::{Error, Result},
    state,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenFileOptions {
    file: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonFileWithSchema {
    content: Value,
    schema: Value,
}

pub fn open_json_file_with_schema(
    state: &state::AppState,
    file: OpenFileOptions,
) -> Result<JsonFileWithSchema> {
    if file.file.contains("..") {
        return Err(Error::new("file path cannot contain `..`"));
    }

    let state = state.read();
    let schema_manager = state.try_schema_manager()?;
    let selected_mod = state.try_selected_mod()?;
    let schema = schema_manager
        .get(Path::new(&file.file))
        .ok_or_else(|| Error::new(format!("schema for `{}` not found", file.file)))?;
    let schema = Value::from_str(schema.as_str())?;
    let path = selected_mod.data_path(&file.file);

    let content: Value = if path.exists() {
        let json = read_to_string(&path)?;
        stracciatella::json::de::from_string(&json)
            .map_err(|e| Error::new(format!("error decoding data for `{:?}`: {}", file, e)))?
    } else {
        let mut f = selected_mod.vfs.open(&Nfc::caseless(&file.file))?;
        let mut json = String::new();
        f.read_to_string(&mut json)?;
        stracciatella::json::de::from_string(&json)
            .map_err(|e| Error::new(format!("error decoding data for `{:?}`: {}", file, e)))?
    };

    Ok(JsonFileWithSchema { content, schema })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistFileOptions {
    file: String,
    content: Value,
}

pub fn persist_json_file(state: &state::AppState, file: PersistFileOptions) -> Result<()> {
    if file.file.contains("..") {
        return Err(Error::new("file path cannot contain `..`"));
    }

    let state = state.read();
    let selected_mod = state.try_selected_mod()?;
    let path = selected_mod.data_path(&file.file);
    let content = serde_json::to_string(&file.content)?;

    write(&path, &content)?;

    Ok(())
}
