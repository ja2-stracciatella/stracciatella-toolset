use crate::{config::PartialToolsetConfig, invokables::Invokable, state};
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
        Ok(match *state {
            state::ToolsetState::Configured { ref config, .. } => SerializableToolsetConfig {
                partial: false,
                config: config.clone().into(),
            },
            state::ToolsetState::NotConfigured { ref config } => SerializableToolsetConfig {
                partial: true,
                config: config.clone(),
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsetUpdateConfig {
    config: PartialToolsetConfig,
}

impl Invokable for ToolsetUpdateConfig {
    type Output = SerializableToolsetConfig;

    fn name() -> &'static str {
        "toolset/updateConfig"
    }

    fn invoke(&self, state: &state::AppState) -> Result<Self::Output> {
        {
            if let Some(config) = self.config.to_full_config() {
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
                *state = state::ToolsetState::not_configured(self.config.clone());
            }
        }

        (ToolsetReadConfig {})
            .invoke(state)
            .context("failed to get toolset config after update")
    }
}
