use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use stracciatella::config::{find_stracciatella_home, EngineOptions};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PartialToolsetConfig {
  stracciatella_home: Option<String>,
  vanilla_game_dir: Option<String>,
  stracciatella_install_dir: Option<String>,
  last_selected_mod: Option<String>,
}

impl PartialToolsetConfig {
  pub fn to_config(&self) -> Option<ToolsetConfig> {
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
pub struct ToolsetConfig {
  pub stracciatella_home: String,
  pub vanilla_game_dir: String,
  pub stracciatella_install_dir: String,
  pub last_selected_mod: Option<String>,
}

impl ToolsetConfig {
  pub fn path() -> Option<PathBuf> {
    let project_dirs =
      directories::ProjectDirs::from("io", "ja2-stracciatella", "stracciatella-toolset")?;
    let config_dir = project_dirs.config_dir();

    Some(config_dir.join("toolset-config.json"))
  }

  pub fn to_engine_options(&self) -> EngineOptions {
    EngineOptions {
      stracciatella_home: (&self.stracciatella_home).into(),
      assets_dir: (&self.stracciatella_install_dir).into(),
      vanilla_game_dir: (&self.vanilla_game_dir).into(),
      ..EngineOptions::default()
    }
  }
}
