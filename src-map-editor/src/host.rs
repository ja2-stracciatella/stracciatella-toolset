//! Communication with the JS host, i.e. the toolset renderer
//!
//! The host reads files (mod files layered over the vanilla data) and is told about the loading
//! status of maps and about the editor. It also forwards keyboard input from outside the canvas.
//! The JS callbacks are not `Send`, so they are kept in a thread local. This is fine because wasm
//! is single threaded.

use std::{
    cell::RefCell,
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use bevy::{
    asset::io::{AssetReader, AssetReaderError, AssetSourceBuilder, PathStream, Reader, VecReader},
    input::{
        keyboard::{Key, KeyboardFocusLost, KeyboardInput, NativeKey},
        ButtonState, InputSystems,
    },
    prelude::*,
    reflect::{
        enums::{DynamicEnum, DynamicVariant, VariantInfo},
        TypeInfo, Typed,
    },
    tasks::futures_lite::stream,
    window::PrimaryWindow,
};
use wasm_bindgen::JsValue;

/// Name of the asset source that reads from the host, e.g. `ja2://maps/B13.DAT`
pub const JA2_ASSET_SOURCE: &str = "ja2";

struct Callbacks {
    read_file: js_sys::Function,
    files_exist: js_sys::Function,
    on_status: js_sys::Function,
    on_editor_event: js_sys::Function,
}

/// The functions settling the promise of a `save_map` call
struct PendingSave {
    resolve: js_sys::Function,
    reject: js_sys::Function,
}

/// The functions settling the promise of a `render_radar_map` call
struct PendingRadar {
    map: String,
    resolve: js_sys::Function,
    reject: js_sys::Function,
}

thread_local! {
    static CALLBACKS: RefCell<Option<Callbacks>> = const { RefCell::new(None) };
    static PENDING_SAVES: RefCell<Vec<PendingSave>> = const { RefCell::new(Vec::new()) };
    static PENDING_RADARS: RefCell<Vec<PendingRadar>> = const { RefCell::new(Vec::new()) };
}

pub fn set_callbacks(
    read_file: js_sys::Function,
    files_exist: js_sys::Function,
    on_status: js_sys::Function,
    on_editor_event: js_sys::Function,
) {
    CALLBACKS.with_borrow_mut(|callbacks| {
        *callbacks = Some(Callbacks {
            read_file,
            files_exist,
            on_status,
            on_editor_event,
        })
    });
}

/// Loading state of a map, reported to the host
#[derive(Clone, Copy, Debug)]
pub enum MapStatus {
    Loading,
    Loaded,
    Error,
}

impl MapStatus {
    fn as_str(&self) -> &'static str {
        match self {
            MapStatus::Loading => "loading",
            MapStatus::Loaded => "loaded",
            MapStatus::Error => "error",
        }
    }
}

/// Tells the host about the loading state of a map
pub fn report_status(status: MapStatus, map: &str, message: Option<&str>) {
    let value = js_sys::Object::new();
    // Setting properties on a plain object cannot fail
    let _ = js_sys::Reflect::set(&value, &"state".into(), &status.as_str().into());
    let _ = js_sys::Reflect::set(&value, &"map".into(), &map.into());
    if let Some(message) = message {
        let _ = js_sys::Reflect::set(&value, &"message".into(), &message.into());
    }
    CALLBACKS.with_borrow(|callbacks| {
        if let Some(callbacks) = callbacks {
            if let Err(e) = callbacks.on_status.call1(&JsValue::NULL, &value) {
                error!("Error reporting map status: {:?}", e);
            }
        }
    });
}

