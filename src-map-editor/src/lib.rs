//! JA2 tactical map viewer, compiled to wasm and embedded in the toolset renderer
//!
//! The toolset starts the Bevy app once per page with [`start`] and keeps it alive. Maps are
//! switched with [`load_map`] or [`new_map`], because winit can only create its event loop once
//! per page.

// Bevy systems commonly take many parameters and complex queries
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};

use bevy::{
    asset::{embedded_asset, AssetMetaCheck},
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,
    winit::{UpdateMode, WinitSettings},
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use editor::{
    commands::{DoorRequest, EditorCommand},
    item_events::{ItemCommand, ItemEdit},
    light_events::{LightCommand, LightEdit},
    mode::EditorMode,
    soldier_events::{SoldierCommand, SoldierEdit},
    traversal_events::TraversalEdit,
    EditorPlugin,
};
use file_formats::{ja2_tileset::Ja2Tilesets, Ja2AssetsPlugin};
use host::{HostKey, HostPlugin};
use map::EditorSmoothing;
use render::RenderOptions;
use states::{AppState, DebugOverlay, LoadingPhase, UiFont};

pub mod components;
pub mod editor;
pub mod file_formats;
mod host;
pub mod item;
pub mod light;
pub mod map;
pub mod random;
pub mod render;
pub mod soldier;
mod states;
mod systems;
pub mod tileset;

#[cfg(all(feature = "bench", not(target_arch = "wasm32")))]
pub mod bench;

/// Map that should be loaded next, set by [`load_map`] or [`new_map`] and picked up by the app
static REQUESTED_MAP: Mutex<Option<MapRequest>> = Mutex::new(None);
/// Render options that should be applied next, set by [`set_render_options`]
static REQUESTED_RENDER_OPTIONS: Mutex<Option<RenderOptions>> = Mutex::new(None);
/// Editor mode that should be used next, set by [`set_editor_mode`]
static REQUESTED_EDITOR_MODE: Mutex<Option<EditorMode>> = Mutex::new(None);
/// Editor commands in the order they were requested, e.g. by [`undo`]
static EDITOR_COMMANDS: Mutex<Vec<EditorCommand>> = Mutex::new(Vec::new());
/// Keyboard input forwarded by the host, see [`key_event`]
static HOST_KEYS: Mutex<Vec<HostKey>> = Mutex::new(Vec::new());
/// Whether the viewer is currently shown, set by [`set_active`]
static ACTIVE: AtomicBool = AtomicBool::new(true);

/// A map that the host asked for
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MapRequest {
    /// The map file at the path, relative to the data root
    File(String),
    /// A new map that is saved at the path, see [`new_map`]
    New {
        path: String,
        options: NewMapOptions,
    },
}

impl MapRequest {
    /// Path of the map, relative to the data root
    pub(crate) fn path(&self) -> &str {
        match self {
            MapRequest::File(path) | MapRequest::New { path, .. } => path,
        }
    }
}

/// What a new map looks like, see [`new_map`]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct NewMapOptions {
    pub tileset: u8,
    pub smoothing: EditorSmoothing,
}

/// How often the app updates while the viewer is not shown
const INACTIVE_UPDATE_INTERVAL: Duration = Duration::from_secs(1);

/// Background color behind the map
pub(crate) const CLEAR_COLOR: Color = Color::linear_rgb(0.34, 0.36, 0.52);

