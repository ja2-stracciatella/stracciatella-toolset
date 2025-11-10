use crate::{cache, config::PartialToolsetConfig, invokables::Invokable, state};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use stracciatella::{mods::ModManager, vfs::Vfs};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableToolsetConfig {
    partial: bool,
    config: PartialToolsetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsetReadConfig;

impl Invokable for ToolsetReadConfig {
    type Output = SerializableToolsetConfig;

    fn name() -> &'static str {
        "toolset/readConfig"
    }

    fn invoke(&self, state: &state::AppState) -> Result<Self::Output> {
        let state = state.read();
        let mut config = match *state {
            state::ToolsetState::Configured { ref config, .. } => SerializableToolsetConfig {
                partial: false,
                config: config.clone().into(),
            },
            state::ToolsetState::NotConfigured { ref config } => SerializableToolsetConfig {
                partial: true,
                config: config.clone(),
            },
        };

        // Update JSON cache if necessary
        let json_cache_dir = cache::get_json_cache_dir()
            .map(|v| v.to_string_lossy().to_string())
            .context("failed to determine JSON cache dir")?;
        let uses_json_cache = config.config.stracciatella_install_dir.is_none()
            || config.config.stracciatella_install_dir.as_ref() == Some(&json_cache_dir);

        if uses_json_cache {
            cache::update_json_cache().context("failed to update JSON cache")?;
        }
        // Set default vanilla game directory to JSON cache directory
        if uses_json_cache {
            config.config.stracciatella_install_dir = Some(json_cache_dir);
        }

        Ok(config)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsetUpdateConfig(PartialToolsetConfig);

impl Invokable for ToolsetUpdateConfig {
    type Output = SerializableToolsetConfig;

    fn name() -> &'static str {
        "toolset/updateConfig"
    }

    fn invoke(&self, state: &state::AppState) -> Result<Self::Output> {
        {
            if let Some(config) = self.0.to_full_config() {
                // Test toolset config for errors
                let engine_options = config.to_engine_options();
                let mod_manager = ModManager::new_unchecked(&engine_options);
                let mut vfs = Vfs::new();
                vfs.init(&config.to_engine_options(), &mod_manager)
                    .map_err(|e| anyhow!("{}", e))
                    .context("failed to initialize vfs")
                    .context("failed to test config")?;

                config.write()?;
                {
                    let mut state = state.write();
                    *state = state::ToolsetState::configured(config);
                }
            } else {
                let mut state = state.write();
                *state = state::ToolsetState::not_configured(self.0.clone());
            }
        }

        (ToolsetReadConfig {})
            .invoke(state)
            .context("failed to get toolset config after update")
    }
}
