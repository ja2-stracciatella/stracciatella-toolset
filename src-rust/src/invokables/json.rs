use crate::invokables::Invokable;
use crate::state;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{read_to_string, write};
use std::io::{self, ErrorKind};
use std::path::Path;
use stracciatella::unicode::Nfc;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Filename(String);

impl Filename {
    fn validate(&self) -> Result<()> {
        if self.0.contains("..") {
            return Err(anyhow!("must not contain `..`"));
        }
        if !self.0.ends_with(".json") {
            return Err(anyhow!("must end with `.json`"));
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
pub struct JsonFileWithSchema {
    schema: Value,
    vanilla: Value,
    #[serde(flatten)]
    persisted: Persisted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenJsonWithSchema {
    filename: Filename,
}

impl Invokable for OpenJsonWithSchema {
    type Output = JsonFileWithSchema;

    fn name() -> &'static str {
        return "open_json_with_schema";
    }

    fn validate(&self) -> Result<()> {
        self.filename
            .validate()
            .context("failed to validate filename")?;
        Ok(())
    }

    fn invoke(&self, state: &state::AppState) -> Result<Self::Output> {
        let filename = &self.filename;
        let state = state.read();
        let schema_manager = state
            .try_schema_manager()
            .context("failed to get schema manager")?;
        let selected_mod = state
            .try_selected_mod()
            .context("failed to get selected mod")?;
        let schema = schema_manager
            .get(&Path::new(filename.as_str()))
            .ok_or_else(|| anyhow!("schema for `{}` not found", filename.as_str()))?;
        let schema = schema.as_value().clone();
        let path = selected_mod.data_path(filename.as_str());
        let patch_path = selected_mod.data_path(filename.patch_filename().as_str());

        let vanilla = selected_mod
            .vfs
            .read_patched_json(&Nfc::caseless(filename.as_str()))
            .context("failed to read vanilla json")?;
        let value: Option<Value> = if path.exists() {
            let json = read_to_string(&path)?;
            stracciatella::json::de::from_string(&json)
                .map_err(|e| anyhow!("{}", e))
                .context("failed to parse json")?
        } else {
            None
        };
        let patch: Option<Vec<Value>> = if patch_path.exists() {
            let json = read_to_string(&patch_path)?;
            stracciatella::json::de::from_string(&json)
                .map_err(|e| anyhow!("{}", e))
                .context("failed to parse patch json")?
        } else {
            None
        };

        Ok(JsonFileWithSchema {
            schema,
            vanilla,
            persisted: Persisted { value, patch },
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistJson {
    filename: Filename,
    #[serde(flatten)]
    values: Persisted,
}

impl Invokable for PersistJson {
    type Output = JsonFileWithSchema;

    fn name() -> &'static str {
        "persist_json"
    }

    fn validate(&self) -> Result<()> {
        self.filename
            .validate()
            .context("failed to validate filename")?;
        Ok(())
    }

    fn invoke(&self, app_state: &state::AppState) -> Result<Self::Output> {
        let filename = &self.filename;
        let state = app_state.read();
        let selected_mod = state
            .try_selected_mod()
            .context("failed to get selected mod")?;

        let path = selected_mod.data_path(filename.as_str());
        if let Some(mod_value) = &self.values.value {
            let content =
                serde_json::to_string_pretty(&mod_value).context("failed to serialize value")?;
            write(&path, &content).context("failed to write value")?;
        } else {
            delete_file(&path).context("failed to delete value")?;
        }

        let path = selected_mod.data_path(filename.patch_filename().as_str());
        if let Some(mod_patch_value) = &self.values.patch {
            if !mod_patch_value.is_empty() {
                let content = serde_json::to_string_pretty(&mod_patch_value)
                    .context("failed to serialize patch value")?;
                write(&path, &content).context("failed to write patch value")?;
            } else {
                delete_file(&path).context("failed to delete patch value")?;
            }
        } else {
            delete_file(&path).context("failed to delete patch value")?;
        }

        Ok(OpenJsonWithSchema {
            filename: self.filename.clone(),
        }
        .invoke(app_state)
        .context("failed to get value after update")?)
    }
}

fn delete_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    if let Err(e) = std::fs::remove_file(path.as_ref()) {
        if e.kind() != ErrorKind::NotFound {
            return Err(e.into());
        }
    }
    return Ok(());
}