/// Starts the viewer on the canvas matching `canvas_selector`
///
/// `read_file(path: string): Promise<Uint8Array | null>` reads a file relative to the data root,
/// `null` if it does not exist.
/// `files_exist(paths: string[]): Promise<boolean[]>` checks which files exist.
/// `on_status(status: { state: 'loading' | 'loaded' | 'error', map: string, message?: string })`
/// is called when a map starts loading, finishes loading or fails to load. Maps are loaded with
/// [`load_map`] or created with [`new_map`].
/// `on_editor_event(event)` is called with the palettes once a map is loaded
/// (`{ type: 'palette', map, tileset, ground, roads, debris, trees, rocks, misc, buildings }`,
/// `buildings` has the palettes of the buildings mode) and whenever the state of the editor
/// changes (`{ type: 'state', map, canUndo, undoDepth, canRedo, redoDepth, background,
/// nextRoomNumber, smoothing, restrictedScroll, revision }`, `smoothing` is `normal`, `basement` or
/// `caves`, `restrictedScroll` whether the game only scrolls over the middle of the map, `revision` counts the changes of the
/// map since it was loaded, see [`save_map`]). Clicking a door with the door locks tool
/// asks to edit it (`{ type: 'door', map, gridNo, door }`, `door` like in [`set_door`] or `null`
/// for a door without lock). The soldiers mode gets its choices once a map is loaded (`{ type:
/// 'soldierPalette', map, maxSoldiers, maxTeamSoldiers, enemyClasses, bodies, civilianGroups,
/// colours }`) and the selected soldier whenever it changes (`{ type: 'soldier', map, soldier,
/// counts, schedule, pick }`, `soldier` is `null` if none is
/// selected, see `SoldierValue`, `schedule` has the 4 actions of its schedule and `pick` the tile
/// of the schedule that the next click picks). The items mode gets the selected tile whenever it,
/// its items or the items of the map change (`{ type: 'item', map, tile, items, counts }`, `tile`
/// is `null` if none is selected, `items` are the items shown on it in the order of the map, see
/// `ItemValue`, `counts` has the number of placed items by item
/// (`items: [{ item, placed, quantity }]`), by trigger slot (`triggers`) and by lock (`keys:
/// [{ lockId, placed }]`)). The lights mode gets the selected light and the lighting of the map
/// whenever they change (`{ type: 'light', map, light, count, color, ambient, radii }`, `light` is
/// `null` if none is selected or `{ id, gridNo, x, y, radius, template, time }`, `radius` is
/// `null` for templates that are not `L-R0N.LHT`, `time` is `always`, `night` or `primeTime`,
/// `color` is `[r, g, b]`, `ambient` is `{ basement, caves, level }` for basements and caves and
/// `null` otherwise, with the light `level` of the original editor (1 to 15, higher is brighter),
/// and `radii` are the radii whose templates exist). The traversal mode gets the exit grids, the
/// entry points, the selection and the edge points whenever they change (`{ type: 'traversal',
/// map, exitGrids, selected, queried, entryPoints, edgePoints }`, `exitGrids` is their number,
/// `selected` is `null`, `{ kind: 'exitGrid', gridNo, destSector, destGridNo }` or `{ kind:
/// 'entry', point, gridNo }`, `queried` is the exit grid that was right clicked last, `null` or `{ gridNo, destSector: { x, y, z }, destGridNo }`,
/// `entryPoints` is `{ north, east, south, west, center, isolated }` with grid numbers or `null`,
/// `edgePoints` is `{ primary, secondary, missing, pending }`, the numbers of edge points of each
/// side (`{ north, east, south, west }`), the sides with an entry point but no edge points and
/// whether they are being generated again). Edits that can't be done are reported (`{ type:
/// 'notice', map, message }`).
///
/// This must only be called once per page.
#[wasm_bindgen]
pub fn start(
    canvas_selector: String,
    read_file: js_sys::Function,
    files_exist: js_sys::Function,
    on_status: js_sys::Function,
    on_editor_event: js_sys::Function,
) {
    host::set_callbacks(read_file, files_exist, on_status, on_editor_event);

    let mut app = App::new();
    build_app(
        &mut app,
        Window {
            title: "Ja2 Map Viewer".into(),
            canvas: Some(canvas_selector),
            fit_canvas_to_parent: true,
            // Keep keyboard input working in the rest of the page
            prevent_default_event_handling: false,
            ..default()
        },
        true,
    );
    // Returns immediately on wasm, the event loop is driven by the browser
    app.run();
}

