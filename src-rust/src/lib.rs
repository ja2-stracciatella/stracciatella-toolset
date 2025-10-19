use std::ops::Deref;

use invokables::{
    images::ReadImageFileParams,
    json::{OpenFileParams, PersistFileParams},
    mods::SelectedMod,
    resources::ListResourcesParams,
    sounds::ReadSoundParams,
    toolset_config::SerializableToolsetConfig,
};
use neon::prelude::*;
use serde::{Deserialize, Serialize};
use simplelog::*;

mod config;
mod error;
mod invokables;
mod state;

pub(crate) use error::{Error, Result};
use state::AppState;

use crate::invokables::mods::NewMod;

fn init_logger(mut ctx: FunctionContext) -> JsResult<JsUndefined> {
    if let Err(e) = TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    ) {
        println!("failed to initialize logger: {}", e)
    }
    Ok(JsUndefined::new(&mut ctx))
}

fn new_app_state(mut ctx: FunctionContext) -> JsResult<JsBox<state::AppState>> {
    if let Ok(toolset_config_path) = config::ToolsetConfig::path() {
        log::info!(
            "Toolset config path: `{}`",
            toolset_config_path.to_string_lossy()
        );
    }
    let initial_state = match config::ToolsetConfig::read() {
        Ok(config) => state::ToolsetState::configured(config),
        Err((e, partial_config)) => {
            log::error!("failed to read toolset config: {}", e);
            state::ToolsetState::not_configured(partial_config)
        }
    };
    let initial_state = state::AppState::new(initial_state);

    Ok(ctx.boxed(initial_state))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Payload {
    func: String,
    params: serde_json::Value,
}

fn invoke_inner(state: &state::AppState, payload: &str) -> Result<String> {
    fn invoke_payload(state: &state::AppState, payload: &Payload) -> Result<String> {
        let result = match payload.func.as_str() {
            "get_toolset_config" => {
                let config = invokables::toolset_config::get_toolset_config(&state)?;
                Ok(serde_json::to_value(config)?)
            }
            "set_toolset_config" => {
                let config: SerializableToolsetConfig =
                    serde_json::from_value(payload.params.clone())?;
                let config = invokables::toolset_config::set_toolset_config(&state, config)?;
                Ok(serde_json::to_value(config)?)
            }
            "get_available_mods" => {
                let config = invokables::mods::get_available_mods(&state)?;
                Ok(serde_json::to_value(config)?)
            }
            "get_editable_mods" => {
                let config = invokables::mods::get_editable_mods(&state)?;
                Ok(serde_json::to_value(config)?)
            }
            "set_selected_mod" => {
                let selected_mod: SelectedMod = serde_json::from_value(payload.params.clone())?;
                let s = invokables::mods::set_selected_mod(&state, selected_mod)?;
                Ok(serde_json::to_value(s)?)
            }
            "create_new_mod" => {
                let new_mod: NewMod = serde_json::from_value(payload.params.clone())?;
                let s = invokables::mods::create_new_mod(&state, new_mod)?;
                Ok(serde_json::to_value(s)?)
            }
            "open_json_with_schema" => {
                let file: OpenFileParams = serde_json::from_value(payload.params.clone())?;
                let json = invokables::json::open_json_with_schema(&state, file)?;
                Ok(serde_json::to_value(json)?)
            }
            "persist_json" => {
                let file: PersistFileParams = serde_json::from_value(payload.params.clone())?;
                let json = invokables::json::persist_json(&state, file)?;
                Ok(serde_json::to_value(json)?)
            }
            "read_image_file" => {
                let params: ReadImageFileParams = serde_json::from_value(payload.params.clone())?;
                let image = invokables::images::read_image_file(state, params)?;
                Ok(serde_json::to_value(image)?)
            }
            "list_resources" => {
                let params: ListResourcesParams = serde_json::from_value(payload.params.clone())?;
                let dir_entries = invokables::resources::list_resources(state, params)?;
                Ok(serde_json::to_value(dir_entries)?)
            }
            "read_sound" => {
                let params: ReadSoundParams = serde_json::from_value(payload.params.clone())?;
                let dir_entries = invokables::sounds::read_sound(state, params)?;
                Ok(serde_json::to_value(dir_entries)?)
            }
            _ => Err(Error::new("unknown function to invoke")),
        }?;
        Ok(serde_json::to_string(&result)?)
    }

    let payload: Payload = serde_json::from_str(payload)?;
    invoke_payload(state, &payload).map_err(|e| {
        Error::new(format!(
            "failed to invoke {} with params {}: {}",
            payload.func, payload.params, e
        ))
    })
}

fn invoke(mut ctx: FunctionContext) -> JsResult<JsPromise> {
    let state: AppState = ctx
        .argument::<JsBox<state::AppState>>(0)?
        .deref()
        .deref()
        .clone();
    let payload: String = ctx.argument::<JsString>(1)?.value(&mut ctx);
    let (deferred, promise) = ctx.promise();

    ctx.task(move || invoke_inner(&state, &payload))
        .and_then(|mut ctx, result| {
            match result {
                Ok(v) => {
                    let val = ctx.string(v);
                    deferred.resolve(&mut ctx, val)
                }
                Err(e) => {
                    let err = ctx.error(e.to_string())?;
                    deferred.reject(&mut ctx, err)
                }
            };

            Ok(())
        });

    Ok(promise)
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("initLogger", init_logger)?;
    cx.export_function("newAppState", new_app_state)?;
    cx.export_function("invoke", invoke)?;
    Ok(())
}