/// Tells the host about the editor, `event` is sent as a JSON object
pub fn report_editor_event(event: &impl serde::Serialize) {
    CALLBACKS.with_borrow(|callbacks| {
        let Some(callbacks) = callbacks else {
            return;
        };
        let value = match serde_json::to_string(event)
            .map_err(|e| e.to_string())
            .and_then(|json| js_sys::JSON::parse(&json).map_err(|e| format!("{:?}", e)))
        {
            Ok(value) => value,
            Err(e) => {
                error!("Error serializing editor event: {}", e);
                return;
            }
        };
        if let Err(e) = callbacks.on_editor_event.call1(&JsValue::NULL, &value) {
            error!("Error reporting editor event: {:?}", e);
        }
    });
}

/// A map written by the editor, see [`request_save`]
pub struct SavedMap {
    pub map: String,
    pub revision: u64,
    pub data: Vec<u8>,
}

/// A promise that is settled with the next [`finish_saves`]
pub fn request_save() -> js_sys::Promise {
    js_sys::Promise::new(&mut |resolve, reject| {
        PENDING_SAVES.with_borrow_mut(|saves| saves.push(PendingSave { resolve, reject }));
    })
}

/// Whether the host is waiting for the map to be saved
pub fn save_requested() -> bool {
    PENDING_SAVES.with_borrow(|saves| !saves.is_empty())
}

/// Resolves the promises of [`request_save`] with the saved map, or rejects them with the error
pub fn finish_saves(result: Result<SavedMap, String>) {
    let saves = PENDING_SAVES.with_borrow_mut(std::mem::take);
    let (settle, value): (fn(&PendingSave) -> &js_sys::Function, JsValue) = match result {
        Ok(saved) => {
            let value = js_sys::Object::new();
            // Setting properties on a plain object cannot fail
            let _ = js_sys::Reflect::set(&value, &"map".into(), &saved.map.into());
            // Revisions stay far below 2^53
            let _ =
                js_sys::Reflect::set(&value, &"revision".into(), &(saved.revision as f64).into());
            let data = js_sys::Uint8Array::from(saved.data.as_slice());
            let _ = js_sys::Reflect::set(&value, &"data".into(), &data);
            (|save| &save.resolve, value.into())
        }
        Err(message) => (|save| &save.reject, js_sys::Error::new(&message).into()),
    };
    for save in &saves {
        if let Err(e) = settle(save).call1(&JsValue::NULL, &value) {
            error!("Error settling the save promise: {:?}", e);
        }
    }
}

/// A promise that is settled with the radar map of `map` by [`finish_radars`]
pub fn request_radar(map: String) -> js_sys::Promise {
    js_sys::Promise::new(&mut |resolve, reject| {
        let map = map.clone();
        PENDING_RADARS.with_borrow_mut(|radars| {
            radars.push(PendingRadar {
                map,
                resolve,
                reject,
            })
        });
    })
}

/// Whether the host is waiting for a radar map
pub fn radar_requested() -> bool {
    PENDING_RADARS.with_borrow(|radars| !radars.is_empty())
}

/// The maps whose radar maps the host is waiting for, in the order of the requests
pub fn requested_radar_maps() -> Vec<String> {
    PENDING_RADARS.with_borrow(|radars| {
        let mut maps: Vec<String> = Vec::new();
        for radar in radars {
            if !maps.contains(&radar.map) {
                maps.push(radar.map.clone());
            }
        }
        maps
    })
}

/// Resolves the promises of [`request_radar`] for `map` with the STI file of its radar map, or
/// rejects them with the error
pub fn finish_radars(map: &str, result: Result<Vec<u8>, String>) {
    let radars = PENDING_RADARS.with_borrow_mut(|radars| {
        let (finished, waiting): (Vec<_>, Vec<_>) = std::mem::take(radars)
            .into_iter()
            .partition(|radar| radar.map == map);
        *radars = waiting;
        finished
    });
    let (settle, value): (fn(&PendingRadar) -> &js_sys::Function, JsValue) = match result {
        Ok(data) => (
            |radar| &radar.resolve,
            js_sys::Uint8Array::from(data.as_slice()).into(),
        ),
        Err(message) => (|radar| &radar.reject, js_sys::Error::new(&message).into()),
    };
    for radar in &radars {
        if let Err(e) = settle(radar).call1(&JsValue::NULL, &value) {
            error!("Error settling the radar map promise: {:?}", e);
        }
    }
}

