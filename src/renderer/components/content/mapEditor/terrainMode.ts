import { useReducer } from 'react';
import {
  EditorBrush,
  EditorPaletteEvent,
  ObjectsMode,
  TerrainMode,
  TerrainModeName,
  TerrainTool,
  TextureWeight,
  TileSelection,
} from '../../../lib/mapViewer';
import type { TileGroup } from './TilePalette';

export type BrushKind = EditorBrush['kind'];

/** Maximum weight of a ground texture, like in the original editor */
export const MAX_TEXTURE_WEIGHT = 10;
/** Maximum size of the square and circle brushes and width of the line brush */
export const MAX_BRUSH_SIZE = 15;

/** The palettes that make up the objects */
export function objectGroups(palette: EditorPaletteEvent): TileGroup[] {
  return [
    { label: 'Debris', entries: palette.debris },
    { label: 'Trees & bushes', entries: palette.trees },
    { label: 'Rocks', entries: palette.rocks },
    { label: 'Misc junk', entries: palette.misc },
    { label: 'Furniture', entries: palette.furniture },
    { label: 'Generic furniture', entries: palette.genericFurniture },
  ];
}

/** The brush that is used, only the ground tool of the terrain mode fills */
export function effectiveBrush(brush: BrushKind, fills: boolean): BrushKind {
  return brush === 'fill' && !fills ? 'square' : brush;
}

/** Everything the terrain panel can change, shared by the terrain and objects modes */
export interface TerrainModeState {
  /** The tool of the terrain mode */
  tool: TerrainTool;
  erase: boolean;
  brush: BrushKind;
  /** Size of the square and circle brushes and width of the line brush */
  size: number;
  /** Chance of a placement on each tile of the brush, in percent */
  density: number;
  /** Textures by their type name, nothing is selected until the palette arrives */
  ground: { selected: string | null; weights: TextureWeight[] };
  /** The objects picked at random, by their counts */
  objects: TileSelection[];
  road: number;
  /** The sub index of the cliff piece */
  cliff: number;
}

export const INITIAL_TERRAIN_MODE: TerrainModeState = {
  tool: 'ground',
  erase: false,
  brush: 'square',
  size: 1,
  density: 2,
  ground: { selected: null, weights: [] },
  objects: [],
  road: 0,
  cliff: 1,
};

export type TerrainModeAction =
  | { type: 'setTool'; tool: TerrainTool }
  | { type: 'setErase'; erase: boolean }
  | { type: 'setBrush'; brush: BrushKind }
  | { type: 'setSize'; size: number }
  | { type: 'setDensity'; density: number }
  /** Paints only this texture */
  | { type: 'selectTexture'; typeName: string }
  | { type: 'addTextureWeight'; typeName: string }
  | { type: 'removeTextureWeight'; typeName: string }
  /** Places only this object */
  | { type: 'selectTile'; tileId: number }
  | { type: 'addTile'; tileId: number }
  | { type: 'removeTile'; tileId: number }
  | { type: 'clearTiles' }
  | { type: 'selectRoad'; road: number }
  | { type: 'selectCliff'; cliff: number }
  /** Selects the first texture and the first object, if nothing is selected */
  | { type: 'initSelections'; palette: EditorPaletteEvent };

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, Math.round(value)));
}

/** Changes the count of a tile, removing it at 0 */
export function changeCount(
  selections: TileSelection[],
  tileId: number,
  change: number,
): TileSelection[] {
  const existing = selections.find((s) => s.tileId === tileId);
  if (!existing) {
    return change > 0 ? [...selections, { tileId, count: change }] : selections;
  }
  const count = existing.count + change;
  if (count <= 0) {
    return selections.filter((s) => s.tileId !== tileId);
  }
  return selections.map((s) => (s.tileId === tileId ? { ...s, count } : s));
}

function changeWeight(
  state: TerrainModeState,
  typeName: string,
  change: number,
): TerrainModeState {
  const current =
    state.ground.weights.find((w) => w.typeName === typeName)?.weight ?? 0;
  const weight = Math.max(0, Math.min(MAX_TEXTURE_WEIGHT, current + change));
  if (weight === current) {
    return state;
  }
  let weights: TextureWeight[];
  if (weight === 0) {
    weights = state.ground.weights.filter((w) => w.typeName !== typeName);
  } else if (current === 0) {
    weights = [...state.ground.weights, { typeName, weight }];
  } else {
    weights = state.ground.weights.map((w) =>
      w.typeName === typeName ? { typeName, weight } : w,
    );
  }
  return {
    ...state,
    ground: {
      selected: change > 0 ? typeName : state.ground.selected,
      weights,
    },
  };
}

export function terrainModeReducer(
  state: TerrainModeState,
  action: TerrainModeAction,
): TerrainModeState {
  switch (action.type) {
    case 'setTool':
      return { ...state, tool: action.tool };
    case 'setErase':
      return { ...state, erase: action.erase };
    case 'setBrush':
      return { ...state, brush: action.brush };
    case 'setSize':
      return { ...state, size: clamp(action.size, 1, MAX_BRUSH_SIZE) };
    case 'setDensity':
      return { ...state, density: clamp(action.density, 1, 100) };
    case 'selectTexture':
      return {
        ...state,
        ground: { selected: action.typeName, weights: [] },
      };
    case 'addTextureWeight':
      return changeWeight(state, action.typeName, 1);
    case 'removeTextureWeight':
      return changeWeight(state, action.typeName, -1);
    case 'selectTile':
      return { ...state, objects: [{ tileId: action.tileId, count: 1 }] };
    case 'addTile':
    case 'removeTile': {
      const change = action.type === 'addTile' ? 1 : -1;
      const objects = changeCount(state.objects, action.tileId, change);
      return objects === state.objects ? state : { ...state, objects };
    }
    case 'clearTiles':
      return { ...state, objects: [] };
    case 'selectRoad':
      return { ...state, road: action.road };
    case 'selectCliff':
      return { ...state, cliff: action.cliff };
    case 'initSelections': {
      const firstObject = objectGroups(action.palette).flatMap(
        (g) => g.entries,
      )[0];
      const objects =
        state.objects.length === 0 && firstObject
          ? [{ tileId: firstObject.tileId, count: 1 }]
          : state.objects;
      const firstTexture = action.palette.ground[0];
      const ground =
        state.ground.selected === null && firstTexture
          ? { ...state.ground, selected: firstTexture.typeName }
          : state.ground;
      return { ...state, ground, objects };
    }
  }
}

/** The brush the viewer gets, the size is the width of the line */
export function toEditorBrush(kind: BrushKind, size: number): EditorBrush {
  if (kind === 'square' || kind === 'circle') {
    return { kind, size };
  }
  if (kind === 'line') {
    return { kind, width: size };
  }
  return { kind };
}

/** The mode the viewer gets */
export function toEditorMode(
  state: TerrainModeState,
  mode: TerrainModeName,
): TerrainMode | ObjectsMode {
  const { tool, erase, density, road, cliff } = state;
  const fills = mode === 'terrain' && tool === 'ground';
  const brush = toEditorBrush(effectiveBrush(state.brush, fills), state.size);
  switch (mode) {
    case 'terrain':
      return {
        mode,
        tool,
        erase,
        brush,
        density,
        ground: state.ground,
        road,
        cliff,
      };
    case 'objects':
      return { mode, erase, brush, density, objects: state.objects };
  }
}

/** The state of the terrain modes, the sidebar sends it to the viewer in the terrain modes */
export function useTerrainMode() {
  return useReducer(terrainModeReducer, INITIAL_TERRAIN_MODE);
}
