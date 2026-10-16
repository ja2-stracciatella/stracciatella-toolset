import { describe, expect, it } from 'vitest';
import {
  allowsTool,
  BuildingModeAction,
  buildingModeReducer,
  BuildingModeState,
  buildingViews,
  canErase,
  hasSlantRoofOnly,
  INITIAL_BUILDING_MODE,
  isFloorsOnly,
  isSmart,
  isSmartRoof,
  selectedKinds,
  toBuildingMode,
  toolPalette,
  usesBrush,
  usesPalette,
} from '../../../../renderer/components/content/mapEditor/buildingMode';
import { paletteGroups } from '../../../../renderer/components/content/mapEditor/BuildingPanel';
import {
  EditorPaletteEvent,
  PaletteEntry,
} from '../../../../renderer/lib/mapViewer';
import { emptyBuildingPalettes } from '../../test-utils/palette';

function reduce(...actions: BuildingModeAction[]): BuildingModeState {
  return actions.reduce(buildingModeReducer, INITIAL_BUILDING_MODE);
}

function entry(tileId: number, typeName: string): PaletteEntry {
  return {
    tileId,
    typeName,
    subIndex: 1,
    file: `tilesets/0/${typeName}.sti`,
    subImage: 0,
  };
}

function paletteEvent(): EditorPaletteEvent {
  return {
    type: 'palette',
    map: 'maps/A9.dat',
    tileset: 0,
    ground: [],
    roads: [],
    cliffs: [],
    debris: [],
    trees: [],
    rocks: [],
    misc: [],
    furniture: [],
    genericFurniture: [],
    buildings: {
      ...emptyBuildingPalettes(),
      room: [
        entry(1, 'WALL1'),
        entry(2, 'WALL2'),
        entry(3, 'FLOOR1'),
        entry(4, 'ROOF1'),
        entry(5, 'SROOF1'),
      ],
      walls: [entry(10, 'WALL1'), entry(11, 'WALL1'), entry(12, 'WALL2')],
      roofs: [entry(15, 'ROOF1')],
      floors: [entry(30, 'FLOOR1'), entry(31, 'FLOOR2')],
      smartWalls: [entry(40, 'WALL1'), entry(41, 'WALL2')],
      smartDoors: [entry(42, 'DOOR1')],
      smartBrokenWalls: [entry(43, 'WALL1')],
      smartRoofs: [entry(44, 'ROOF1')],
    },
  };
}

