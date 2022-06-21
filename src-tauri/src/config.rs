use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use stracciatella::config::{find_stracciatella_home, EngineOptions};

use crate::{Error, Result};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialToolsetConfig {
    stracciatella_home: Option<String>,
    vanilla_game_dir: Option<String>,
    stracciatella_install_dir: Option<String>,
    last_selected_mod: Option<String>,
}

impl PartialToolsetConfig {
    pub fn to_full_config(&self) -> Option<ToolsetConfig> {
        Some(ToolsetConfig {
            stracciatella_home: self.stracciatella_home.as_ref()?.clone(),
            vanilla_game_dir: self.vanilla_game_dir.as_ref()?.clone(),
            stracciatella_install_dir: self.stracciatella_install_dir.as_ref()?.clone(),
            last_selected_mod: self.last_selected_mod.clone(),
        })
    }
}

impl PartialToolsetConfig {
    pub fn guess() -> PartialToolsetConfig {
        let mut toolset_config = PartialToolsetConfig::default();
        let stracciatella_home = find_stracciatella_home().ok();
        toolset_config.stracciatella_home = stracciatella_home
            .as_ref()
            .map(|v| v.to_string_lossy().to_string());
        if let Some(stracciatella_home) = stracciatella_home {
            let args = vec!["stracciatella-toolset".to_owned()];
            let engine_options = EngineOptions::from_home_and_args(&stracciatella_home, &args).ok();

            toolset_config.vanilla_game_dir = engine_options
                .as_ref()
                .map(|v| v.vanilla_game_dir.to_string_lossy().to_string());
        }

        toolset_config
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsetConfig {
    pub stracciatella_home: String,
    pub vanilla_game_dir: String,
    pub stracciatella_install_dir: String,
    pub last_selected_mod: Option<String>,
}

impl ToolsetConfig {
    pub fn path() -> Result<PathBuf> {
        let project_dirs =
            directories::ProjectDirs::from("io", "ja2-stracciatella", "stracciatella-toolset")
                .ok_or_else(|| {
                    Error::new("could not determine toolset config directory".to_owned())
                })?;
        let config_dir = project_dirs.config_dir();

        Ok(config_dir.join("toolset-config.json"))
    }

    pub fn to_engine_options(&self) -> EngineOptions {
        EngineOptions {
            stracciatella_home: (&self.stracciatella_home).into(),
            assets_dir: (&self.stracciatella_install_dir).into(),
            vanilla_game_dir: (&self.vanilla_game_dir).into(),
            ..EngineOptions::default()
        }
    }

    /// Tries to read toolset config from home
    /// Returns either a full toolset config or an error why the full config could not be read and a partial toolset config that is populated as much as possible
    pub fn read() -> core::result::Result<ToolsetConfig, (Error, PartialToolsetConfig)> {
        let path = ToolsetConfig::path().map_err(|e| (e, PartialToolsetConfig::guess()))?;
        let config = if path.exists() {
            std::fs::read_to_string(&path)
                .map_err(Error::from)
                .and_then(|v| serde_json::from_str(&v).map_err(Error::from))
                .map_err(|e| (e, PartialToolsetConfig::guess()))?
        } else {
            PartialToolsetConfig::guess()
        };

        if let Some(config) = config.to_full_config() {
            Ok(config)
        } else {
            Err((
                Error::new("failed to load full configuration".to_owned()),
                PartialToolsetConfig::guess(),
            ))
        }
    }

    /// Tries to write toolset config to home
    pub fn write(&self) -> Result<()> {
        let contents = serde_json::to_string(self).map_err(Error::from)?;
        let path = Self::path()?;
        let dir = path.parent();
        if let Some(dir) = dir {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, contents)?;
        Ok(())
    }
}

impl Into<PartialToolsetConfig> for ToolsetConfig {
    fn into(self) -> PartialToolsetConfig {
        PartialToolsetConfig {
            stracciatella_home: Some(self.stracciatella_home),
            vanilla_game_dir: Some(self.vanilla_game_dir),
            stracciatella_install_dir: Some(self.stracciatella_install_dir),
            last_selected_mod: self.last_selected_mod,
        }
    }
}
