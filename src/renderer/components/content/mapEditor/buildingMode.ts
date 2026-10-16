import { useReducer } from 'react';
import {
  BuildingMode,
  BuildingPalettes,
  BuildingSelections,
  BuildingTool,
  EditorPaletteEvent,
  MapFloor,
  MapLayers,
  MapSmoothing,
  PaletteEntry,
  SmartValue,
  TileSelection,
} from '../../../lib/mapViewer';
import {
  BrushKind,
  changeCount,
  MAX_BRUSH_SIZE,
  toEditorBrush,
} from './terrainMode';

/** The palette that a tool of the buildings mode places tiles of */
export type SelectionPalette = keyof BuildingSelections;

/** The palette of a tool, the whole building tools use the room palette */
export function toolPalette(tool: BuildingTool): SelectionPalette {
  switch (tool) {
    case 'tiles':
      return 'tiles';
    default:
      return 'room';
  }
}

/** The kinds of tiles of the tiles tool, the palettes it merges */
export type TileKind =
  | 'walls'
  | 'doors'
  | 'windows'
  | 'roofs'
  | 'brokenWalls'
  | 'decals'
  | 'floors';

/** The palettes of the tiles tool with their labels, in the order they are shown */
export const TILE_PALETTES: [TileKind, string][] = [
  ['walls', 'Walls'],
  ['doors', 'Doors'],
  ['windows', 'Windows'],
  ['roofs', 'Roofs'],
  ['brokenWalls', 'Broken walls'],
  ['decals', 'Decals'],
  ['floors', 'Floors'],
];

/** The kinds of the tiles of a selection of the tiles tool, by the palettes they are in */
export function selectedKinds(
  selection: TileSelection[],
  palettes: BuildingPalettes,
): Set<TileKind> {
  const kinds = new Set<TileKind>();
  for (const [kind] of TILE_PALETTES) {
    const entries = palettes[kind];
    if (
      selection.some(({ tileId }) => entries.some((e) => e.tileId === tileId))
    ) {
      kinds.add(kind);
    }
  }
  return kinds;
}

/** Whether a selection of the tiles tool has only floors, which take any brush */
export function isFloorsOnly(kinds: Set<TileKind>): boolean {
  return kinds.size === 1 && kinds.has('floors');
}

/** The tools on whole rooms and buildings, which cave maps don't have */
const WHOLE_BUILDING_TOOLS: BuildingTool[] = ['room', 'select'];

/**
 * Whether the map type allows a tool (`SetEditorSmoothingMode`): cave maps only have the caves
 * tool and no smart method
 */
export function allowsTool(
  smoothing: MapSmoothing,
  tool: BuildingTool,
): boolean {
  switch (smoothing) {
    case 'caves':
      return !WHOLE_BUILDING_TOOLS.includes(tool) && !isSmart(tool);
    case 'basement':
      return tool !== 'caves';
    default:
      return tool !== 'caves';
  }
}

/** The smart tool, which places what the smart value says */
export function isSmart(tool: BuildingTool): boolean {
  return tool === 'smart';
}

/** Whether erasing removes what the tool places, like in the original editor */
export function canErase(tool: BuildingTool): boolean {
  return (
    (!WHOLE_BUILDING_TOOLS.includes(tool) || tool === 'room') &&
    tool !== 'caves' &&
    tool !== 'doorLocks'
  );
}

/** Whether the smart value is a roof, which replaces the roof of the clicked building */
export function isSmartRoof(
  smart: SmartValue | null,
  palettes: BuildingPalettes,
): boolean {
  return (
    smart?.kind === 'tile' &&
    palettes.smartRoofs.some((e) => e.tileId === smart.tileId)
  );
}

/** The highest room number of the map files */
export const MAX_ROOM_NUMBER = 255;

/**
 * Whether the tool uses the brush, the others place on the clicked tile. The tiles tool uses
 * it when only floors are selected.
 */
export function usesBrush(
  tool: BuildingTool,
  erase: boolean,
  floorsOnly = false,
): boolean {
  return (
    (tool === 'tiles' && floorsOnly) ||
    ['room', 'roomNumber', 'caves'].includes(tool) ||
    (erase && canErase(tool))
  );
}

/** Whether the tool uses the tiles of its palette */
export function usesPalette(tool: BuildingTool): boolean {
  return (
    !isSmart(tool) &&
    !['select', 'roomNumber', 'caves', 'doorLocks'].includes(tool)
  );
}

/** The layers and the floor a building tool shows */
export interface BuildingViews {
  layers: Partial<Pick<MapLayers, 'buildings' | 'roomNumbers'>>;
  floor: MapFloor;
}

/**
 * The views a tool shows, like `SetEditorBuildingTaskbarMode`: the buildings, and the roof floor
 * only for the select tool, the tiles and smart tools with roofs selected and in basements,
 * whose roofs are the rock
 */
export function buildingViews(
  tool: BuildingTool,
  basement: boolean,
  roofsSelected = false,
): BuildingViews {
  const floor = (roof: boolean): MapFloor => (roof ? 'roof' : 'ground');
  switch (tool) {
    case 'select':
      return { layers: { buildings: true, roomNumbers: true }, floor: 'roof' };
    case 'tiles':
    case 'smart':
      return {
        layers: { buildings: true, roomNumbers: false },
        floor: floor(basement || roofsSelected),
      };
    case 'roomNumber':
      return { layers: { roomNumbers: true }, floor: floor(basement) };
    default:
      return {
        layers: { buildings: true, roomNumbers: false },
        floor: floor(basement),
      };
  }
}

