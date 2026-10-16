import { describe, expect, it } from 'vitest';
import {
  effectiveBrush,
  INITIAL_TERRAIN_MODE,
  MAX_BRUSH_SIZE,
  MAX_TEXTURE_WEIGHT,
  objectGroups,
  TerrainModeAction,
  terrainModeReducer,
  TerrainModeState,
  toEditorMode,
} from '../../../../renderer/components/content/mapEditor/terrainMode';
import {
  EditorPaletteEvent,
  PaletteEntry,
} from '../../../../renderer/lib/mapViewer';
import { emptyBuildingPalettes } from '../../test-utils/palette';

function reduce(...actions: TerrainModeAction[]): TerrainModeState {
  return actions.reduce(terrainModeReducer, INITIAL_TERRAIN_MODE);
}

function entry(tileId: number): PaletteEntry {
  return {
    tileId,
    typeName: 'OFSTRUCT1',
    subIndex: 1,
    file: 'tilesets/0/trees.sti',
    subImage: 0,
  };
}

describe('terrainModeReducer', () => {
  it('should clamp the size and the density', () => {
    expect(INITIAL_TERRAIN_MODE.size).toBe(1);
    expect(reduce({ type: 'setSize', size: 4 }).size).toBe(4);
    expect(reduce({ type: 'setSize', size: 0 }).size).toBe(1);
    expect(reduce({ type: 'setSize', size: 40 }).size).toBe(MAX_BRUSH_SIZE);
    expect(reduce({ type: 'setSize', size: 2.6 }).size).toBe(3);
    expect(reduce({ type: 'setDensity', density: 35 }).density).toBe(35);
    expect(reduce({ type: 'setDensity', density: 0 }).density).toBe(1);
    expect(reduce({ type: 'setDensity', density: 120 }).density).toBe(100);
  });

  it('should weight the ground textures by their type name', () => {
    let state = reduce(
      { type: 'selectTexture', typeName: 'TEXTURE4' },
      { type: 'addTextureWeight', typeName: 'DEEP WATER' },
      { type: 'addTextureWeight', typeName: 'TEXTURE2' },
      { type: 'addTextureWeight', typeName: 'TEXTURE2' },
      { type: 'addTextureWeight', typeName: 'WATER1' },
      { type: 'removeTextureWeight', typeName: 'TEXTURE2' },
      { type: 'removeTextureWeight', typeName: 'WATER1' },
      { type: 'removeTextureWeight', typeName: 'TEXTURE6' },
    );
    expect(state.ground).toEqual({
      selected: 'WATER1',
      weights: [
        { typeName: 'DEEP WATER', weight: 1 },
        { typeName: 'TEXTURE2', weight: 1 },
      ],
    });

    for (let i = 0; i < 20; i++) {
      state = terrainModeReducer(state, {
        type: 'addTextureWeight',
        typeName: 'TEXTURE1',
      });
    }
    expect(state.ground.weights[2]).toEqual({
      typeName: 'TEXTURE1',
      weight: MAX_TEXTURE_WEIGHT,
    });

    // Selecting a texture paints only that one again
    state = terrainModeReducer(state, {
      type: 'selectTexture',
      typeName: 'TEXTURE3',
    });
    expect(state.ground).toEqual({ selected: 'TEXTURE3', weights: [] });
  });

  it('should count the selected tiles', () => {
    let state = reduce(
      { type: 'addTile', tileId: 5 },
      { type: 'addTile', tileId: 5 },
      { type: 'addTile', tileId: 9 },
      { type: 'removeTile', tileId: 9 },
      { type: 'removeTile', tileId: 11 },
      { type: 'addTile', tileId: 1 },
    );
    expect(state.objects).toEqual([
      { tileId: 5, count: 2 },
      { tileId: 1, count: 1 },
    ]);

    state = terrainModeReducer(state, { type: 'clearTiles' });
    expect(state.objects).toEqual([]);
  });

  it('should select a single tile', () => {
    const state = reduce(
      { type: 'addTile', tileId: 5 },
      { type: 'addTile', tileId: 9 },
      { type: 'selectTile', tileId: 7 },
    );
    expect(state.objects).toEqual([{ tileId: 7, count: 1 }]);
  });

  it('should select the first texture and object', () => {
    const palette: EditorPaletteEvent = {
      type: 'palette',
      map: 'maps/A9.dat',
      tileset: 0,
      ground: [{ ...entry(0), typeName: 'TEXTURE3' }],
      roads: [],
      cliffs: [],
      debris: [],
      trees: [entry(3)],
      rocks: [],
      misc: [entry(4)],
      furniture: [entry(5)],
      genericFurniture: [entry(6)],
      buildings: emptyBuildingPalettes(),
    };
    const state = reduce({ type: 'initSelections', palette });
    expect(state.ground.selected).toBe('TEXTURE3');
    expect(state.objects).toEqual([{ tileId: 3, count: 1 }]);
    // A selected texture and selected objects are kept
    const selected = terrainModeReducer(
      reduce(
        { type: 'selectTexture', typeName: 'TEXTURE6' },
        { type: 'addTile', tileId: 8 },
      ),
      { type: 'initSelections', palette },
    );
    expect(selected.ground.selected).toBe('TEXTURE6');
    expect(selected.objects).toEqual([{ tileId: 8, count: 1 }]);
  });

  it('should group the objects with the furniture last', () => {
    const palette: EditorPaletteEvent = {
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
      furniture: [entry(5)],
      genericFurniture: [entry(6)],
      buildings: emptyBuildingPalettes(),
    };
    const groups = objectGroups(palette);
    expect(groups.map((g) => g.label)).toEqual([
      'Debris',
      'Trees & bushes',
      'Rocks',
      'Misc junk',
      'Furniture',
      'Generic furniture',
    ]);
    expect(groups[4]?.entries).toEqual([entry(5)]);
    expect(groups[5]?.entries).toEqual([entry(6)]);
  });

  it('should build the modes of the viewer', () => {
    const state = reduce(
      { type: 'setErase', erase: true },
      { type: 'setBrush', brush: 'line' },
      { type: 'setSize', size: 20 },
      { type: 'selectRoad', road: 4 },
    );
    expect(toEditorMode(state, 'terrain')).toEqual({
      mode: 'terrain',
      tool: 'ground',
      erase: true,
      brush: { kind: 'line', width: MAX_BRUSH_SIZE },
      density: 2,
      ground: INITIAL_TERRAIN_MODE.ground,
      road: 4,
      cliff: 1,
    });
    expect(
      toEditorMode(
        terrainModeReducer(state, { type: 'setTool', tool: 'roads' }),
        'terrain',
      ),
    ).toMatchObject({ mode: 'terrain', tool: 'roads', erase: true, road: 4 });
    expect(
      toEditorMode(
        reduce(
          { type: 'setTool', tool: 'cliffs' },
          { type: 'selectCliff', cliff: 7 },
        ),
        'terrain',
      ),
    ).toMatchObject({ mode: 'terrain', tool: 'cliffs', cliff: 7 });
    expect(toEditorMode(state, 'objects')).toEqual({
      mode: 'objects',
      erase: true,
      brush: { kind: 'line', width: MAX_BRUSH_SIZE },
      density: 2,
      objects: [],
    });
    expect(
      toEditorMode(
        terrainModeReducer(state, { type: 'setBrush', brush: 'area' }),
        'objects',
      ),
    ).toMatchObject({ brush: { kind: 'area' } });
  });

  it('should send the size with the square and circle brushes', () => {
    const state = reduce({ type: 'setSize', size: 5 });
    const brush = (kind: 'square' | 'circle' | 'line' | 'area' | 'fill') =>
      toEditorMode(
        terrainModeReducer(state, { type: 'setBrush', brush: kind }),
        'terrain',
      );
    expect(brush('square')).toMatchObject({
      brush: { kind: 'square', size: 5 },
    });
    expect(brush('circle')).toMatchObject({
      brush: { kind: 'circle', size: 5 },
    });
    expect(brush('line')).toMatchObject({ brush: { kind: 'line', width: 5 } });
    expect(brush('area')).toMatchObject({ brush: { kind: 'area' } });
    expect(brush('fill')).toMatchObject({ brush: { kind: 'fill' } });
  });

  it('should switch the terrain tool', () => {
    expect(INITIAL_TERRAIN_MODE.tool).toBe('ground');
    expect(reduce({ type: 'setTool', tool: 'roads' }).tool).toBe('roads');
    expect(reduce({ type: 'setTool', tool: 'cliffs' }).tool).toBe('cliffs');
  });

  it('should only fill with the ground tool', () => {
    const state = reduce({ type: 'setBrush', brush: 'fill' });
    expect(toEditorMode(state, 'terrain')).toMatchObject({
      brush: { kind: 'fill' },
    });
    expect(toEditorMode(state, 'objects')).toMatchObject({
      brush: { kind: 'square', size: 1 },
    });
    expect(
      toEditorMode(
        terrainModeReducer(state, { type: 'setTool', tool: 'roads' }),
        'terrain',
      ),
    ).toMatchObject({ brush: { kind: 'square', size: 1 } });
    expect(
      toEditorMode(
        terrainModeReducer(state, { type: 'setTool', tool: 'cliffs' }),
        'terrain',
      ),
    ).toMatchObject({ brush: { kind: 'square', size: 1 } });
    expect(effectiveBrush('fill', false)).toBe('square');
    expect(effectiveBrush('fill', true)).toBe('fill');
    expect(effectiveBrush('circle', false)).toBe('circle');
  });
});
