import { useReducer } from 'react';
import {
  SoldierDefaults,
  SoldierTeam,
  SoldierMode,
  SoldierTool,
} from '../../../lib/mapViewer';
import { BrushKind, MAX_BRUSH_SIZE, toEditorBrush } from './terrainMode';

/** The relative equipment and attribute levels, from 0 to 4 */
export const RELATIVE_LEVELS = ['Bad', 'Poor', 'Average', 'Good', 'Great'];

/** Everything the soldiers panel can change about the mode */
export interface SoldierModeState {
  tool: SoldierTool;
  /** Removes the soldiers of the teams shown in the layers on the brush instead of placing them */
  erase: boolean;
  /** The brush that erases soldiers */
  brush: BrushKind;
  /** Size of the square and circle brushes and width of the line brush */
  size: number;
  /** The team of new soldiers */
  team: SoldierTeam;
  /** What new soldiers get, the values last set for a soldier like in the original */
  defaults: SoldierDefaults;
}

export const INITIAL_SOLDIER_MODE: SoldierModeState = {
  tool: 'select',
  erase: false,
  brush: 'square',
  size: 1,
  team: 'enemy',
  defaults: {
    orders: 'stationary',
    attitude: 'defensive',
    direction: 'northWest',
    relativeEquipmentLevel: 2,
    relativeAttributeLevel: 2,
    // Army
    soldierClass: 3,
    civilianGroup: 0,
    // Bloodcat
    creatureBody: 20,
  },
};

export type SoldierModeAction =
  | { type: 'setTool'; tool: SoldierTool }
  | { type: 'setErase'; erase: boolean }
  | { type: 'setBrush'; brush: BrushKind }
  | { type: 'setSize'; size: number }
  | { type: 'setTeam'; team: SoldierTeam }
  /** Keeps values of the selected soldier for new soldiers */
  | { type: 'setDefaults'; defaults: Partial<SoldierDefaults> };

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, Math.round(value)));
}

function clampLevel(level: number): number {
  return clamp(level, 0, RELATIVE_LEVELS.length - 1);
}

export function soldierModeReducer(
  state: SoldierModeState,
  action: SoldierModeAction,
): SoldierModeState {
  switch (action.type) {
    case 'setTool':
      return state.tool === action.tool
        ? state
        : { ...state, tool: action.tool };
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
    case 'setTeam':
      return state.team === action.team
        ? state
        : { ...state, team: action.team };
    case 'setDefaults': {
      const defaults = { ...state.defaults, ...action.defaults };
      defaults.relativeEquipmentLevel = clampLevel(
        defaults.relativeEquipmentLevel,
      );
      defaults.relativeAttributeLevel = clampLevel(
        defaults.relativeAttributeLevel,
      );
      const changed = (Object.keys(defaults) as (keyof SoldierDefaults)[]).some(
        (key) => defaults[key] !== state.defaults[key],
      );
      return changed ? { ...state, defaults } : state;
    }
  }
}

/** The mode the viewer gets */
export function toSoldierMode(state: SoldierModeState): SoldierMode {
  return {
    mode: 'soldiers',
    tool: state.tool,
    erase: state.erase,
    brush: toEditorBrush(state.brush, state.size),
    team: state.team,
    defaults: state.defaults,
  };
}

/** The state of the soldiers mode, the sidebar sends it to the viewer */
export function useSoldierMode() {
  return useReducer(soldierModeReducer, INITIAL_SOLDIER_MODE);
}
