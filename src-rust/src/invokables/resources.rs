use crate::{invokables::Invokable, state};
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{BufReader, Read as _};
use stracciatella::{unicode::Nfc, vfs::VfsLayer};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(tag = "type")]
pub enum ResourceEntry {
    Dir { path: String },
    File { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    path: String,
    mod_only: Option<bool>,
}

impl Invokable for List {
    type Output = HashSet<ResourceEntry>;

    fn name() -> &'static str {
        "resources/list"
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.path.contains("..") {
            return Err(anyhow!("must not contain `..`"));
        }
        Ok(())
    }

    fn invoke(&self, state: &state::AppState) -> anyhow::Result<Self::Output> {
        let state = state.read();
        let selected_mod = state.try_selected_mod()?;
        let mut result = HashSet::new();

        if !self.mod_only.unwrap_or(true) {
            let candidates = selected_mod.vfs.read_dir(&Nfc::caseless(&self.path)).ok();
            for candidate in candidates.iter().flatten() {
                let path = if self.path.is_empty() {
                    candidate.clone()
                } else {
                    Nfc::caseless(&format!("{}/{}", self.path, candidate))
                };
                // Workaround to determine whether the candidate is a file
                let is_file = selected_mod.vfs.open(&path).is_ok();
                let entry = if is_file {
                    ResourceEntry::File {
                        path: candidate.to_string().to_lowercase(),
                    }
                } else {
                    ResourceEntry::Dir {
                        path: candidate.to_string().to_lowercase(),
                    }
                };

                result.insert(entry);
            }
        }

        let dir = selected_mod.data_path(&self.path);
        for entry in fs::read_dir(&dir).into_iter().flatten() {
            let entry = entry.context("failed to read dir entry")?;
            let candidate = Nfc::from(entry.file_name().to_string_lossy().into_owned());

            let entry = if entry.path().is_file() {
                ResourceEntry::File {
                    path: candidate.to_string().to_lowercase(),
                }
            } else {
                ResourceEntry::Dir {
                    path: candidate.to_string().to_lowercase(),
                }
            };

            result.insert(entry);
        }

        Ok(result)
    }
}

/// Reads a file from the selected mod or the vanilla data, returned as base64
///
/// Returns `None` if the file exists in neither.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Read {
    file: String,
}

impl Invokable for Read {
    type Output = Option<String>;

    fn name() -> &'static str {
        "resources/read"
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.file.contains("..") {
            return Err(anyhow!("must not contain `..`"));
        }
        Ok(())
    }

    fn invoke(&self, state: &state::AppState) -> anyhow::Result<Self::Output> {
        let state = state.read();
        let Some(f) = state
            .try_open_file(&self.file)
            .context("failed to open file")?
        else {
            return Ok(None);
        };
        let mut f = BufReader::new(f);
        let mut content = vec![];

        f.read_to_end(&mut content).context("failed to read file")?;

        Ok(Some(base64::encode(&content)))
    }
}

/// Checks for each file whether it exists in the selected mod or the vanilla data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exists {
    files: Vec<String>,
}

impl Invokable for Exists {
    type Output = Vec<bool>;

    fn name() -> &'static str {
        "resources/exists"
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.files.iter().any(|file| file.contains("..")) {
            return Err(anyhow!("must not contain `..`"));
        }
        Ok(())
    }

    fn invoke(&self, state: &state::AppState) -> anyhow::Result<Self::Output> {
        let state = state.read();
        self.files
            .iter()
            .map(|file| {
                state
                    .try_file_exists(file)
                    .with_context(|| format!("failed to check whether `{}` exists", file))
            })
            .collect()
    }
}

/// Writes a file to the selected mod, `data` is base64 encoded
///
/// Missing directories are created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persist {
    file: String,
    data: String,
}

impl Invokable for Persist {
    type Output = ();

    fn name() -> &'static str {
        "resources/persist"
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.file.contains("..") {
            return Err(anyhow!("must not contain `..`"));
        }
        Ok(())
    }

    fn invoke(&self, state: &state::AppState) -> anyhow::Result<Self::Output> {
        let state = state.read();
        let selected_mod = state
            .try_selected_mod()
            .context("failed to get selected mod")?;
        let content = base64::decode(&self.data).context("failed to decode data")?;
        let path = selected_mod.data_path(&self.file);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create `{}`", parent.display()))?;
        }
        fs::write(&path, content)
            .with_context(|| format!("failed to write `{}`", path.display()))?;
        Ok(())
    }
}
