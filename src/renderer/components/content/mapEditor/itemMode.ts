import { useReducer } from 'react';
import { ItemChoice, ItemMode, ItemTool } from '../../../lib/mapViewer';
import { ItemCategory } from './itemData';
import { BrushKind, MAX_BRUSH_SIZE, toEditorBrush } from './terrainMode';

/** The highest trap level */
export const MAX_TRAP_LEVEL = 20;
/** What new action items do in the original: a medium explosion (`ACTIONITEM_MEDIUM`) */
export const DEFAULT_ACTION = 12;

/** Everything the items panel can change about the mode */
export interface ItemModeState {
  tool: ItemTool;
  /** Removes the items of the brush instead of placing them */
  erase: boolean;
  /** The brush that erases items */
  brush: BrushKind;
  /** Size of the square and circle brushes and width of the line brush */
  size: number;
  /** The category the palette shows */
  category: ItemCategory;
  /** What a click places, by category, so each category keeps its choice like the original */
  choices: Partial<Record<ItemCategory, ItemChoice>>;
  /** Trap level of new action items, the one last set for an action item */
  trapLevel: number;
  /** What new action items do, the action last chosen */
  action: number;
}

export const INITIAL_ITEM_MODE: ItemModeState = {
  tool: 'select',
  erase: false,
  brush: 'square',
  size: 1,
  category: 'weapons',
  choices: {},
  trapLevel: 9,
  action: DEFAULT_ACTION,
};

export type ItemModeAction =
  | { type: 'setTool'; tool: ItemTool }
  | { type: 'setErase'; erase: boolean }
  | { type: 'setBrush'; brush: BrushKind }
  | { type: 'setSize'; size: number }
  | { type: 'setCategory'; category: ItemCategory }
  | { type: 'choose'; category: ItemCategory; choice: ItemChoice }
  | { type: 'setTrapLevel'; trapLevel: number }
  | { type: 'setAction'; action: number };

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, Math.round(value)));
}

function sameChoice(a: ItemChoice | undefined, b: ItemChoice): boolean {
  return a !== undefined && JSON.stringify(a) === JSON.stringify(b);
}

export function itemModeReducer(
  state: ItemModeState,
  action: ItemModeAction,
): ItemModeState {
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
    case 'setCategory':
      return state.category === action.category
        ? state
        : { ...state, category: action.category };
    case 'choose':
      if (sameChoice(state.choices[action.category], action.choice)) {
        return state;
      }
      return {
        ...state,
        choices: { ...state.choices, [action.category]: action.choice },
      };
    case 'setTrapLevel': {
      const trapLevel = clamp(action.trapLevel, 0, MAX_TRAP_LEVEL);
      return state.trapLevel === trapLevel ? state : { ...state, trapLevel };
    }
    case 'setAction':
      return state.action === action.action
        ? state
        : { ...state, action: action.action };
  }
}

/** What a click places: the choice of the shown category */
export function currentChoice(state: ItemModeState): ItemChoice | null {
  return state.choices[state.category] ?? null;
}

/** The mode the viewer gets */
export function toItemMode(state: ItemModeState): ItemMode {
  return {
    mode: 'items',
    tool: state.tool,
    erase: state.erase,
    brush: toEditorBrush(state.brush, state.size),
    choice: currentChoice(state),
    trapLevel: state.trapLevel,
    action: state.action,
  };
}

/** The state of the items mode, the sidebar sends it to the viewer */
export function useItemMode() {
  return useReducer(itemModeReducer, INITIAL_ITEM_MODE);
}
