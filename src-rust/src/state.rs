use anyhow::{anyhow, Result};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use stracciatella::{
    fs::resolve_existing_components,
    mods::{Mod, ModManager, ModPath},
    schemas::SchemaManager,
    vfs::Vfs,
};

use crate::config::{self, PartialToolsetConfig, ToolsetConfig};

#[derive(Debug, Clone)]
pub struct OpenedMod {
    pub vfs: Arc<Vfs>,
    pub m: Mod,
}

impl OpenedMod {
    pub fn new(
        config: &ToolsetConfig,
        mod_manager: &ModManager,
        mod_id: &str,
    ) -> Result<OpenedMod> {
        let m = mod_manager
            .get_mod_by_id(mod_id)
            .ok_or_else(|| anyhow!("failed to find mod `{}`", mod_id))?
            .clone();
        let mut vfs = Vfs::new();
        let engine_options = config.to_engine_options();
        vfs.init(&engine_options, mod_manager)
            .map_err(|e| anyhow!("failed to initialize vfs: {}", e))?;

        Ok(OpenedMod {
            vfs: Arc::new(vfs),
            m,
        })
    }

    pub fn data_path(&self, file_path: &str) -> PathBuf {
        let file_path = PathBuf::from("data").join(file_path);
        match self.m.path() {
            ModPath::Path(p) => resolve_existing_components(Path::new(&file_path), Some(p), true),
        }
    }
}

pub enum ToolsetState {
    NotConfigured {
        config: config::PartialToolsetConfig,
    },
    Configured {
        config: ToolsetConfig,
        schema_manager: SchemaManager,
        mod_manager: ModManager,
        opened_mod: Option<OpenedMod>,
    },
}

impl ToolsetState {
    /// Creates a `ToolsetState::Configured` from a toolset config
    pub fn configured(config: ToolsetConfig) -> Self {
        let engine_options = config.to_engine_options();
        let mod_manager = ModManager::new_unchecked(&engine_options);
        ToolsetState::Configured {
            config,
            schema_manager: SchemaManager::default(),
            mod_manager,
            opened_mod: None,
        }
    }

    /// Creates a `ToolsetState::NotConfigured` from a partial toolset config
    pub fn not_configured(config: PartialToolsetConfig) -> Self {
        return ToolsetState::NotConfigured { config };
    }

    pub fn try_selected_mod(&self) -> Result<&OpenedMod> {
        match self {
            ToolsetState::Configured {
                opened_mod: Some(opened_mod),
                ..
            } => Ok(opened_mod),
            _ => Err(anyhow!("no mod selected")),
        }
    }

    pub fn try_schema_manager(&self) -> Result<&SchemaManager> {
        match self {
            ToolsetState::Configured { schema_manager, .. } => Ok(schema_manager),
            _ => Err(anyhow!("schema manager not initialized")),
        }
    }

    pub fn try_mod_manager(&self) -> Result<&ModManager> {
        match self {
            ToolsetState::Configured { mod_manager, .. } => Ok(mod_manager),
            _ => Err(anyhow!("mod manager not initialized")),
        }
    }

    pub fn try_config(&self) -> Result<&ToolsetConfig> {
        match self {
            ToolsetState::Configured { config, .. } => Ok(config),
            _ => Err(anyhow!("config not initialized")),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<ToolsetState>>,
}

impl AppState {
    pub fn new(inner: ToolsetState) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, ToolsetState> {
        self.inner.read().expect("state poisoned")
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, ToolsetState> {
        self.inner.write().expect("state poisoned")
    }
}

impl neon::types::Finalize for AppState {}