/// Adds the plugins, states and resources of the viewer to `app`
///
/// Without the `host`, maps can't be loaded from files and nothing is reported to the host. The
/// map resources then have to be inserted by the caller, like the benchmark does.
fn build_app(app: &mut App, window: Window, host: bool) {
    if host {
        // Asset sources have to be registered before the `AssetPlugin`
        app.add_plugins(HostPlugin);
    }
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(window),
                ..default()
            })
            .set(AssetPlugin {
                // None of the JA2 assets have .meta files, don't look for them
                meta_check: AssetMetaCheck::Never,
                ..default()
            }),
    )
    .add_plugins(FrameTimeDiagnosticsPlugin::default())
    .add_plugins(embed_shaders)
    .insert_resource(ClearColor(CLEAR_COLOR))
    .insert_resource(WinitSettings::continuous())
    .init_resource::<UiFont>()
    .init_state::<AppState>()
    .add_sub_state::<LoadingPhase>()
    .add_sub_state::<DebugOverlay>()
    .add_plugins((
        Ja2AssetsPlugin,
        tileset::TilesetPlugin,
        item::ItemPlugin,
        light::LightPlugin,
    ));
    if host {
        app.add_plugins(states::loading::LoadingPlugin);
    }
    app.add_plugins((
        states::map::MapPlugin,
        states::tile_debug::TileDebugPlugin,
        render::RenderPlugin,
        soldier::SoldierPlugin,
        systems::camera::CameraPlugin,
        systems::pixel_view::PixelViewPlugin,
        systems::view_replay::ViewReplayPlugin,
        systems::radar::RadarPlugin,
        systems::cursor::CursorPlugin,
        EditorPlugin,
    ))
    .add_plugins(systems::configure_map_systems)
    .add_systems(Update, apply_active);
}

/// Requests loading the map at `path`, relative to the data root (e.g. `maps/A9.dat`)
#[wasm_bindgen]
pub fn load_map(path: String) {
    request_map(MapRequest::File(path));
}

/// Requests a new map that is saved at `path`, relative to the data root (e.g. `maps/A9.dat`)
///
/// `options` is JSON like `{ "tileset": 0, "smoothing": "normal" }`, `smoothing` is `normal`,
/// `basement` or `caves`. Like the original editor, every tile gets a random texture, basements
/// are roofed everywhere and caves are rock everywhere. The map is not read from `path` and not
/// written anywhere until it is saved with [`save_map`]. It is reported like a loaded map, a
/// tileset that does not exist is reported as an error.
#[wasm_bindgen]
pub fn new_map(path: String, options: String) -> Result<(), JsError> {
    let options: NewMapOptions = serde_json::from_str(&options)?;
    request_map(MapRequest::New { path, options });
    Ok(())
}

fn request_map(request: MapRequest) {
    *REQUESTED_MAP.lock().unwrap_or_else(|e| e.into_inner()) = Some(request);
}

/// A tileset of [`TilesetList`]
#[derive(Serialize)]
struct TilesetEntry {
    id: u8,
    name: String,
}

/// What [`tilesets`] resolves with
#[derive(Serialize)]
struct TilesetList {
    default: u8,
    tilesets: Vec<TilesetEntry>,
}

