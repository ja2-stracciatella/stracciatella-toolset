import { describe, expect, it } from 'vitest';
import {
  INITIAL_ITEM_MODE,
  itemModeReducer,
  toItemMode,
} from '../../../../renderer/components/content/mapEditor/itemMode';

describe('itemModeReducer', () => {
  it('should keep a choice for each category', () => {
    let state = itemModeReducer(INITIAL_ITEM_MODE, {
      type: 'choose',
      category: 'weapons',
      choice: { kind: 'item', item: 2 },
    });
    expect(toItemMode(state)).toEqual({
      mode: 'items',
      tool: 'select',
      erase: false,
      brush: { kind: 'square', size: 1 },
      choice: { kind: 'item', item: 2 },
      trapLevel: 9,
      action: 12,
    });
    state = itemModeReducer(state, { type: 'setCategory', category: 'keys' });
    expect(toItemMode(state).choice).toBe(null);
    state = itemModeReducer(state, {
      type: 'choose',
      category: 'keys',
      choice: { kind: 'key', lockId: 3, item: 274 },
    });
    state = itemModeReducer(state, {
      type: 'setCategory',
      category: 'weapons',
    });
    expect(toItemMode(state).choice).toEqual({ kind: 'item', item: 2 });
    // The same choice keeps the state
    expect(
      itemModeReducer(state, {
        type: 'choose',
        category: 'weapons',
        choice: { kind: 'item', item: 2 },
      }),
    ).toBe(state);
  });

  it('should clamp the defaults of new items', () => {
    let state = itemModeReducer(INITIAL_ITEM_MODE, {
      type: 'setTrapLevel',
      trapLevel: 30,
    });
    state = itemModeReducer(state, { type: 'setAction', action: 9 });
    expect(toItemMode(state)).toMatchObject({
      trapLevel: 20,
      action: 9,
    });
    expect(
      itemModeReducer(state, { type: 'setTrapLevel', trapLevel: 20.2 }),
    ).toBe(state);
  });

  it('should set the tool and the erase brush', () => {
    let state = itemModeReducer(INITIAL_ITEM_MODE, {
      type: 'setTool',
      tool: 'place',
    });
    state = itemModeReducer(state, { type: 'setErase', erase: true });
    state = itemModeReducer(state, { type: 'setBrush', brush: 'line' });
    state = itemModeReducer(state, { type: 'setSize', size: 4 });
    expect(toItemMode(state)).toEqual({
      mode: 'items',
      tool: 'place',
      erase: true,
      brush: { kind: 'line', width: 4 },
      choice: null,
      trapLevel: 9,
      action: 12,
    });
    // The same values keep the state
    expect(itemModeReducer(state, { type: 'setTool', tool: 'place' })).toBe(
      state,
    );
    expect(itemModeReducer(state, { type: 'setErase', erase: true })).toBe(
      state,
    );
    expect(itemModeReducer(state, { type: 'setBrush', brush: 'line' })).toBe(
      state,
    );
    expect(itemModeReducer(state, { type: 'setSize', size: 4 })).toBe(state);
    // The size is clamped
    expect(itemModeReducer(state, { type: 'setSize', size: 40 }).size).toBe(15);
  });
});
