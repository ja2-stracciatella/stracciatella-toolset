import { useReducer } from 'react';
import {
  EditorTraversalEvent,
  EntryPoint,
  ENTRY_POINTS,
  ExitGrid,
  SectorId,
  TraversalMode,
  TraversalPlace,
  TraversalTool,
} from '../../../lib/mapViewer';
import { MAX_BRUSH_SIZE, toEditorBrush } from './terrainMode';

/** The brushes of the exit grids, like the original editor allows them */
export type TraversalBrush = 'square' | 'area';

/** The levels of a sector: the surface and three basement levels */
export const MAX_SECTOR_LEVEL = 3;
/** The last grid number of a map */
export const MAX_GRID_NO = 160 * 160 - 1;
/** The rows of the strategic map */
const ROWS = 'ABCDEFGHIJKLMNOP';

/** Everything the traversal panel can change about the mode */
export interface TraversalModeState {
  tool: TraversalTool;
  /** What the place tool places */
  place: TraversalPlace;
  erase: boolean;
  brush: TraversalBrush;
  /** Size of the square brush */
  size: number;
  /** Where new exit grids lead */
  destSector: SectorId;
  destGridNo: number;
  /** The entry point a click sets, if it is not set yet, see `placedEntry` */
  entry: EntryPoint;
}

export const INITIAL_TRAVERSAL_MODE: TraversalModeState = {
  tool: 'select',
  place: 'exit',
  erase: false,
  brush: 'square',
  size: 1,
  destSector: { x: 1, y: 1, z: 0 },
  destGridNo: 0,
  entry: 'north',
};

export type TraversalModeAction =
  | { type: 'setTool'; tool: TraversalTool }
  | { type: 'setPlace'; place: TraversalPlace }
  | { type: 'setErase'; erase: boolean }
  | { type: 'setBrush'; brush: TraversalBrush }
  | { type: 'setSize'; size: number }
  /** Sets the column and row, keeps the level */
  | { type: 'setSector'; x: number; y: number }
  | { type: 'setLevel'; level: number }
  | { type: 'setDestGridNo'; gridNo: number }
  | { type: 'setEntry'; entry: EntryPoint }
  /** Copies the destination of an exit grid, the right click of the viewer */
  | { type: 'copyExitGrid'; exitGrid: ExitGrid };

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, Math.round(value)));
}

/** A sector with its column, row and level clamped like the original editor clamps them */
function clampSector({ x, y, z }: SectorId): SectorId {
  return {
    x: clamp(x, 1, 16),
    y: clamp(y, 1, 16),
    z: clamp(z, 0, MAX_SECTOR_LEVEL),
  };
}

function sameSector(a: SectorId, b: SectorId) {
  return a.x === b.x && a.y === b.y && a.z === b.z;
}

/**
 * The column and row of a sector like `J11` or `a9`, `null` if it is not one: a row letter A to
 * P and a column 1 to 16
 */
export function parseSector(text: string): { x: number; y: number } | null {
  const match = /^\s*([A-Pa-p])\s*(\d{1,2})\s*$/.exec(text);
  const [, row, column] = match ?? [];
  if (row === undefined || column === undefined) {
    return null;
  }
  const x = Number(column);
  if (x < 1 || x > 16) {
    return null;
  }
  return { x, y: ROWS.indexOf(row.toUpperCase()) + 1 };
}

/** `J11` for column 11 of row 10 */
export function formatSector({ x, y }: { x: number; y: number }): string {
  return `${ROWS[y - 1] ?? '?'}${x}`;
}

/** `J11`, or `J11 B1` below the surface */
export function formatDestination(sector: SectorId): string {
  const name = formatSector(sector);
  return sector.z > 0 ? `${name} B${sector.z}` : name;
}

export function traversalModeReducer(
  state: TraversalModeState,
  action: TraversalModeAction,
): TraversalModeState {
  switch (action.type) {
    case 'setTool':
      return state.tool === action.tool
        ? state
        : { ...state, tool: action.tool };
    case 'setPlace':
      return state.place === action.place
        ? state
        : { ...state, place: action.place };
    case 'setErase':
      return state.erase === action.erase
        ? state
        : { ...state, erase: action.erase };
    case 'setBrush':
      return state.brush === action.brush
        ? state
        : { ...state, brush: action.brush };
    case 'setSize': {
      const size = clamp(action.size, 1, MAX_BRUSH_SIZE);
      return state.size === size ? state : { ...state, size };
    }
    case 'setSector': {
      const destSector = clampSector({
        ...state.destSector,
        x: action.x,
        y: action.y,
      });
      return sameSector(state.destSector, destSector)
        ? state
        : { ...state, destSector };
    }
    case 'setLevel': {
      const destSector = clampSector({ ...state.destSector, z: action.level });
      return sameSector(state.destSector, destSector)
        ? state
        : { ...state, destSector };
    }
    case 'setDestGridNo': {
      const destGridNo = clamp(action.gridNo, 0, MAX_GRID_NO);
      return state.destGridNo === destGridNo ? state : { ...state, destGridNo };
    }
    case 'setEntry':
      return state.entry === action.entry
        ? state
        : { ...state, entry: action.entry };
    case 'copyExitGrid': {
      const destSector = clampSector(action.exitGrid.destSector);
      const destGridNo = clamp(action.exitGrid.destGridNo, 0, MAX_GRID_NO);
      return sameSector(state.destSector, destSector) &&
        state.destGridNo === destGridNo
        ? state
        : { ...state, destSector, destGridNo };
    }
  }
}

/** The entry points of the map, `undefined` until the map is loaded */
type EntryPoints = EditorTraversalEvent['entryPoints'] | undefined;

/** The entry points that are not set, the ones that can be placed */
export function availableEntryPoints(entryPoints: EntryPoints): EntryPoint[] {
  return ENTRY_POINTS.filter(
    (point) => (entryPoints?.[point] ?? null) === null,
  );
}

/**
 * The entry point that a click places: the one of the mode if it is not set, otherwise the
 * first one that isn't, `null` if all are set
 */
export function placedEntry(
  state: TraversalModeState,
  entryPoints: EntryPoints,
): EntryPoint | null {
  const available = availableEntryPoints(entryPoints);
  return available.includes(state.entry) ? state.entry : (available[0] ?? null);
}

/**
 * The mode the viewer gets, with the entry point that is placed (`placedEntry`), the place tool
 * paints exit grids if there is none
 */
export function toTraversalMode(
  state: TraversalModeState,
  entry: EntryPoint | null,
): TraversalMode {
  return {
    mode: 'traversal',
    tool: state.tool,
    place: entry === null ? 'exit' : state.place,
    erase: state.erase,
    brush: toEditorBrush(state.brush, state.size),
    destSector: state.destSector,
    destGridNo: state.destGridNo,
    entry: entry ?? state.entry,
  };
}

/** The state of the traversal mode, the sidebar sends it to the viewer in the traversal mode */
export function useTraversalMode() {
  return useReducer(traversalModeReducer, INITIAL_TRAVERSAL_MODE);
}
