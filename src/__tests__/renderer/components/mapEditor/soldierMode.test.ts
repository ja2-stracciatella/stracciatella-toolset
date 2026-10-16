import { describe, expect, it } from 'vitest';
import {
  INITIAL_SOLDIER_MODE,
  soldierModeReducer,
  SoldierModeAction,
  SoldierModeState,
  toSoldierMode,
} from '../../../../renderer/components/content/mapEditor/soldierMode';
import { MAX_BRUSH_SIZE } from '../../../../renderer/components/content/mapEditor/terrainMode';

function reduce(...actions: SoldierModeAction[]): SoldierModeState {
  return actions.reduce(soldierModeReducer, INITIAL_SOLDIER_MODE);
}

describe('soldierModeReducer', () => {
  it('should start with the defaults of the original editor', () => {
    expect(toSoldierMode(INITIAL_SOLDIER_MODE)).toEqual({
      mode: 'soldiers',
      tool: 'select',
      erase: false,
      brush: { kind: 'square', size: 1 },
      team: 'enemy',
      defaults: {
        orders: 'stationary',
        attitude: 'defensive',
        direction: 'northWest',
        relativeEquipmentLevel: 2,
        relativeAttributeLevel: 2,
        soldierClass: 3,
        civilianGroup: 0,
        creatureBody: 20,
      },
    });
  });

  it('should set the team', () => {
    const state = reduce({ type: 'setTeam', team: 'civilian' });
    expect(state.team).toBe('civilian');
    expect(toSoldierMode(state).team).toBe('civilian');
  });

  it('should set the tool and the brush that erases', () => {
    const state = reduce(
      { type: 'setTool', tool: 'place' },
      { type: 'setErase', erase: true },
      { type: 'setBrush', brush: 'circle' },
      { type: 'setSize', size: 4.4 },
    );
    expect(state).toMatchObject({
      tool: 'place',
      erase: true,
      brush: 'circle',
      size: 4,
    });
    expect(toSoldierMode(state)).toMatchObject({
      tool: 'place',
      erase: true,
      brush: { kind: 'circle', size: 4 },
    });
    // The size is clamped
    expect(reduce({ type: 'setSize', size: 0 }).size).toBe(1);
    expect(reduce({ type: 'setSize', size: 99 }).size).toBe(MAX_BRUSH_SIZE);
  });

  it('should keep the state when nothing changes', () => {
    expect(reduce({ type: 'setTeam', team: 'enemy' })).toBe(
      INITIAL_SOLDIER_MODE,
    );
    expect(reduce({ type: 'setTool', tool: 'select' })).toBe(
      INITIAL_SOLDIER_MODE,
    );
    expect(reduce({ type: 'setErase', erase: false })).toBe(
      INITIAL_SOLDIER_MODE,
    );
    expect(reduce({ type: 'setBrush', brush: 'square' })).toBe(
      INITIAL_SOLDIER_MODE,
    );
    expect(reduce({ type: 'setSize', size: 1 })).toBe(INITIAL_SOLDIER_MODE);
    expect(
      reduce({ type: 'setDefaults', defaults: { orders: 'stationary' } }),
    ).toBe(INITIAL_SOLDIER_MODE);
  });

  it('should keep the values of the selected soldier for new soldiers', () => {
    const state = reduce({
      type: 'setDefaults',
      defaults: {
        orders: 'pointPatrol',
        direction: 'south',
        relativeEquipmentLevel: 7,
        relativeAttributeLevel: -1,
      },
    });
    expect(state.defaults).toMatchObject({
      orders: 'pointPatrol',
      direction: 'south',
      attitude: 'defensive',
      // Clamped to the 5 levels
      relativeEquipmentLevel: 4,
      relativeAttributeLevel: 0,
    });
  });
});
