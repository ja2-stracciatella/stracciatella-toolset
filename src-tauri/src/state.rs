use std::sync::{Arc, Mutex};

use stracciatella::{mods::{ModManager, Mod}, vfs::Vfs, schemas::SchemaManager};

use crate::config::{self, ToolsetConfig};

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
  ) -> Result<OpenedMod, String> {
    let m = mod_manager
      .get_mod_by_id(&mod_id)
      .ok_or_else(|| format!("cannot open unknown mod: {}", mod_id))?
      .clone();
    let mut vfs = Vfs::new();
    let engine_options = config.to_engine_options();
    vfs
      .init(&engine_options, &mod_manager)
      .map_err(|e| format!("error initializing vfs: {}", e))?;

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
    opened_mod: Arc<Mutex<Option<OpenedMod>>>,
  },
}

impl ToolsetState {
  pub fn configure(config: ToolsetConfig) -> Self {
    let engine_options = config.to_engine_options();
    let mod_manager = ModManager::new_unchecked(&engine_options, &engine_options.assets_dir);
    ToolsetState::Configured {
      config: config.clone(),
      schema_manager: SchemaManager::default(),
      mod_manager,
      opened_mod: Arc::new(Mutex::new(None)),
    }
  }
}
