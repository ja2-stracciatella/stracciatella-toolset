use std::fs::{read_to_string, write};
use std::io::{self, ErrorKind};
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use stracciatella::unicode::Nfc;

use crate::{
    error::{Error, Result},
    state,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Filename(String);

impl Filename {
    fn validate(&self) -> Result<()> {
        if self.0.contains("..") {
            return Err(Error::new("file path cannot contain `..`"));
        }
        if !self.0.ends_with(".json") {
            return Err(Error::new("file path must end with `.json`"));
        }
        Ok(())
    }

    fn as_str(&self) -> &str {
        self.as_ref()
    }

    fn patch_filename(&self) -> Self {
        let mut f = self.0.strip_suffix(".json").unwrap().to_owned();
        f.push_str(".patch.json");
        Self(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persisted {
    value: Option<Value>,
    patch: Option<Vec<Value>>,
}

impl AsRef<str> for Filename {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFileParams {
    filename: Filename,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonFileWithSchema {
    schema: Value,
    vanilla: Value,
    #[serde(flatten)]
    persisted: Persisted,
}

pub fn open_json_with_schema(
    state: &state::AppState,
    params: OpenFileParams,
) -> Result<JsonFileWithSchema> {
    let filename = &params.filename;
    filename.validate()?;

    let state = state.read();
    let schema_manager = state.try_schema_manager()?;
    let selected_mod = state.try_selected_mod()?;
    let schema = schema_manager
        .get(&Path::new(filename.as_str()))
        .ok_or_else(|| Error::new(format!("schema for `{}` not found", filename.as_str())))?;
    let schema = Value::from_str(schema.as_str())?;
    let path = selected_mod.data_path(filename.as_str());
    let patch_path = selected_mod.data_path(filename.patch_filename().as_str());

    let vanilla = selected_mod
        .vfs
        .read_patched_json(&Nfc::caseless(filename.as_str()))?;
    let value: Option<Value> = if path.exists() {
        let json = read_to_string(&path)?;
        stracciatella::json::de::from_string(&json).map_err(|e| {
            Error::new(format!(
                "error decoding patch data for `{:?}`: {}",
                filename, e
            ))
        })?
    } else {
        None
    };
    let patch: Option<Vec<Value>> = if patch_path.exists() {
        let json = read_to_string(&patch_path)?;
        stracciatella::json::de::from_string(&json).map_err(|e| {
            Error::new(format!(
                "error decoding patch data for `{:?}`: {}",
                filename, e
            ))
        })?
    } else {
        None
    };

    Ok(JsonFileWithSchema {
        schema,
        vanilla,
        persisted: Persisted { value, patch },
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistFileParams {
    filename: Filename,
    #[serde(flatten)]
    values: Persisted,
}

pub fn persist_json(state: &state::AppState, params: PersistFileParams) -> Result<Persisted> {
    let filename = &params.filename;
    filename.validate()?;

    let state = state.read();
    let selected_mod = state.try_selected_mod()?;

    let path = selected_mod.data_path(filename.as_str());
    if let Some(mod_value) = &params.values.value {
        let content = serde_json::to_string_pretty(&mod_value)?;
        write(&path, &content)?;
    } else {
        delete_file(&path)?;
    }

    let mut rewrite_patch_to_none = false;
    let path = selected_mod.data_path(filename.patch_filename().as_str());
    if let Some(mod_patch_value) = &params.values.patch {
        if !mod_patch_value.is_empty() {
            let content = serde_json::to_string_pretty(&mod_patch_value)?;
            write(&path, &content)?;
        } else {
            rewrite_patch_to_none = true;
            delete_file(&path)?;
        }
    } else {
        delete_file(&path)?;
    }

    Ok(Persisted {
        value: params.values.value.clone(),
        patch: if rewrite_patch_to_none {
            None
        } else {
            params.values.patch.clone()
        },
    })
}

fn delete_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    if let Err(e) = std::fs::remove_file(path.as_ref()) {
        if e.kind() != ErrorKind::NotFound {
            return Err(e.into());
        }
    }
    return Ok(());
}
