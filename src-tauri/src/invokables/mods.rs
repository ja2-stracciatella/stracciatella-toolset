use std::path::{Path, PathBuf};

use async_fs::read_to_string;
use serde::{Deserialize, Serialize};
use stracciatella::{fs::resolve_existing_components, mods::ModPath};

use crate::{
    error::{Error, Result},
    state,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mod {
    id: String,
    name: String,
    description: String,
    version: String,
    path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ModSettings {
    required_mods: Vec<String>,
}

impl ModSettings {
    fn filename() -> &'static str {
        "toolset.json"
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EditableMod {
    #[serde(flatten)]
    m: Mod,
    settings: Option<ModSettings>,
}

#[tauri::command]
pub async fn get_available_mods(state: tauri::State<'_, state::TauriState>) -> Result<Vec<Mod>> {
    let state = state.read().await;
    let mod_manager = state.try_mod_manager()?;
    Ok(mod_manager
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
        .collect())
}

#[tauri::command]
pub async fn get_editable_mods(
    state: tauri::State<'_, state::TauriState>,
) -> Result<Vec<EditableMod>> {
    let available_mods = get_available_mods(state.clone()).await?;
    let state = state.read().await;
    let config = state.try_config()?;
    let mut editable_mods = vec![];

    for m in available_mods {
        if m.path.starts_with(&config.stracciatella_install_dir) {
            continue;
        }
        let toolset_file =
            resolve_existing_components(Path::new(ModSettings::filename()), Some(&m.path), true);
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

#[tauri::command]
pub async fn set_selected_mod(
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
            "failed to set selected mod in current editor state",
        )),
    }
}
