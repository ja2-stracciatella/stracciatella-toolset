use serde::{Deserialize, Serialize};
use stracciatella::{mods::ModManager, vfs::Vfs};

use crate::{
    config,
    error::{Error, Result},
    state,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableToolsetConfig {
    partial: bool,
    config: config::PartialToolsetConfig,
}

#[tauri::command]
pub async fn get_toolset_config(
    state: tauri::State<'_, state::TauriState>,
) -> Result<SerializableToolsetConfig> {
    let state = state.read().await;
    match *state {
        state::ToolsetState::Configured { ref config, .. } => Ok(SerializableToolsetConfig {
            partial: false,
            config: config.clone().into(),
        }),
        state::ToolsetState::NotConfigured { ref config } => Ok(SerializableToolsetConfig {
            partial: true,
            config: config.clone(),
        }),
    }
}

#[tauri::command]
pub async fn set_toolset_config(
    state: tauri::State<'_, state::TauriState>,
    config: SerializableToolsetConfig,
) -> Result<SerializableToolsetConfig> {
    {
        let mut state = state.write().await;

        if config.partial {
            *state = state::ToolsetState::not_configured(config.config.clone());
        } else {
            let config = config.config.to_full_config().ok_or_else(|| {
                Error::new("failed to create full config, but partial flag set to true")
            })?;

            // Test toolset config for errors
            let engine_options = config.to_engine_options();
            let mod_manager = ModManager::new_unchecked(&engine_options);
            let mut vfs = Vfs::new();
            vfs.init(&config.to_engine_options(), &mod_manager)
                .map_err(|e| {
                    Error::new(format!(
                        "failed to test config: could not initialize vfs: {}",
                        e
                    ))
                })?;

            config.write()?;
            *state = state::ToolsetState::configured(config);
        }
    }

    get_toolset_config(state).await
}
