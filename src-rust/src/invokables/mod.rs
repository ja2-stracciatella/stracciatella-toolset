use crate::state::AppState;
use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

mod images;
mod json;
mod mods;
mod resources;
mod sounds;
mod toolset_config;

pub trait Invokable: DeserializeOwned {
    type Output: Serialize;

    fn name() -> &'static str;
    fn validate(&self) -> Result<()> {
        Ok(())
    }
    fn invoke(&self, state: &AppState) -> Result<Self::Output>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InvokePayload {
    func: String,
    params: serde_json::Value,
}

pub struct Invokables {
    executable: HashMap<&'static str, Box<dyn Fn(&AppState, Value) -> Result<Value> + Send + Sync>>,
}

impl Invokables {
    pub fn new() -> Self {
        let mut new = Self {
            executable: HashMap::new(),
        };
        // All invokables must be registered here
        new.register::<images::ReadImageFile>();
        new.register::<json::OpenJsonWithSchema>();
        new.register::<json::PersistJson>();
        new.register::<mods::GetAvailableMods>();
        new.register::<mods::GetEditableMods>();
        new.register::<mods::SetSelectedMod>();
        new.register::<mods::NewMod>();
        new.register::<resources::ListResources>();
        new.register::<sounds::ReadSound>();
        new.register::<toolset_config::GetToolsetConfig>();
        new.register::<toolset_config::SetToolsetConfig>();

        new
    }

    fn register<I: Invokable>(&mut self) {
        self.executable.insert(
            I::name(),
            Box::new(|state, value| {
                let invokable: I = serde_json::from_value(value)
                    .with_context(|| format!("invalid payload for invokable {}", I::name()))?;

                invokable
                    .validate()
                    .with_context(|| format!("invalid payload for invokable {}", I::name()))?;

                let result = invokable
                    .invoke(state)
                    .with_context(|| format!("failed to invoke invokable {}", I::name()))?;
                let result = serde_json::to_value(result).with_context(|| {
                    format!("failed to serialize result for invokable {}", I::name())
                })?;

                Ok(result)
            }),
        );
    }

    pub fn invoke(&self, state: &AppState, payload: String) -> Result<String> {
        let payload: InvokePayload =
            serde_json::from_str(&payload).context("failed to deserialize payload")?;

        if let Some(func) = self.executable.get(payload.func.as_str()) {
            let result = func(state, payload.params.clone()).with_context(|| {
                format!(
                    "failed to invoke invokable {} with parameters {:?}",
                    &payload.func, &payload.params
                )
            })?;
            let result =
                serde_json::to_string(&result).context("failed to serialize invokable result")?;

            Ok(result)
        } else {
            Err(anyhow::anyhow!(
                "invokable {} not found",
                payload.func.as_str()
            ))
        }
    }
}