/**
 * Whether new rooms get a slant roof: the room selection has a slant roof, but no flat roof. The
 * viewer then uses a line 8 tiles wide as brush, like the original.
 */
export function hasSlantRoofOnly(
  selection: BuildingSelections['room'],
  entries: PaletteEntry[],
): boolean {
  const types = selection.flatMap(
    ({ tileId }) => entries.find((e) => e.tileId === tileId)?.typeName ?? [],
  );
  return (
    types.some((t) => t.startsWith('SROOF')) &&
    !types.some((t) => t.startsWith('ROOF'))
  );
}

/** Everything the buildings panel can change */
export interface BuildingModeState {
  tool: BuildingTool;
  erase: boolean;
  brush: BrushKind;
  /** Size of the square and circle brushes and width of the line brush */
  size: number;
  /** The tiles of each palette, picked at random by their counts */
  selections: BuildingSelections;
  smart: SmartValue | null;
  /** The room the room number tool paints, 0 is no room */
  roomNumber: number;
}

export const INITIAL_BUILDING_MODE: BuildingModeState = {
  tool: 'room',
  erase: false,
  brush: 'area',
  size: 1,
  selections: {
    room: [],
    tiles: [],
  },
  smart: null,
  roomNumber: 1,
};

export type BuildingModeAction =
  | { type: 'setTool'; tool: BuildingTool }
  | { type: 'setErase'; erase: boolean }
  | { type: 'setBrush'; brush: BrushKind }
  | { type: 'setSize'; size: number }
  /** Places only this tile of the palette */
  | { type: 'selectTile'; palette: SelectionPalette; tileId: number }
  | { type: 'addTile'; palette: SelectionPalette; tileId: number }
  | { type: 'removeTile'; palette: SelectionPalette; tileId: number }
  | { type: 'clearTiles'; palette: SelectionPalette }
  | { type: 'setSmart'; smart: SmartValue | null }
  | { type: 'setRoomNumber'; roomNumber: number }
  /** Selects the first tiles of the palettes that have nothing selected */
  | { type: 'initSelections'; palette: EditorPaletteEvent };

/** The first entry whose type name starts with each prefix */
function firstOfTypes(
  entries: PaletteEntry[],
  prefixes: string[],
): PaletteEntry[] {
  return prefixes.flatMap((prefix) => {
    const entry = entries.find((e) => e.typeName.startsWith(prefix));
    return entry ? [entry] : [];
  });
}

/** What is selected in a palette before anything is chosen */
function initialSelection(
  palettes: BuildingPalettes,
  palette: SelectionPalette,
): PaletteEntry[] {
  if (palette === 'tiles') {
    // The first wall
    return palettes.walls.slice(0, 1);
  }
  const entries = palettes[palette];
  if (palette === 'room') {
    // A wall, a floor and a flat roof
    return firstOfTypes(entries, ['WALL', 'FLOOR', 'ROOF']);
  }
  return entries.slice(0, 1);
}

function withSelection(
  state: BuildingModeState,
  palette: SelectionPalette,
  selection: BuildingSelections[SelectionPalette],
): BuildingModeState {
  if (selection === state.selections[palette]) {
    return state;
  }
  return {
    ...state,
    selections: { ...state.selections, [palette]: selection },
  };
}

export function buildingModeReducer(
  state: BuildingModeState,
  action: BuildingModeAction,
): BuildingModeState {
  switch (action.type) {
    case 'setTool':
      return { ...state, tool: action.tool };
    case 'setErase':
      return { ...state, erase: action.erase };
    case 'setBrush':
      return { ...state, brush: action.brush };
    case 'setSize':
      return {
        ...state,
        size: Math.max(1, Math.min(MAX_BRUSH_SIZE, Math.round(action.size))),
      };
    case 'selectTile':
      return withSelection(state, action.palette, [
        { tileId: action.tileId, count: 1 },
      ]);
    case 'addTile':
    case 'removeTile': {
      const change = action.type === 'addTile' ? 1 : -1;
      const selection = changeCount(
        state.selections[action.palette],
        action.tileId,
        change,
      );
      return withSelection(state, action.palette, selection);
    }
    case 'clearTiles':
      return withSelection(state, action.palette, []);
    case 'setSmart':
      return { ...state, smart: action.smart };
    case 'setRoomNumber':
      return {
        ...state,
        roomNumber: Math.max(
          0,
          Math.min(MAX_ROOM_NUMBER, Math.round(action.roomNumber)),
        ),
      };
    case 'initSelections': {
      const palettes = action.palette.buildings;
      let next = state;
      for (const palette of Object.keys(
        state.selections,
      ) as SelectionPalette[]) {
        if (state.selections[palette].length > 0) {
          continue;
        }
        const selection = initialSelection(palettes, palette).map((e) => ({
          tileId: e.tileId,
          count: 1,
        }));
        if (selection.length > 0) {
          next = withSelection(next, palette, selection);
        }
      }
      // The first smart wall
      const wall = palettes.smartWalls[0];
      if (next.smart === null && wall) {
        next = { ...next, smart: { kind: 'tile', tileId: wall.tileId } };
      }
      return next;
    }
  }
}

/** The mode the viewer gets */
export function toBuildingMode(state: BuildingModeState): BuildingMode {
  return {
    mode: 'buildings',
    tool: state.tool,
    erase: state.erase,
    brush: toEditorBrush(state.brush, state.size),
    selections: state.selections,
    smart: state.smart,
    roomNumber: state.roomNumber,
  };
}

/** The state of the buildings mode, the sidebar sends it to the viewer */
export function useBuildingMode() {
  return useReducer(buildingModeReducer, INITIAL_BUILDING_MODE);
}