/// Lists the tilesets of `binarydata/ja2set.dat`, e.g. for choosing the tileset of a new map
///
/// Resolves with `{ default, tilesets: [{ id, name }] }`, `default` is the id of the tileset that
/// the others fall back to (`GENERIC_1`, or the default tileset of JA2.5 data). Rejects if the
/// file can't be read. Only needs [`start`] to have been called, not a loaded map.
#[wasm_bindgen]
pub fn tilesets() -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async {
        let path = std::path::Path::new("binarydata/ja2set.dat");
        let bytes = host::read_from_host(path)
            .await
            .map_err(|e| JsError::new(&format!("Error reading the tilesets: {}", e)))?;
        let tilesets = Ja2Tilesets::parse(&bytes)
            .map_err(|e| JsError::new(&format!("Error reading the tilesets: {}", e)))?;
        let list = TilesetList {
            default: tilesets.default_tileset_id().number(),
            tilesets: tilesets
                .ids()
                .filter_map(|id| {
                    Some(TilesetEntry {
                        id: id.number(),
                        name: tilesets.get(id.index())?.name.clone(),
                    })
                })
                .collect(),
        };
        let json = serde_json::to_string(&list).map_err(|e| JsError::new(&e.to_string()))?;
        js_sys::JSON::parse(&json)
    })
}

/// Sets what the viewer shows
///
/// `options` is JSON like `{ "layers": { "terrain": true, "objects": true, "buildings": true,
/// "items": true, "soldiers": true, "soldierTeams": { "enemy": true, "creature": true, "rebel":
/// true, "civilian": true }, "roomNumbers": false }, "floor": "ground", "lighting": "day" }`,
/// `lighting` is `day` or `night`. `terrain`, `objects` and `buildings` show what the editing mode
/// of that name places, with the shadows (the floors with the `terrain`), `roomNumbers` shows the
/// room numbers of the tiles. The soldiers of teams hidden in `soldierTeams` are neither drawn
/// nor selected or erased by the soldiers mode.
/// `floor` is `ground` or `roof`: the roofs, what is on them and the topmost tiles are only drawn
/// on the roof, and only soldiers and items on the floor can be selected, placed, moved or
/// erased. The terrain, objects, lights and traversal modes only edit on the ground. Page
/// Up and Page Down switch the floor, which is reported (`{ type: 'floor', map, floor }`).
/// Missing values keep their defaults.
#[wasm_bindgen]
pub fn set_render_options(options: String) -> Result<(), JsError> {
    let options: RenderOptions = serde_json::from_str(&options)?;
    *REQUESTED_RENDER_OPTIONS
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(options);
    Ok(())
}

