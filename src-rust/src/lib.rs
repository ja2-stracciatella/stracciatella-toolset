use std::{ops::Deref, sync::LazyLock};

use invokables::Invokables;
use neon::prelude::*;
use simplelog::*;

mod config;
mod invokables;
mod state;

use state::AppState;

static INVOKABLES: LazyLock<Invokables> = LazyLock::new(|| Invokables::new());

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

fn invoke(mut ctx: FunctionContext) -> JsResult<JsPromise> {
    let state: AppState = ctx
        .argument::<JsBox<state::AppState>>(0)?
        .deref()
        .deref()
        .clone();
    let payload: String = ctx.argument::<JsString>(1)?.value(&mut ctx);
    let (deferred, promise) = ctx.promise();

    ctx.task(move || {
        let invokables = &*INVOKABLES;
        invokables.invoke(&state, payload)
    })
    .and_then(|mut ctx, result| {
        match result {
            Ok(v) => {
                let val = ctx.string(v);
                deferred.resolve(&mut ctx, val)
            }
            Err(e) => {
                let err = ctx.error(format!("{:#}", e))?;
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