describe('buildingModeReducer', () => {
  it('should select a wall, a floor and a roof for new rooms', () => {
    const state = reduce({ type: 'initSelections', palette: paletteEvent() });
    expect(state.selections.room).toEqual([
      { tileId: 1, count: 1 },
      { tileId: 3, count: 1 },
      { tileId: 4, count: 1 },
    ]);
    // The tiles start with the first wall
    expect(state.selections.tiles).toEqual([{ tileId: 10, count: 1 }]);
    // Selections are kept
    const kept = buildingModeReducer(
      reduce({ type: 'selectTile', palette: 'tiles', tileId: 30 }),
      { type: 'initSelections', palette: paletteEvent() },
    );
    expect(kept.selections.tiles).toEqual([{ tileId: 30, count: 1 }]);
  });

  it('should pick the first smart wall', () => {
    expect(INITIAL_BUILDING_MODE.smart).toBeNull();
    const state = reduce({ type: 'initSelections', palette: paletteEvent() });
    expect(state.smart).toEqual({ kind: 'tile', tileId: 40 });
    const door = buildingModeReducer(state, {
      type: 'setSmart',
      smart: { kind: 'tile', tileId: 42 },
    });
    expect(door.smart).toEqual({ kind: 'tile', tileId: 42 });
    const changed = buildingModeReducer(door, {
      type: 'setSmart',
      smart: { kind: 'hole' },
    });
    expect(changed.smart).toEqual({ kind: 'hole' });
    // Chosen values are kept
    expect(
      buildingModeReducer(changed, {
        type: 'initSelections',
        palette: paletteEvent(),
      }).smart,
    ).toEqual(changed.smart);
    expect(toBuildingMode(changed).smart).toEqual(changed.smart);
  });

  it('should keep room numbers in range', () => {
    expect(INITIAL_BUILDING_MODE.roomNumber).toBe(1);
    expect(reduce({ type: 'setRoomNumber', roomNumber: 12 }).roomNumber).toBe(
      12,
    );
    expect(reduce({ type: 'setRoomNumber', roomNumber: 300 }).roomNumber).toBe(
      255,
    );
    expect(reduce({ type: 'setRoomNumber', roomNumber: -2 }).roomNumber).toBe(
      0,
    );
  });

  it('should count the tiles of each palette', () => {
    const state = reduce(
      { type: 'addTile', palette: 'tiles', tileId: 10 },
      { type: 'addTile', palette: 'tiles', tileId: 10 },
      { type: 'addTile', palette: 'room', tileId: 3 },
      { type: 'removeTile', palette: 'room', tileId: 3 },
      { type: 'setSize', size: 99 },
    );
    expect(state.selections.tiles).toEqual([{ tileId: 10, count: 2 }]);
    expect(state.selections.room).toEqual([]);
    expect(state.size).toBe(15);
    expect(
      buildingModeReducer(state, { type: 'clearTiles', palette: 'tiles' })
        .selections.tiles,
    ).toEqual([]);
  });

  it('should make the mode of the viewer', () => {
    const state = reduce(
      { type: 'setTool', tool: 'tiles' },
      { type: 'setBrush', brush: 'line' },
      { type: 'setSize', size: 3 },
      { type: 'setErase', erase: true },
    );
    expect(toBuildingMode(state)).toEqual({
      mode: 'buildings',
      tool: 'tiles',
      erase: true,
      brush: { kind: 'line', width: 3 },
      selections: INITIAL_BUILDING_MODE.selections,
      smart: INITIAL_BUILDING_MODE.smart,
      roomNumber: 1,
    });
  });
});