/// Sets the editing mode, the latest one wins
///
/// `mode` is JSON like `{ "mode": "view" }`, `{ "mode": "terrain", "tool": "ground", "erase":
/// false, "brush": { "kind": "square", "size": 3 }, "density": 20, "ground": { "selected":
/// "TEXTURE1", "weights": [{ "typeName": "TEXTURE2", "weight": 1 }] } }`, `{ "mode": "terrain",
/// "tool": "roads", "erase": false, "road": 0 }`, `{ "mode": "terrain", "tool": "cliffs",
/// "erase": false, "cliff": 1 }` (roads and cliffs ignore the brush, `cliff` is the sub index of
/// the `FIRSTCLIFF` piece, the height of the land is worked out again after every cliff edit),
/// `{ "mode": "objects",
/// "erase": false, "brush": { "kind": "square", "size": 3 }, "density": 20, "objects":
/// [{ "tileId": 1234, "count": 1 }] }` or `{ "mode": "buildings", "tool": "room", "erase": false,
/// "brush": { "kind": "area" }, "selections": { "room": [{ "tileId": 1234, "count": 1 }],
/// "tiles": [] }, "smart": { "kind": "tile", "tileId": 1237 } }`.
///
/// The buildings `tool` is `tiles`,
/// which places single tiles of the `tiles` selection (of the walls, doors, windows, roofs,
/// broken walls, decals and floors palettes) in the layers of their pieces and when erasing
/// clears the layers of the selected kinds, `smart`, which
/// places what the `smart` value says (a tile of the smart palettes, whose piece tells whether it
/// is a wall, door, window, broken wall or roof, or `{ "kind": "hole" }`; roofs replace the
/// roof of the clicked building) and when erasing restores doors, windows and broken walls and
/// removes plain walls, or `room`, which uses the `room` selection (and saws the room out of the
/// buildings when erasing), `select`, which selects the clicked building (Delete removes it, dragging moves
/// it and Ctrl+dragging, or Cmd+dragging on macOS, copies it), or `roomNumber`, which gives the tiles the `roomNumber` of
/// the mode (0 is no room), `caves`, which fills the area with rock and carves caves with the
/// right button, or `doorLocks`, which asks the host to edit the clicked door.
///
/// `{ "mode": "items", "tool": "place", "erase": false, "brush": { "kind": "square", "size": 3 },
/// "choice": { "kind": "item", "item": 2 }, "trapLevel": 9, "action": 12 }`
/// places items with left clicks (on flat roofs on the roof floor), or erases the items of the
/// brush with `erase`. The `select`
/// tool selects the clicked tile with its items, dragging moves them, Ctrl+dragging (Cmd+dragging
/// on macOS) copies them and Delete removes them. The `choice` is `{ "kind": "item", "item": 2 }`,
/// `{ "kind": "trigger", "slot": 3 }` (even slots below 26 are triggers, the others actions, 26 is
/// the pressure action) or `{ "kind": "key", "lockId": 4, "item": 275 }` (the key item of the
/// lock), `null` places nothing. New items always exist. `trapLevel` and `action` (an
/// `ACTIONITEM_*` index of the original editor, 0 to 33) are those of new action items.
///
/// `{ "mode": "soldiers", "tool": "place", "erase": false, "brush": { "kind": "square", "size":
/// 3 }, "team": "enemy", "defaults": { "orders": "stationary", ... } }` places a soldier of the
/// `team` with the `defaults` with left clicks (on flat roofs on the roof floor, see
/// [`set_render_options`]), or with `erase` removes the soldiers of the teams shown by the
/// `soldierTeams` layer on the tiles of the brush. The `tool` `select` selects the clicked soldier (clicking it again selects
/// the next soldier of the tile), dragging moves it and Ctrl+dragging (Cmd+dragging on macOS)
/// copies it, both keep its level. Delete removes it, Esc deselects it and 1 to 9 set its patrol
/// points at the cursor (Shift removes them).
///
/// `{ "mode": "lights", "tool": "place", "erase": false, "brush": { "kind": "square", "size": 3 },
/// "radius": 5, "time": "primeTime" }` places lights with left clicks (replacing the lights of
/// the tile), or with `erase` removes the lights of the tiles of the brush. `radius` (1 to 8)
/// picks the template `L-R0N.LHT`, `time` is `always`, `night` or `primeTime`. The `tool`
/// `select` selects the clicked light (clicking it again selects the next light of the tile),
/// dragging moves it and Ctrl+dragging (Cmd+dragging on macOS) copies it, Delete removes it and
/// Esc deselects it.
///
/// `{ "mode": "traversal", "tool": "place", "place": "exit", "erase": false, "brush": { "kind":
/// "square", "size": 3 }, "destSector": { "x": 11, "y": 10, "z": 1 }, "destGridNo": 12345,
/// "entry": "north" }` paints exit grids to `destSector` (`x` is the column 1 to 16, `y` the row
/// 1 (A) to 16 (P), `z` the level 0 to 3) and `destGridNo` with the brush (`square` sizes or
/// `area`), replacing the exit grids of the tiles, or removes them with `erase`. A right click
/// reports the clicked exit grid (`queried` of the `traversal` event). The `place` `entry` sets
/// the `entry` point (`north`, `east`, `south`, `west`, `center` or `isolated`) with a left
/// click. The `tool` `select` selects the entry point or exit grid of the clicked tile (clicking
/// it again selects the next one of the tile), dragging moves it and Ctrl+dragging (Cmd+dragging
/// on macOS) copies exit grids, Delete removes exit grids and clears entry points and Esc
/// deselects.
///
/// The brush `kind` is `square` or `circle` (with a `size`), `line` (with a `width`), `area` or
/// `fill` (only for the terrain). The buildings mode restricts the brush like the original
/// editor: most tools use the clicked tile.
#[wasm_bindgen]
pub fn set_editor_mode(mode: String) -> Result<(), JsError> {
    let mode: EditorMode = serde_json::from_str(&mode)?;
    *REQUESTED_EDITOR_MODE
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(mode);
    Ok(())
}