/// Keyboard input forwarded by the host, see `crate::key_event`
#[derive(Debug)]
pub enum HostKey {
    Key {
        key_code: KeyCode,
        /// The `key` of the web keyboard event
        key: String,
        pressed: bool,
        repeat: bool,
    },
    ReleaseAll,
}

/// The key code of the `code` of a web keyboard event, `None` if it is not known
pub fn key_code(code: String) -> Option<KeyCode> {
    // Cmd and the Windows key, which winit maps the same way
    match code.as_str() {
        "MetaLeft" | "OSLeft" => return Some(KeyCode::SuperLeft),
        "MetaRight" | "OSRight" => return Some(KeyCode::SuperRight),
        _ => {}
    }
    // The other unit variants of `KeyCode` are named like the `code` values of web keyboard events
    let TypeInfo::Enum(info) = KeyCode::type_info() else {
        return None;
    };
    if !matches!(info.variant(&code)?, VariantInfo::Unit(_)) {
        return None;
    }
    KeyCode::from_reflect(&DynamicEnum::new(code, DynamicVariant::Unit))
}

/// Registers the host as the `ja2://` asset source, must be added before `AssetPlugin`
pub struct HostPlugin;

impl Plugin for HostPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_source(
            JA2_ASSET_SOURCE,
            AssetSourceBuilder::new(|| Box::new(HostAssetReader)),
        )
        .add_systems(PreUpdate, forward_host_keys.before(InputSystems));
    }
}

/// Writes the keys forwarded by the host as if the window received them
fn forward_host_keys(
    window: Query<Entity, With<PrimaryWindow>>,
    mut keyboard_input: MessageWriter<KeyboardInput>,
    mut focus_lost: MessageWriter<KeyboardFocusLost>,
) {
    let keys = crate::take_host_keys();
    let Ok(window) = window.single() else {
        return;
    };
    for key in keys {
        match key {
            HostKey::Key {
                key_code,
                key,
                pressed,
                repeat,
            } => {
                keyboard_input.write(KeyboardInput {
                    key_code,
                    logical_key: Key::Unidentified(NativeKey::Web(key.into())),
                    state: if pressed {
                        ButtonState::Pressed
                    } else {
                        ButtonState::Released
                    },
                    text: None,
                    repeat,
                    window,
                });
            }
            HostKey::ReleaseAll => {
                focus_lost.write(KeyboardFocusLost);
            }
        }
    }
}

/// Reads assets through the `read_file` callback of the host
struct HostAssetReader;

fn io_error(message: String) -> AssetReaderError {
    AssetReaderError::Io(Arc::new(io::Error::other(message)))
}

fn host_path(path: &Path) -> Result<String, AssetReaderError> {
    let path = path
        .to_str()
        .ok_or_else(|| io_error(format!("path `{}` is not valid unicode", path.display())))?;
    Ok(path.replace('\\', "/"))
}

#[cfg(target_arch = "wasm32")]
fn js_error_message(value: &JsValue) -> String {
    use wasm_bindgen::JsCast;

    value
        .dyn_ref::<js_sys::Error>()
        .map(|e| String::from(e.message()))
        .or_else(|| value.as_string())
        .unwrap_or_else(|| format!("{:?}", value))
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn read_from_host(path: &Path) -> Result<Vec<u8>, AssetReaderError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let path_str = host_path(path)?;
    let read_file = CALLBACKS
        .with_borrow(|callbacks| callbacks.as_ref().map(|c| c.read_file.clone()))
        .ok_or_else(|| io_error("host callbacks are not set".to_owned()))?;
    let promise = read_file
        .call1(&JsValue::NULL, &path_str.as_str().into())
        .map_err(|e| io_error(js_error_message(&e)))?
        .dyn_into::<js_sys::Promise>()
        .map_err(|_| io_error("read_file did not return a promise".to_owned()))?;
    let value = JsFuture::from(promise).await.map_err(|e| {
        io_error(format!(
            "Error reading `{}`: {}",
            path_str,
            js_error_message(&e)
        ))
    })?;
    if value.is_null() {
        return Err(AssetReaderError::NotFound(path.to_owned()));
    }
    let bytes = value
        .dyn_into::<js_sys::Uint8Array>()
        .map_err(|_| io_error("read_file did not resolve to a Uint8Array or null".to_owned()))?;
    Ok(bytes.to_vec())
}

