use std::collections::HashSet;
use std::fs;

use serde::{Deserialize, Serialize};
use stracciatella::{unicode::Nfc, vfs::VfsLayer};

use crate::{
    error::{Error, Result},
    state,
};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(tag = "type")]
pub enum DirEntry {
    Dir { path: String },
    File { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesParams {
  dir: String,
}

pub fn list_resources(
    state: &state::AppState,
    params: ListResourcesParams,
) -> Result<HashSet<DirEntry>> {
    if params.dir.contains("..") {
        return Err(Error::new("file path cannot contain `..`"));
    }

    let state = state.read();
    let selected_mod = state.try_selected_mod()?;
    let mut result = HashSet::new();

    let candidates = selected_mod.vfs.read_dir(&Nfc::caseless(&params.dir)).ok();
    for candidate in candidates.iter().flatten() {
        let path = if params.dir.is_empty() {
            candidate.clone()
        } else {
            Nfc::caseless(&format!("{}/{}", params.dir, candidate))
        };
        // Workaround to determine whether the candidate is a file
        let is_file = selected_mod.vfs.open(&path).is_ok();
        let entry = if is_file {
            DirEntry::File {
                path: candidate.to_string().to_lowercase(),
            }
        } else {
            DirEntry::Dir {
                path: candidate.to_string().to_lowercase(),
            }
        };

        result.insert(entry);
    }

    let dir = selected_mod.data_path(&params.dir);
    for entry in fs::read_dir(&dir).into_iter().flatten() {
        let entry = entry?;
        let candidate = Nfc::from(entry.file_name().to_string_lossy().into_owned());

        let entry = if entry.path().is_file() {
            DirEntry::File {
                path: candidate.to_string().to_lowercase(),
            }
        } else {
            DirEntry::Dir {
                path: candidate.to_string().to_lowercase(),
            }
        };

        result.insert(entry);
    }

    Ok(result)
}