/// Undoes the last edit of the loaded map
#[wasm_bindgen]
pub fn undo() {
    push_editor_command(EditorCommand::Undo);
}

/// Redoes the last undone edit of the loaded map, until the map is edited again
#[wasm_bindgen]
pub fn redo() {
    push_editor_command(EditorCommand::Redo);
}

/// Replaces the ground of the whole map by a texture, the name of the tile type of a texture of
/// the ground palette (e.g. `TEXTURE1`). Other tile types are ignored.
#[wasm_bindgen]
pub fn set_map_background(type_name: String) {
    push_editor_command(EditorCommand::SetBackground(type_name));
}

/// Sets how the editor smooths the loaded map, `normal`, `basement` or `caves`, which decides
/// what the building tools do. Can be undone.
#[wasm_bindgen]
pub fn set_map_smoothing(smoothing: String) -> Result<(), JsError> {
    let smoothing: EditorSmoothing = serde_json::from_value(serde_json::Value::String(smoothing))?;
    push_editor_command(EditorCommand::SetSmoothing(smoothing));
    Ok(())
}

/// Sets whether the game only scrolls over the middle of the loaded map, like basements do
/// (`ubRestrictedScrollID` 1 instead of 0). Can be undone.
#[wasm_bindgen]
pub fn set_restricted_scroll(restricted_scroll: bool) {
    push_editor_command(EditorCommand::SetRestrictedScroll(restricted_scroll));
}

/// Adds, replaces or removes the lock and trap of a door, which can be undone
///
/// `door` is JSON like `{ "gridNo": 12345, "door": { "lockId": 3, "trap": { "kind": "siren",
/// "level": 2 }, "locked": true } }`, the trap `kind` is `explosion`, `electric`, `siren`,
/// `silentAlarm`, `brothelSiren` or `superElectric`. The trap and the door can be `null`, a door
/// that is `null` is removed from the door table. The lock is at most 63, the trap level 1 to 20.
#[wasm_bindgen]
pub fn set_door(door: String) -> Result<(), JsError> {
    let request: DoorRequest = serde_json::from_str(&door)?;
    push_editor_command(EditorCommand::SetDoor(request));
    Ok(())
}

/// Gives every tile with a room a flat roof of the roofs selection of the buildings mode, the
/// ceilings of the rooms of basements. Can be undone.
#[wasm_bindgen]
pub fn roof_all_rooms() {
    push_editor_command(EditorCommand::RoofAllRooms);
}

/// Edits the selected soldier, which can be undone
///
/// `soldier` is JSON like the `soldier` of the `soldier` events: its `id` and the values it
/// edits (`orders`, `attitude`, `direction`, `relativeAttributeLevel`, `relativeEquipmentLevel`,
/// `soldierClass`, `civilianGroup`, `body`, `priorityExistence`, `hasKeys` and `detailed`, see
/// `SoldierFields`). Values that are out of range are clamped, unknown classes and body types
/// are ignored, the `detailed` values only if the soldier has a detailed placement.
#[wasm_bindgen]
pub fn set_soldier(soldier: String) -> Result<(), JsError> {
    let edit: SoldierEdit = serde_json::from_str(&soldier)?;
    push_editor_command(EditorCommand::UpdateSoldier(Box::new(edit)));
    Ok(())
}

