import { useReducer } from 'react';
import {
  LIGHT_RADII,
  LightTime,
  LightMode,
  LightTool,
} from '../../../lib/mapViewer';
import { BrushKind, MAX_BRUSH_SIZE, toEditorBrush } from './terrainMode';

/** Everything the lights panel can change about the mode */
export interface LightModeState {
  tool: LightTool;
  /** Removes the lights of the brush instead of placing them */
  erase: boolean;
  /** The brush that erases lights */
  brush: BrushKind;
  /** Size of the square and circle brushes and width of the line brush */
  size: number;
  /** Radius of new lights, the template `L-R0N.LHT` */
  radius: number;
  /** When new lights are on */
  time: LightTime;
}

/** The radius and the time are the defaults of the original editor */
export const INITIAL_LIGHT_MODE: LightModeState = {
  tool: 'select',
  erase: false,
  brush: 'square',
  size: 1,
  radius: 5,
  time: 'primeTime',
};

export type LightModeAction =
  | { type: 'setTool'; tool: LightTool }
  | { type: 'setErase'; erase: boolean }
  | { type: 'setBrush'; brush: BrushKind }
  | { type: 'setSize'; size: number }
  | { type: 'setRadius'; radius: number }
  | { type: 'setTime'; time: LightTime };

const MIN_RADIUS = Math.min(...LIGHT_RADII);
const MAX_RADIUS = Math.max(...LIGHT_RADII);

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, Math.round(value)));
}

export function lightModeReducer(
  state: LightModeState,
  action: LightModeAction,
): LightModeState {
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
    case 'setRadius': {
      const radius = clamp(action.radius, MIN_RADIUS, MAX_RADIUS);
      return state.radius === radius ? state : { ...state, radius };
    }
    case 'setTime':
      return state.time === action.time
        ? state
        : { ...state, time: action.time };
  }
}

/** The mode the viewer gets */
export function toLightMode(state: LightModeState): LightMode {
  return {
    mode: 'lights',
    tool: state.tool,
    erase: state.erase,
    brush: toEditorBrush(state.brush, state.size),
    radius: state.radius,
    time: state.time,
  };
}

/** The state of the lights mode, the sidebar sends it to the viewer */
export function useLightMode() {
  return useReducer(lightModeReducer, INITIAL_LIGHT_MODE);
}
