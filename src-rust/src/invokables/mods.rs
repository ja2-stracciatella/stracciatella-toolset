use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use stracciatella::mods::ModManager;
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

impl Mod {
    pub fn from_stracciatella(m: &stracciatella::mods::Mod) -> Self {
        Mod {
            id: m.id().to_owned(),
            name: m.name().to_owned(),
            description: m.description().to_owned(),
            version: m.version().to_owned(),
            path: match m.path() {
                ModPath::Path(p) => p.to_owned(),
            },
        }
    }
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

pub fn get_available_mods(state: &state::AppState) -> Result<Vec<Mod>> {
    let state = state.read();
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

pub fn get_editable_mods(state: &state::AppState) -> Result<Vec<EditableMod>> {
    let available_mods = get_available_mods(state)?;
    let state = state.read();
    let config = state.try_config()?;
    let mut editable_mods = vec![];

    for m in available_mods {
        if m.path.starts_with(&config.stracciatella_install_dir) {
            continue;
        }
        let toolset_file =
            resolve_existing_components(Path::new(ModSettings::filename()), Some(&m.path), true);
        let settings = read_to_string(&toolset_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());
        editable_mods.push(EditableMod {
            m: m.clone(),
            settings,
        });
    }

    Ok(editable_mods)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedMod {
    mod_id: String,
}

pub fn set_selected_mod(state: &state::AppState, selected_mod: SelectedMod) -> Result<Mod> {
    let mut state = state.write();

    match *state {
        state::ToolsetState::Configured {
            ref config,
            ref mod_manager,
            ref mut opened_mod,
            ..
        } => {
            let mod_to_open = state::OpenedMod::new(config, mod_manager, &selected_mod.mod_id)?;
            let m = Mod::from_stracciatella(&mod_to_open.m);
            *opened_mod = Some(mod_to_open);
            Ok(m)
        }
        _ => Err(Error::new(
            "failed to set selected mod in current editor state",
        )),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewMod {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
}

pub fn create_new_mod(state: &state::AppState, new_mod: NewMod) -> Result<Mod> {
    let mut state = state.write();

    match *state {
        state::ToolsetState::Configured {
            ref config,
            ref mut mod_manager,
            ref mut opened_mod,
            ..
        } => {
            let mods_dir = resolve_existing_components(
                Path::new("mods"),
                Some(&Path::new(&config.stracciatella_home)),
                true,
            );
            let mod_dir = mods_dir.join(&new_mod.id);
            let data_dir = mod_dir.join("data");
            let manifest_path = mod_dir.join("manifest.json");

            let manifest_json = {
                let mut m = serde_json::json!({
                  "name": new_mod.name.clone(),
                  "version": new_mod.version.clone()
                });
                let o = m
                    .as_object_mut()
                    .ok_or_else(|| Error::new("manifest should be an object"))?;
                if let Some(description) = &new_mod.description {
                    o.insert(
                        "description".to_owned(),
                        serde_json::json!(description.clone()),
                    );
                }

                m
            };

            if std::fs::exists(&mod_dir)? {
                return Err(Error::new(format!(
                    "mod with id {} already exists",
                    new_mod.id
                )));
            }

            std::fs::create_dir_all(&mod_dir)?;
            std::fs::create_dir_all(&data_dir)?;
            std::fs::write(manifest_path, serde_json::to_string_pretty(&manifest_json)?)?;

            // We need to reload the mods list to reflect the new mod
            // FIXME: Would probably be better to use a method on ModManager
            *mod_manager = ModManager::new(&config.to_engine_options())?;

            let mod_to_open = state::OpenedMod::new(config, mod_manager, &new_mod.id)?;
            let m = Mod::from_stracciatella(&mod_to_open.m);
            *opened_mod = Some(mod_to_open);
            Ok(m)
        }
        _ => Err(Error::new("not configured")),
    }
}