describe('building tools', () => {
  it('should know their palettes, brushes and views', () => {
    expect(toolPalette('room')).toBe('room');
    expect(toolPalette('select')).toBe('room');
    expect(toolPalette('tiles')).toBe('tiles');
    expect(canErase('tiles')).toBe(true);
    expect(canErase('room')).toBe(true);
    expect(canErase('smart')).toBe(true);
    expect(isSmart('smart')).toBe(true);
    expect(isSmart('tiles')).toBe(false);
    expect(usesPalette('smart')).toBe(false);
    expect(usesPalette('tiles')).toBe(true);
    expect(toolPalette('smart')).toBe('room');
    expect(usesBrush('tiles', false)).toBe(false);
    expect(usesBrush('tiles', false, true)).toBe(true);
    expect(usesBrush('tiles', true)).toBe(true);
    expect(usesBrush('smart', false)).toBe(false);
    expect(usesBrush('smart', true)).toBe(true);
    expect(usesBrush('select', true)).toBe(false);
    expect(usesBrush('room', false)).toBe(true);
    expect(usesBrush('room', true)).toBe(true);
    expect(buildingViews('tiles', false)).toEqual({
      layers: { buildings: true, roomNumbers: false },
      floor: 'ground',
    });
    // Basements show the roofs, which are the rock
    expect(buildingViews('tiles', true)).toMatchObject({ floor: 'roof' });
    // Selected roofs show the roofs
    expect(buildingViews('tiles', false, true)).toMatchObject({
      floor: 'roof',
    });
    // Smart roofs show the roofs
    expect(buildingViews('smart', false)).toMatchObject({ floor: 'ground' });
    expect(buildingViews('smart', false, true)).toEqual({
      layers: { buildings: true, roomNumbers: false },
      floor: 'roof',
    });
    // Selecting buildings shows them from above
    expect(buildingViews('select', false)).toEqual({
      layers: { buildings: true, roomNumbers: true },
      floor: 'roof',
    });
    // The room numbers show the numbers and keep the buildings
    expect(buildingViews('roomNumber', false)).toEqual({
      layers: { roomNumbers: true },
      floor: 'ground',
    });
    expect(buildingViews('roomNumber', true)).toMatchObject({ floor: 'roof' });
    expect(usesBrush('roomNumber', false)).toBe(true);
    expect(usesPalette('roomNumber')).toBe(false);
  });

  it('should know the kinds of the selected tiles', () => {
    const palettes = paletteEvent().buildings;
    const selection = (...tileIds: number[]) =>
      tileIds.map((tileId) => ({ tileId, count: 1 }));
    expect(selectedKinds(selection(10, 30), palettes)).toEqual(
      new Set(['walls', 'floors']),
    );
    expect(selectedKinds(selection(15), palettes)).toEqual(new Set(['roofs']));
    expect(selectedKinds(selection(), palettes)).toEqual(new Set());
    expect(isFloorsOnly(selectedKinds(selection(30, 31), palettes))).toBe(true);
    expect(isFloorsOnly(selectedKinds(selection(10, 30), palettes))).toBe(
      false,
    );
    expect(isFloorsOnly(selectedKinds(selection(), palettes))).toBe(false);
  });

  it('should know whether the smart value is a roof', () => {
    const palettes = paletteEvent().buildings;
    expect(isSmartRoof({ kind: 'tile', tileId: 44 }, palettes)).toBe(true);
    expect(isSmartRoof({ kind: 'tile', tileId: 40 }, palettes)).toBe(false);
    expect(isSmartRoof({ kind: 'hole' }, palettes)).toBe(false);
    expect(isSmartRoof(null, palettes)).toBe(false);
  });

  it('should know when new rooms get slant roofs', () => {
    const entries = paletteEvent().buildings.room;
    const selection = (...tileIds: number[]) =>
      tileIds.map((tileId) => ({ tileId, count: 1 }));
    expect(hasSlantRoofOnly(selection(1, 3, 5), entries)).toBe(true);
    expect(hasSlantRoofOnly(selection(1, 4, 5), entries)).toBe(false);
    expect(hasSlantRoofOnly(selection(1, 3), entries)).toBe(false);
  });

  it('should only allow the tools of the map type', () => {
    expect(allowsTool('normal', 'room')).toBe(true);
    expect(allowsTool('normal', 'caves')).toBe(false);
    expect(allowsTool('basement', 'room')).toBe(true);
    expect(allowsTool('basement', 'smart')).toBe(true);
    expect(allowsTool('basement', 'caves')).toBe(false);
    expect(allowsTool('caves', 'caves')).toBe(true);
    expect(allowsTool('caves', 'room')).toBe(false);
    expect(allowsTool('caves', 'smart')).toBe(false);
    expect(allowsTool('caves', 'tiles')).toBe(true);
    expect(canErase('caves')).toBe(false);
    expect(canErase('room')).toBe(true);
    expect(canErase('select')).toBe(false);
    expect(canErase('doorLocks')).toBe(false);
    expect(usesPalette('doorLocks')).toBe(false);
    expect(allowsTool('caves', 'doorLocks')).toBe(true);
    expect(usesBrush('select', false)).toBe(false);
    expect(usesPalette('select')).toBe(false);
    expect(allowsTool('normal', 'select')).toBe(true);
    expect(allowsTool('basement', 'select')).toBe(true);
    expect(allowsTool('caves', 'select')).toBe(false);
    expect(usesBrush('caves', false)).toBe(true);
  });

  it('should group the palettes by their tile types', () => {
    const palettes = paletteEvent().buildings;
    expect(
      paletteGroups(palettes, 'room').map((g) => [g.label, g.entries.length]),
    ).toEqual([
      ['Walls', 2],
      ['Floors', 1],
      ['Roofs', 1],
      ['Slant roofs', 1],
    ]);
    // The tiles merge their palettes, grouped by kind and tile type
    expect(
      paletteGroups(palettes, 'tiles').map((g) => [g.label, g.entries.length]),
    ).toEqual([
      ['Walls · WALL1', 2],
      ['Walls · WALL2', 1],
      ['Roofs · ROOF1', 1],
      ['Floors · FLOOR1', 1],
      ['Floors · FLOOR2', 1],
    ]);
  });
});
