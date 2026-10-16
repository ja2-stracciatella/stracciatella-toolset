// Types of the wasm-bindgen glue of the tactical map viewer (src-map-editor/pkg), which is built by
// `npm run rebuild:map-editor`. It is resolved via a webpack alias, so that typechecking, linting
// and tests work without building it. Keep in sync with `src-map-editor/src/lib.rs`.
declare module 'ja2-map-editor' {
  export default function init(): Promise<unknown>;
  export function start(
    canvasSelector: string,
    // Resolves to null if the file does not exist
    readFile: (path: string) => Promise<Uint8Array | null>,
    // Whether each of the files exists, in the same order
    filesExist: (paths: string[]) => Promise<boolean[]>,
    onStatus: (status: unknown) => void,
    // Palettes and state of the editor, see `EditorEvent` in `mapViewer.ts`
    onEditorEvent: (event: unknown) => void,
  ): void;
  export function load_map(path: string): void;
  // JSON of `NewMapOptions` in `mapViewer.ts`, throws if it is invalid
  export function new_map(path: string, options: string): void;
  // Resolves with `{ default, tilesets: [{ id, name }] }`, rejects if the tilesets can't be read
  export function tilesets(): Promise<unknown>;
  export function set_active(active: boolean): void;
  // JSON of `MapRenderOptions` in `mapViewer.ts`, throws if it is invalid
  export function set_render_options(options: string): void;
  // JSON of `EditorMode` in `mapViewer.ts`, throws if it is invalid
  export function set_editor_mode(mode: string): void;
  export function undo(): void;
  export function redo(): void;
  // `tile_type` of a ground texture, other tile types are ignored
  export function set_map_background(type_name: string): void;
  // `normal`, `basement` or `caves`, throws otherwise
  export function set_map_smoothing(smoothing: string): void;
  export function set_restricted_scroll(restricted_scroll: boolean): void;
  export function roof_all_rooms(): void;
  // Resolves with `{ map, revision, data: Uint8Array }`, rejects if the map can't be saved
  export function save_map(): Promise<unknown>;
  // Resolves with the radar map STI of `map` (`Uint8Array`) once it is loaded, rejects if it can't
  // be loaded or another map is loaded first
  export function render_radar_map(map: string): Promise<unknown>;
  // JSON of `{ gridNo, door }` with a `DoorLock` or `null`, throws if it is invalid
  export function set_door(door: string): void;
  // JSON of `SoldierEdit` in `mapViewer.ts`, throws if it is invalid
  export function set_soldier(soldier: string): void;
  // JSON of `SoldierCommand` in `mapViewer.ts`, throws if it is invalid
  export function soldier_command(command: string): void;
  // JSON of `ItemEdit` in `mapViewer.ts`, throws if it is invalid
  export function set_item(item: string): void;
  // JSON of `ItemCommand` in `mapViewer.ts`, throws if it is invalid
  export function item_command(command: string): void;
  // JSON of `LightEdit` in `mapViewer.ts`, throws if it is invalid
  export function set_light(light: string): void;
  // JSON of `LightCommand` in `mapViewer.ts`, throws if it is invalid
  export function light_command(command: string): void;
  // JSON of `TraversalEdit` in `mapViewer.ts`, throws if it is invalid
  export function set_traversal(edit: string): void;
  // `code` and `key` of a `KeyboardEvent` outside of the canvas, unknown codes are ignored
  export function key_event(
    code: string,
    key: string,
    pressed: boolean,
    repeat: boolean,
  ): void;
  export function release_keys(): void;
}
