use crate::invokables::Invokable;
use crate::state;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use stracciatella::mods::ModManager;
use stracciatella::{fs::resolve_existing_components, mods::ModPath};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct GetAvailableMods;

impl Invokable for GetAvailableMods {
    type Output = Vec<Mod>;

    fn name() -> &'static str {
        "get_available_mods"
    }

    fn invoke(&self, state: &state::AppState) -> anyhow::Result<Self::Output> {
        let state = state.read();
        let mod_manager = state
            .try_mod_manager()
            .context("failed to get mod manager")?;
        Ok(mod_manager
            .available_mods()
            .iter()
            .map(|m| Mod::from_stracciatella(m))
            .collect())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetEditableMods;

impl Invokable for GetEditableMods {
    type Output = Vec<EditableMod>;

    fn name() -> &'static str {
        "get_editable_mods"
    }

    fn invoke(&self, state: &state::AppState) -> anyhow::Result<Self::Output> {
        let available_mods = (GetAvailableMods {})
            .invoke(state)
            .context("failed to get available mods")?;
        let state = state.read();
        let config = state.try_config().context("failed to get config")?;
        let mut editable_mods = vec![];

        for m in available_mods {
            if m.path.starts_with(&config.stracciatella_install_dir) {
                continue;
            }
            let toolset_file = resolve_existing_components(
                Path::new(ModSettings::filename()),
                Some(&m.path),
                true,
            );
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetSelectedMod {
    mod_id: String,
}

impl Invokable for SetSelectedMod {
    type Output = Mod;

    fn name() -> &'static str {
        "set_selected_mod"
    }

    fn invoke(&self, state: &state::AppState) -> anyhow::Result<Self::Output> {
        let mut state = state.write();

        match *state {
            state::ToolsetState::Configured {
                ref config,
                ref mod_manager,
                ref mut opened_mod,
                ..
            } => {
                let mod_to_open = state::OpenedMod::new(config, mod_manager, &self.mod_id)
                    .context("failed to initialize open mod")?;
                let m = Mod::from_stracciatella(&mod_to_open.m);
                *opened_mod = Some(mod_to_open);
                Ok(m)
            }
            _ => Err(anyhow!("toolset state not configured")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewMod {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
}

impl Invokable for NewMod {
    type Output = Mod;

    fn name() -> &'static str {
        "new_mod"
    }

    fn invoke(&self, state: &state::AppState) -> Result<Self::Output> {
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
                let mod_dir = mods_dir.join(&self.id);
                let data_dir = mod_dir.join("data");
                let manifest_path = mod_dir.join("manifest.json");

                let manifest_json = {
                    let mut m = serde_json::json!({
                      "name": self.name.clone(),
                      "version": self.version.clone()
                    });
                    let o = m
                        .as_object_mut()
                        .ok_or_else(|| anyhow!("manifest should be an object"))?;
                    if let Some(description) = &self.description {
                        o.insert(
                            "description".to_owned(),
                            serde_json::json!(description.clone()),
                        );
                    }

                    m
                };

                if std::fs::exists(&mod_dir).context("failed to check mod directory existence")? {
                    return Err(anyhow!("mod with id {} already exists", self.id));
                }

                std::fs::create_dir_all(&mod_dir).context("failed to create mod dir")?;
                std::fs::create_dir_all(&data_dir).context("failed to create data dir")?;
                std::fs::write(
                    manifest_path,
                    serde_json::to_string_pretty(&manifest_json)
                        .context("failed to serialize manifest")?,
                )
                .context("failed to write manifest")?;

                // We need to reload the mods list to reflect the new mod
                // FIXME: Would probably be better to use a method on ModManager
                *mod_manager = ModManager::new(&config.to_engine_options())
                    .context("failed to reinitialize mod manager")?;

                let mod_to_open = state::OpenedMod::new(config, mod_manager, &self.id)
                    .context("failed to initialize open mod")?;
                let m = Mod::from_stracciatella(&mod_to_open.m);
                *opened_mod = Some(mod_to_open);
                Ok(m)
            }
            _ => Err(anyhow!("toolset state not configured")),
        }
    }
}
