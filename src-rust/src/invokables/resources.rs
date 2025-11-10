use crate::{invokables::Invokable, state};
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use stracciatella::{unicode::Nfc, vfs::VfsLayer};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(tag = "type")]
pub enum ResourceEntry {
    Dir { path: String },
    File { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    path: String,
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