/// JS futures are not `Send`, which native Bevy requires. Natively there is no host anyways.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn read_from_host(path: &Path) -> Result<Vec<u8>, AssetReaderError> {
    Err(io_error(format!(
        "cannot read `{}`, reading from the host is only supported on wasm",
        host_path(path)?
    )))
}

/// Asks the host which of the files exist, relative to the data root
#[cfg(target_arch = "wasm32")]
pub async fn files_exist(paths: Vec<String>) -> Result<Vec<bool>, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let files_exist = CALLBACKS
        .with_borrow(|callbacks| callbacks.as_ref().map(|c| c.files_exist.clone()))
        .ok_or_else(|| "host callbacks are not set".to_owned())?;
    let paths_array: js_sys::Array = paths.iter().map(|p| JsValue::from_str(p)).collect();
    let promise = files_exist
        .call1(&JsValue::NULL, &paths_array)
        .map_err(|e| js_error_message(&e))?
        .dyn_into::<js_sys::Promise>()
        .map_err(|_| "files_exist did not return a promise".to_owned())?;
    let value = JsFuture::from(promise)
        .await
        .map_err(|e| format!("Error checking which files exist: {}", js_error_message(&e)))?;
    let exists = value
        .dyn_into::<js_sys::Array>()
        .map_err(|_| "files_exist did not resolve to an array".to_owned())?
        .iter()
        .map(|v| {
            v.as_bool()
                .ok_or_else(|| "files_exist did not resolve to an array of booleans".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if exists.len() != paths.len() {
        return Err(format!(
            "files_exist returned {} results for {} files",
            exists.len(),
            paths.len()
        ));
    }
    Ok(exists)
}

/// See `read_from_host`, natively there is no host
#[cfg(not(target_arch = "wasm32"))]
pub async fn files_exist(_paths: Vec<String>) -> Result<Vec<bool>, String> {
    Err("checking whether files exist is only supported on wasm".to_owned())
}

impl AssetReader for HostAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        Ok(VecReader::new(read_from_host(path).await?))
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        // JA2 assets have no meta files
        Err::<VecReader, _>(AssetReaderError::NotFound(path.to_owned()))
    }

    async fn read_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        // Directories are never listed, all paths are known up front
        Ok(Box::new(stream::empty::<PathBuf>()))
    }

    async fn is_directory<'a>(&'a self, _path: &'a Path) -> Result<bool, AssetReaderError> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_code_should_map_web_codes() {
        assert_eq!(key_code("KeyW".to_owned()), Some(KeyCode::KeyW));
        assert_eq!(key_code("F3".to_owned()), Some(KeyCode::F3));
        assert_eq!(key_code("Equal".to_owned()), Some(KeyCode::Equal));
        assert_eq!(key_code("NumpadAdd".to_owned()), Some(KeyCode::NumpadAdd));
        assert_eq!(key_code("ArrowUp".to_owned()), Some(KeyCode::ArrowUp));
        assert_eq!(key_code("MetaLeft".to_owned()), Some(KeyCode::SuperLeft));
        assert_eq!(key_code("MetaRight".to_owned()), Some(KeyCode::SuperRight));
        assert_eq!(key_code("NotAKey".to_owned()), None);
        assert_eq!(key_code("Unidentified".to_owned()), None);
        assert_eq!(key_code(String::new()), None);
    }
}
