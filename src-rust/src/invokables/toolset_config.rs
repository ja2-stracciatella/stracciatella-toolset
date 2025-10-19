use crate::{config, invokables::Invokable, state};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use stracciatella::{mods::ModManager, vfs::Vfs};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableToolsetConfig {
    partial: bool,
    config: config::PartialToolsetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetToolsetConfig;

impl Invokable for GetToolsetConfig {
    type Output = SerializableToolsetConfig;

    fn name() -> &'static str {
        "get_toolset_config"
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
pub struct SetToolsetConfig {
    config: SerializableToolsetConfig,
}

impl Invokable for SetToolsetConfig {
    type Output = SerializableToolsetConfig;

    fn name() -> &'static str {
        "set_toolset_config"
    }

    fn invoke(&self, state: &state::AppState) -> Result<Self::Output> {
        {
            let mut state = state.write();

            if self.config.partial {
                *state = state::ToolsetState::not_configured(self.config.config.clone());
            } else {
                let config = self.config.config.to_full_config().ok_or_else(|| {
                    anyhow!("failed to create full config, but partial flag set to true")
                })?;

                // Test toolset config for errors
                let engine_options = config.to_engine_options();
                let mod_manager = ModManager::new_unchecked(&engine_options);
                let mut vfs = Vfs::new();
                vfs.init(&config.to_engine_options(), &mod_manager)
                    .map_err(|e| anyhow!("{}", e))
                    .context("failed to initialize vfs")
                    .context("failed to test config")?;

                config.write()?;
                *state = state::ToolsetState::configured(config);
            }
        }

        (GetToolsetConfig {})
            .invoke(state)
            .context("failed to get toolset config after update")
    }
}
