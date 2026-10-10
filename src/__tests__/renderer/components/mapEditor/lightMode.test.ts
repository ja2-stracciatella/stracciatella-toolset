import { describe, expect, it } from 'vitest';
import {
  INITIAL_LIGHT_MODE,
  lightModeReducer,
  toLightMode,
} from '../../../../renderer/components/content/mapEditor/lightMode';

describe('lightModeReducer', () => {
  it('should start with the select tool and the lights of the original editor', () => {
    expect(toLightMode(INITIAL_LIGHT_MODE)).toEqual({
      mode: 'lights',
      tool: 'select',
      erase: false,
      brush: { kind: 'square', size: 1 },
      radius: 5,
      time: 'primeTime',
    });
  });

  it('should set the radius and the time of new lights', () => {
    let state = lightModeReducer(INITIAL_LIGHT_MODE, {
      type: 'setRadius',
      radius: 3,
    });
    state = lightModeReducer(state, { type: 'setTime', time: 'night' });
    expect(toLightMode(state)).toMatchObject({ radius: 3, time: 'night' });
    // The same values keep the state
    expect(lightModeReducer(state, { type: 'setRadius', radius: 3 })).toBe(
      state,
    );
    expect(lightModeReducer(state, { type: 'setTime', time: 'night' })).toBe(
      state,
    );
  });

  it('should clamp the radius to the templates of the editor', () => {
    expect(
      lightModeReducer(INITIAL_LIGHT_MODE, { type: 'setRadius', radius: 0 })
        .radius,
    ).toBe(1);
    expect(
      lightModeReducer(INITIAL_LIGHT_MODE, { type: 'setRadius', radius: 12 })
        .radius,
    ).toBe(8);
  });

  it('should set the tool and the erase brush', () => {
    let state = lightModeReducer(INITIAL_LIGHT_MODE, {
      type: 'setTool',
      tool: 'place',
    });
    state = lightModeReducer(state, { type: 'setErase', erase: true });
    state = lightModeReducer(state, { type: 'setBrush', brush: 'line' });
    state = lightModeReducer(state, { type: 'setSize', size: 4 });
    expect(toLightMode(state)).toEqual({
      mode: 'lights',
      tool: 'place',
      erase: true,
      brush: { kind: 'line', width: 4 },
      radius: 5,
      time: 'primeTime',
    });
    // The same values keep the state
    expect(lightModeReducer(state, { type: 'setTool', tool: 'place' })).toBe(
      state,
    );
    expect(lightModeReducer(state, { type: 'setErase', erase: true })).toBe(
      state,
    );
    expect(lightModeReducer(state, { type: 'setBrush', brush: 'line' })).toBe(
      state,
    );
    expect(lightModeReducer(state, { type: 'setSize', size: 4 })).toBe(state);
  });

  it('should clamp the brush size', () => {
    expect(
      lightModeReducer(INITIAL_LIGHT_MODE, { type: 'setSize', size: 0 }).size,
    ).toBe(1);
    expect(
      lightModeReducer(INITIAL_LIGHT_MODE, { type: 'setSize', size: 40 }).size,
    ).toBe(15);
  });
});
