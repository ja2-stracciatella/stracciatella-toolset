import { describe, expect, it } from 'vitest';
import {
  availableEntryPoints,
  formatDestination,
  formatSector,
  INITIAL_TRAVERSAL_MODE,
  parseSector,
  placedEntry,
  toTraversalMode,
  traversalModeReducer,
} from '../../../../renderer/components/content/mapEditor/traversalMode';

describe('traversalModeReducer', () => {
  it('should start selecting, and place exit grids to A1', () => {
    expect(toTraversalMode(INITIAL_TRAVERSAL_MODE, 'north')).toEqual({
      mode: 'traversal',
      tool: 'select',
      place: 'exit',
      erase: false,
      brush: { kind: 'square', size: 1 },
      destSector: { x: 1, y: 1, z: 0 },
      destGridNo: 0,
      entry: 'north',
    });
  });

  it('should set the brush, the destination and the entry point', () => {
    let state = traversalModeReducer(INITIAL_TRAVERSAL_MODE, {
      type: 'setSize',
      size: 3,
    });
    state = traversalModeReducer(state, { type: 'setTool', tool: 'place' });
    state = traversalModeReducer(state, { type: 'setSector', x: 11, y: 10 });
    state = traversalModeReducer(state, { type: 'setLevel', level: 1 });
    state = traversalModeReducer(state, { type: 'setDestGridNo', gridNo: 500 });
    state = traversalModeReducer(state, { type: 'setErase', erase: true });
    expect(toTraversalMode(state, 'north')).toEqual({
      mode: 'traversal',
      tool: 'place',
      place: 'exit',
      erase: true,
      brush: { kind: 'square', size: 3 },
      destSector: { x: 11, y: 10, z: 1 },
      destGridNo: 500,
      entry: 'north',
    });
    state = traversalModeReducer(state, { type: 'setBrush', brush: 'area' });
    state = traversalModeReducer(state, { type: 'setPlace', place: 'entry' });
    state = traversalModeReducer(state, { type: 'setEntry', entry: 'center' });
    expect(toTraversalMode(state, 'center')).toMatchObject({
      place: 'entry',
      brush: { kind: 'area' },
      entry: 'center',
    });
    // Without entry points to place, exit grids are placed
    expect(toTraversalMode(state, null)).toMatchObject({
      place: 'exit',
      entry: 'center',
    });
    // The same values keep the state
    expect(
      traversalModeReducer(state, { type: 'setSector', x: 11, y: 10 }),
    ).toBe(state);
    expect(
      traversalModeReducer(state, { type: 'setEntry', entry: 'center' }),
    ).toBe(state);
    expect(
      traversalModeReducer(state, { type: 'setPlace', place: 'entry' }),
    ).toBe(state);
  });

  it('should only place entry points that are not set', () => {
    const entryPoints = {
      north: 12000,
      east: null,
      south: 13000,
      west: null,
      center: null,
      isolated: null,
    };
    expect(availableEntryPoints(entryPoints)).toEqual([
      'east',
      'west',
      'center',
      'isolated',
    ]);
    expect(availableEntryPoints(undefined)).toHaveLength(6);
    // North is set, so the first one that isn't is placed
    expect(placedEntry(INITIAL_TRAVERSAL_MODE, entryPoints)).toBe('east');
    const center = traversalModeReducer(INITIAL_TRAVERSAL_MODE, {
      type: 'setEntry',
      entry: 'center',
    });
    expect(placedEntry(center, entryPoints)).toBe('center');
    expect(placedEntry(center, undefined)).toBe('center');
    const allSet = {
      ...entryPoints,
      east: 1,
      west: 2,
      center: 3,
      isolated: 4,
    };
    expect(placedEntry(center, allSet)).toBe(null);
  });

  it('should clamp the destination like the original editor', () => {
    let state = traversalModeReducer(INITIAL_TRAVERSAL_MODE, {
      type: 'setSector',
      x: 20,
      y: 0,
    });
    state = traversalModeReducer(state, { type: 'setLevel', level: 7 });
    state = traversalModeReducer(state, {
      type: 'setDestGridNo',
      gridNo: 30000,
    });
    expect(state.destSector).toEqual({ x: 16, y: 1, z: 3 });
    expect(state.destGridNo).toBe(25599);
    expect(
      traversalModeReducer(state, { type: 'setDestGridNo', gridNo: -4 })
        .destGridNo,
    ).toBe(0);
    expect(
      traversalModeReducer(state, { type: 'setSize', size: 40 }).size,
    ).toBe(15);
  });

  it('should copy the destination of an exit grid', () => {
    const state = traversalModeReducer(INITIAL_TRAVERSAL_MODE, {
      type: 'copyExitGrid',
      exitGrid: {
        gridNo: 1234,
        destSector: { x: 9, y: 1, z: 1 },
        destGridNo: 4321,
      },
    });
    expect(state.destSector).toEqual({ x: 9, y: 1, z: 1 });
    expect(state.destGridNo).toBe(4321);
  });
});

describe('sectors', () => {
  it('should read a row letter A to P and a column 1 to 16', () => {
    expect(parseSector('J11')).toEqual({ x: 11, y: 10 });
    expect(parseSector(' a9 ')).toEqual({ x: 9, y: 1 });
    expect(parseSector('P16')).toEqual({ x: 16, y: 16 });
    for (const invalid of ['Q1', 'A0', 'A17', 'A', '11', 'J11 B1', '']) {
      expect(parseSector(invalid)).toBe(null);
    }
  });

  it('should format sectors and levels', () => {
    expect(formatSector({ x: 11, y: 10 })).toBe('J11');
    expect(formatDestination({ x: 9, y: 1, z: 0 })).toBe('A9');
    expect(formatDestination({ x: 9, y: 1, z: 2 })).toBe('A9 B2');
  });
});