/// Asks something of the soldiers, see `SoldierCommand`
///
/// `command` is JSON like `{ "kind": "select", "id": 3 }` (`null` deselects), `{ "kind":
/// "detailed", "detailed": true }` (gives the selected soldier a detailed placement or removes
/// it), `{ "kind": "center", "gridNo": 12345 }`
/// (centers the view on the tile), `{ "kind": "schedule", "rows": [...] }` (sets the 4 actions of
/// the schedule of the selected soldier, like the `schedule` of the `soldier` events), `{ "kind":
/// "clearSchedule" }`, `{ "kind": "pick", "slot": 0, "point": "a", "single": false }` (the next
/// left click picks tile A or B of an action, after A of a door action B is picked unless
/// `single` is set) or `{ "kind": "cancelPick" }`.
#[wasm_bindgen]
pub fn soldier_command(command: String) -> Result<(), JsError> {
    let command: SoldierCommand = serde_json::from_str(&command)?;
    push_editor_command(EditorCommand::Soldier(command));
    Ok(())
}

/// Edits a world item, e.g. one of the selected tile, which can be undone
///
/// `item` is JSON like the `item` of the `item` events: its `id`, `stack` (the item stack, its
/// item can't be changed), `visibility` (`hiddenItem`, `buried`, `hiddenInObject`, `invisible`,
/// `unseen` or `visible`), `flags` and `nonExistChance`. Values that are out of range are
/// clamped like the original editor does, see `EditContext::change_item`.
#[wasm_bindgen]
pub fn set_item(item: String) -> Result<(), JsError> {
    let edit: ItemEdit = serde_json::from_str(&item)?;
    push_editor_command(EditorCommand::UpdateItem(Box::new(edit)));
    Ok(())
}

/// Asks something of the world items, see `ItemCommand`
///
/// `command` is JSON like `{ "kind": "delete", "id": 3 }` (removes an item, which can be undone)
/// or `{ "kind": "center" }` (centers the view on the selected tile).
#[wasm_bindgen]
pub fn item_command(command: String) -> Result<(), JsError> {
    let command: ItemCommand = serde_json::from_str(&command)?;
    push_editor_command(EditorCommand::Item(command));
    Ok(())
}

/// Edits the selected light, which can be undone
///
/// `light` is JSON like `{ "id": 3, "radius": 4, "time": "night" }`, a `radius` of `null` keeps
/// the template. A radius whose template doesn't exist is reported with a `notice`.
#[wasm_bindgen]
pub fn set_light(light: String) -> Result<(), JsError> {
    let edit: LightEdit = serde_json::from_str(&light)?;
    push_editor_command(EditorCommand::UpdateLight(edit));
    Ok(())
}

/// Asks something of the lights, see `LightCommand`
///
/// `command` is JSON like `{ "kind": "deselect" }` or `{ "kind": "settings", "color": [r, g, b],
/// "ambientLevel": 8 }` (sets the light colour and, for basements and caves, the light level of
/// the original editor, 1 to 15, which can be undone).
#[wasm_bindgen]
pub fn light_command(command: String) -> Result<(), JsError> {
    let command: LightCommand = serde_json::from_str(&command)?;
    push_editor_command(EditorCommand::Light(command));
    Ok(())
}

/// Edits the selected entry point or exit grid, which can be undone
///
/// `edit` is JSON like `{ "kind": "entry", "point": "north", "gridNo": 12345 }` (moved to where
/// the game can show it, like a click does) or `{ "kind": "exitGrid", "gridNo": 12345,
/// "destSector": { "x": 11, "y": 10, "z": 1 }, "destGridNo": 6000 }` (the destination of the exit
/// grid of the tile).
#[wasm_bindgen]
pub fn set_traversal(edit: String) -> Result<(), JsError> {
    let edit: TraversalEdit = serde_json::from_str(&edit)?;
    push_editor_command(EditorCommand::UpdateTraversal(edit));
    Ok(())
}

/// Saves the loaded map with the edits so far, in the version it was loaded in
///
/// Resolves with `{ map, revision, data }` once the edits requested before are applied: `map` is
/// the path of the map, `revision` the `revision` of the state events that the saved map has and
/// `data` the map file as `Uint8Array`. The map is not written anywhere, that is up to the host.
/// Edge points that edits made out of date are generated again first. Rejects if no map is
/// loaded or the map can't be saved.
#[wasm_bindgen]
pub fn save_map() -> js_sys::Promise {
    host::request_save()
}

