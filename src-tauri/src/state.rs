use async_lock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::Arc;

use stracciatella::{
    mods::{Mod, ModManager},
    schemas::SchemaManager,
    vfs::Vfs,
};

use crate::config::{self, PartialToolsetConfig, ToolsetConfig};
use crate::error::{Error, Result};

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
            .ok_or_else(|| Error::new(format!("failed to find mod `{}`", mod_id)))?
            .clone();
        let mut vfs = Vfs::new();
        let engine_options = config.to_engine_options();
        vfs.init(&engine_options, mod_manager)
            .map_err(|e| Error::new(format!("failed to initialize vfs: {}", e)))?;

        Ok(OpenedMod {
            vfs: Arc::new(vfs),
            m,
        })
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

    pub fn not_configured(config: PartialToolsetConfig) -> Self {
        return ToolsetState::NotConfigured { config };
    }
}

pub struct TauriState {
    inner: Arc<RwLock<ToolsetState>>,
}

impl TauriState {
    pub fn new(inner: ToolsetState) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, ToolsetState> {
        self.inner.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, ToolsetState> {
        self.inner.write().await
    }
}
