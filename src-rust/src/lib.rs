use std::ops::Deref;

use invokables::{toolset_config::SerializableToolsetConfig, mods::SelectedMod, json::{OpenFileOptions, PersistFileOptions}, images::ReadImageFileParams, resources::ListResourcesParams, sounds::ReadSoundParams};
use neon::prelude::*;
use serde::{Deserialize, Serialize};
use simplelog::*;

mod config;
mod error;
mod invokables;
mod state;

pub(crate) use error::{Error, Result};
use state::AppState;

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
    let payload: Payload = serde_json::from_str(payload)?;
    let result = match payload.func.as_str() {
        "get_toolset_config" => {
            let config = invokables::toolset_config::get_toolset_config(&state)?;
            Ok(serde_json::to_value(config)?)
        }
        "set_toolset_config" => {
            let config: SerializableToolsetConfig = serde_json::from_value(payload.params)?;
            let config = invokables::toolset_config::set_toolset_config(&state, config)?;
            Ok(serde_json::to_value(config)?)
        }
        "get_available_mods" => {
          let config = invokables::mods::get_available_mods(&state)?;
          Ok(serde_json::to_value(config)?)
        },
        "get_editable_mods" => {
          let config = invokables::mods::get_editable_mods(&state)?;
          Ok(serde_json::to_value(config)?)
        },
        "set_selected_mod" => {
          let selected_mod: SelectedMod = serde_json::from_value(payload.params)?;
          invokables::mods::set_selected_mod(&state, selected_mod)?;
          Ok(serde_json::to_value("success")?)
        },
        "open_json_file_with_schema" => {
          let file: OpenFileOptions = serde_json::from_value(payload.params)?;
          let json = invokables::json::open_json_file_with_schema(&state, file)?;
          Ok(serde_json::to_value(json)?)
        },
        "persist_json_file" => {
          let file: PersistFileOptions = serde_json::from_value(payload.params)?;
          invokables::json::persist_json_file(&state, file)?;
          Ok(serde_json::to_value("success")?)
        },
        "read_image_file" => {
          let params: ReadImageFileParams = serde_json::from_value(payload.params)?;
          let image = invokables::images::read_image_file(state, params)?;
          Ok(serde_json::to_value(image)?)

        },
        "list_resources" => {
          let params: ListResourcesParams = serde_json::from_value(payload.params)?;
          let dir_entries = invokables::resources::list_resources(state, params)?;
          Ok(serde_json::to_value(dir_entries)?)
        },
        "read_sound" => {
          let params: ReadSoundParams = serde_json::from_value(payload.params)?;
          let dir_entries = invokables::sounds::read_sound(state, params)?;
          Ok(serde_json::to_value(dir_entries)?)
        },
        _ => Err(Error::new("unknown function to invoke")),
    }?;
    Ok(serde_json::to_string(&result)?)
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