/// Renders the radar map of `map` (e.g. `maps/A9.dat`), the small image of the map in the
/// strategic screen of the game, like the game's map utility does
///
/// Resolves with the radar map as an 88×44 8 bit STI file (`Uint8Array`) once `map` is loaded,
/// which is up to the host to write (to `radarmaps/<name of the map>.sti`). `map` must be the map
/// that was requested last with [`load_map`] or [`new_map`]. The loaded map is rendered with its edits so far.
/// Rejects if `map` can't be loaded or another map is loaded before the radar map is done.
#[wasm_bindgen]
pub fn render_radar_map(map: String) -> js_sys::Promise {
    host::request_radar(map)
}

fn push_editor_command(command: EditorCommand) {
    EDITOR_COMMANDS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(command);
}

/// Forwards a key that was pressed or released outside of the canvas
///
/// `code` and `key` are the `code` and `key` of the `KeyboardEvent`. The canvas only receives
/// keyboard input while it is focused, so the host forwards keys pressed elsewhere on the page.
/// Codes that are not known are ignored.
#[wasm_bindgen]
pub fn key_event(code: String, key: String, pressed: bool, repeat: bool) {
    let Some(key_code) = host::key_code(code) else {
        return;
    };
    push_host_key(HostKey::Key {
        key_code,
        key,
        pressed,
        repeat,
    });
}

/// Releases all keys, e.g. when the page loses the focus
#[wasm_bindgen]
pub fn release_keys() {
    push_host_key(HostKey::ReleaseAll);
}

fn push_host_key(key: HostKey) {
    HOST_KEYS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(key);
}

/// Throttles updates while the viewer is not shown
#[wasm_bindgen]
pub fn set_active(active: bool) {
    ACTIVE.store(active, Ordering::Relaxed);
}

/// Takes the map requested by [`load_map`] or [`new_map`], if any
pub(crate) fn take_requested_map() -> Option<MapRequest> {
    REQUESTED_MAP
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}

/// Whether a map was requested by [`load_map`] or [`new_map`] that is not switched to yet
pub(crate) fn map_requested() -> bool {
    REQUESTED_MAP
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
}

/// Takes the render options requested by [`set_render_options`], if any
pub(crate) fn take_requested_render_options() -> Option<RenderOptions> {
    REQUESTED_RENDER_OPTIONS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}

/// Takes the editor mode requested by [`set_editor_mode`], if any
pub(crate) fn take_requested_editor_mode() -> Option<EditorMode> {
    REQUESTED_EDITOR_MODE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}

/// Takes the editor commands requested since the last call
pub(crate) fn take_editor_commands() -> Vec<EditorCommand> {
    std::mem::take(&mut *EDITOR_COMMANDS.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Takes the keyboard input forwarded since the last call
pub(crate) fn take_host_keys() -> Vec<HostKey> {
    std::mem::take(&mut *HOST_KEYS.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Embeds the shaders, see `embedded_asset!` for the resulting paths
fn embed_shaders(app: &mut App) {
    embedded_asset!(app, "shaders/ja2_replay.wesl");
}

fn apply_active(mut winit_settings: ResMut<WinitSettings>) {
    // Radar maps are also rendered while the viewer is not shown
    let mode = if ACTIVE.load(Ordering::Relaxed) || host::radar_requested() {
        UpdateMode::Continuous
    } else {
        UpdateMode::reactive_low_power(INACTIVE_UPDATE_INTERVAL)
    };
    // On the web, the window is only focused while the canvas has keyboard focus. Keep updating
    // while the viewer is shown, so that it also reacts to the mouse without being clicked first.
    if winit_settings.focused_mode != mode {
        winit_settings.focused_mode = mode;
        winit_settings.unfocused_mode = mode;
    }
}
