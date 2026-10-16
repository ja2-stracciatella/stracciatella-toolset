import { describe, expect, it, vi } from 'vitest';
import {
  parseEditorEvent,
  radarMapFile,
} from '../../../renderer/lib/mapViewer';
import { emptyBuildingPalettes } from '../test-utils/palette';
import {
  detailedPlacement,
  soldier,
  soldierEvent,
  soldierPaletteEvent,
} from '../test-utils/soldier';

const entry = {
  tileId: 1234,
  typeName: 'OFSTRUCT1',
  subIndex: 4,
  file: 'tilesets/0/trees.sti',
  subImage: 3,
};

describe('parseEditorEvent', () => {
  it('should accept palette and state events', () => {
    const palette = {
      type: 'palette',
      map: 'maps/A9.dat',
      tileset: 0,
      ground: [entry],
      roads: [],
      cliffs: [],
      debris: [],
      trees: [entry],
      rocks: [],
      misc: [],
      furniture: [],
      genericFurniture: [],
      buildings: emptyBuildingPalettes(),
    };
    expect(parseEditorEvent(palette)).toEqual(palette);

    const state = {
      type: 'state',
      map: 'maps/A9.dat',
      canUndo: true,
      undoDepth: 2,
      canRedo: false,
      redoDepth: 0,
      background: 'TEXTURE1',
      nextRoomNumber: 1,
      revision: 0,
      smoothing: 'normal',
      restrictedScroll: false,
    };
    expect(parseEditorEvent(state)).toEqual(state);
  });

  it('should accept door events', () => {
    const door = {
      type: 'door',
      map: 'maps/A9.dat',
      gridNo: 805,
      door: {
        lockId: 3,
        trap: { kind: 'silentAlarm', level: 2 },
        locked: true,
      },
    };
    expect(parseEditorEvent(door)).toEqual(door);
    expect(parseEditorEvent({ ...door, door: null })).toMatchObject({
      door: null,
    });
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(
      parseEditorEvent({
        ...door,
        door: { ...door.door, trap: { kind: 'lasers', level: 1 } },
      }),
    ).toBe(null);
    error.mockRestore();
  });

  it('should accept the events of the soldiers mode', () => {
    expect(parseEditorEvent(soldierPaletteEvent())).toEqual(
      soldierPaletteEvent(),
    );
    const selected = soldier({
      detailed: {
        ...detailedPlacement(),
        stats: { ...detailedPlacement().stats, life: 50 },
        inventory: [
          {
            item: 5,
            number: 1,
            data: [90, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            attachments: [{ item: 7, status: 60 }],
            flags: 0x81,
            mission: 0,
            trap: 0,
            imprintId: 200,
            weight: 0,
            used: 0,
          },
          ...detailedPlacement().inventory.slice(1),
        ],
      },
      patrol: [100, 200],
    });
    expect(parseEditorEvent(soldierEvent(selected))).toEqual(
      soldierEvent(selected),
    );
    expect(parseEditorEvent(soldierEvent(null))).toEqual(soldierEvent(null));
    const scheduled = soldierEvent(selected, {
      pick: { slot: 1, point: 'b', single: false },
    });
    expect(scheduled.schedule).toHaveLength(4);
    expect(parseEditorEvent(scheduled)).toEqual(scheduled);
    const notice = { type: 'notice', map: 'maps/A9.dat', message: 'Full' };
    expect(parseEditorEvent(notice)).toEqual(notice);

    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(
      parseEditorEvent(soldierEvent(soldier({ orders: 'dance' as never }))),
    ).toBe(null);
    error.mockRestore();
  });

  it('should accept the floor that the viewer switched to', () => {
    const roof = { type: 'floor', map: 'maps/A9.dat', floor: 'roof' };
    expect(parseEditorEvent(roof)).toEqual(roof);
    const ground = { ...roof, floor: 'ground' };
    expect(parseEditorEvent(ground)).toEqual(ground);

    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(parseEditorEvent({ ...roof, floor: 'basement' })).toBe(null);
    error.mockRestore();
  });

  it('should accept the events of the items mode', () => {
    const event = {
      type: 'item',
      map: 'maps/A9.dat',
      tile: 12345,
      items: [
        {
          id: 3,
          gridNo: 12345,
          onRoof: false,
          stack: {
            item: 234,
            number: 1,
            data: [100, 2, 137, 0, 52, 1, 3, 0, 0, 0, 0, 0],
            attachments: [],
            flags: 0x08,
            mission: 0,
            trap: 9,
            imprintId: 0,
            weight: 0,
            used: 0,
          },
          visibility: 'buried',
          flags: 0x40,
          nonExistChance: 0,
        },
      ],
      counts: {
        items: [{ item: 234, placed: 1, quantity: 1 }],
        triggers: Array.from({ length: 27 }, () => 0),
        keys: [{ lockId: 4, placed: 2 }],
      },
    };
    expect(parseEditorEvent(event)).toEqual(event);
    expect(parseEditorEvent({ ...event, tile: null, items: [] })).toEqual({
      ...event,
      tile: null,
      items: [],
    });
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(
      parseEditorEvent({
        ...event,
        items: [{ ...event.items[0], visibility: 'gone' }],
      }),
    ).toBe(null);
    error.mockRestore();
  });

  it('should accept the events of the lights mode', () => {
    const event = {
      type: 'light',
      map: 'maps/A9_B1.dat',
      light: {
        id: 3,
        gridNo: 12345,
        x: 25,
        y: 77,
        radius: 4,
        template: 'L-R04.LHT',
        time: 'primeTime',
      },
      count: 2,
      color: [10, 20, 30],
      ambient: { basement: true, caves: false, level: 6 },
      radii: [1, 2, 3, 4, 5, 6, 7, 8],
    };
    expect(parseEditorEvent(event)).toEqual(event);
    // No selected light, a template of another radius and a map above ground
    const other = {
      ...event,
      light: { ...event.light, radius: null, template: 'LAMP.LHT' },
      ambient: null,
    };
    expect(parseEditorEvent(other)).toEqual(other);
    expect(parseEditorEvent({ ...event, light: null })).toEqual({
      ...event,
      light: null,
    });
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(
      parseEditorEvent({
        ...event,
        light: { ...event.light, time: 'noon' },
      }),
    ).toBe(null);
    expect(parseEditorEvent({ ...event, color: [1, 2] })).toBe(null);
    error.mockRestore();
  });

  it('should accept the events of the traversal mode', () => {
    const event = {
      type: 'traversal',
      map: 'maps/A9.dat',
      exitGrids: 3,
      selected: { kind: 'entry', point: 'north', gridNo: 12000 },
      queried: {
        gridNo: 1234,
        destSector: { x: 9, y: 1, z: 1 },
        destGridNo: 4321,
      },
      entryPoints: {
        north: 12000,
        east: null,
        south: 13000,
        west: null,
        center: 12880,
        isolated: null,
      },
      edgePoints: {
        primary: { north: 290, east: 0, south: 300, west: 0 },
        secondary: { north: 0, east: 0, south: 0, west: 0 },
        missing: ['south'],
        pending: false,
      },
    };
    expect(parseEditorEvent(event)).toEqual(event);
    expect(parseEditorEvent({ ...event, queried: null })).toEqual({
      ...event,
      queried: null,
    });
    const selectedExitGrid = {
      ...event,
      selected: {
        kind: 'exitGrid',
        gridNo: 1234,
        destSector: { x: 9, y: 1, z: 1 },
        destGridNo: 4321,
      },
    };
    expect(parseEditorEvent(selectedExitGrid)).toEqual(selectedExitGrid);
    expect(parseEditorEvent({ ...event, selected: null })).toEqual({
      ...event,
      selected: null,
    });
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(
      parseEditorEvent({
        ...event,
        edgePoints: { ...event.edgePoints, missing: ['center'] },
      }),
    ).toBe(null);
    expect(
      parseEditorEvent({
        ...event,
        selected: { kind: 'entry', point: 'up', gridNo: 1 },
      }),
    ).toBe(null);
    expect(
      parseEditorEvent({
        ...event,
        entryPoints: { ...event.entryPoints, north: undefined },
      }),
    ).toBe(null);
    error.mockRestore();
  });

  it('should reject invalid events', () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(parseEditorEvent({ type: 'unknown', map: 'maps/A9.dat' })).toBe(
      null,
    );
    expect(
      parseEditorEvent({
        type: 'state',
        map: 'maps/A9.dat',
        canUndo: 'yes',
        undoDepth: 2,
        canRedo: false,
        redoDepth: 0,
        background: 'TEXTURE1',
        nextRoomNumber: 1,
        revision: 0,
        smoothing: 'normal',
        restrictedScroll: false,
      }),
    ).toBe(null);
    expect(parseEditorEvent(null)).toBe(null);
    expect(error).toHaveBeenCalledTimes(3);
    error.mockRestore();
  });
});

describe('radarMapFile', () => {
  it('should name the radar map like the map in lower case', () => {
    expect(radarMapFile('maps/A9.dat')).toBe('radarmaps/a9.sti');
    expect(radarMapFile('maps/A9_b1.dat')).toBe('radarmaps/a9_b1.sti');
    expect(radarMapFile('maps/j9_b1_a.dat')).toBe('radarmaps/j9_b1_a.sti');
  });
});
