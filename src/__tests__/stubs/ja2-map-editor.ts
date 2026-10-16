// Stands in for the wasm map viewer (src-map-editor/pkg), which is not built for tests

function notAvailable(): never {
  throw new Error('The map viewer is not available in tests');
}

export default async function init(): Promise<unknown> {
  notAvailable();
}

export const start = notAvailable;
export const load_map = notAvailable;
export const set_active = notAvailable;
export const set_render_options = notAvailable;
export const set_editor_mode = notAvailable;
export const undo = notAvailable;
export const set_map_background = notAvailable;
export const save_map = notAvailable;
export const set_soldier = notAvailable;
export const soldier_command = notAvailable;
export const set_item = notAvailable;
export const item_command = notAvailable;
export const set_light = notAvailable;
export const light_command = notAvailable;
export const set_traversal = notAvailable;
