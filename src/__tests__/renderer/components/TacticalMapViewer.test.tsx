import {
  act,
  fireEvent,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { Select } from 'antd';
import * as viewer from 'ja2-map-editor';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TacticalMapViewer } from '../../../renderer/components/content/TacticalMapViewer';
import { DEFAULT_RENDER_OPTIONS } from '../../../renderer/lib/mapViewer';
import { getInvokeMock } from '../test-utils/invoke';
import { renderWithTestProviders } from '../test-utils/render';
import { emptyBuildingPalettes } from '../test-utils/palette';
import {
  detailedPlacement,
  emptySchedule,
  soldier,
  soldierEvent,
  soldierPaletteEvent,
} from '../test-utils/soldier';

vi.mock('ja2-map-editor', () => ({
  default: vi.fn(() => Promise.resolve()),
  start: vi.fn(),
  load_map: vi.fn(),
  new_map: vi.fn(),
  tilesets: vi.fn(),
  set_active: vi.fn(),
  set_render_options: vi.fn(),
  set_editor_mode: vi.fn(),
  undo: vi.fn(),
  redo: vi.fn(),
  set_map_background: vi.fn(),
  set_map_smoothing: vi.fn(),
  set_restricted_scroll: vi.fn(),
  roof_all_rooms: vi.fn(),
  set_door: vi.fn(),
  set_soldier: vi.fn(),
  soldier_command: vi.fn(),
  set_item: vi.fn(),
  item_command: vi.fn(),
  set_light: vi.fn(),
  light_command: vi.fn(),
  set_traversal: vi.fn(),
  save_map: vi.fn(),
  key_event: vi.fn(),
  release_keys: vi.fn(),
}));

vi.mock('../../../renderer/components/content/StiPreview', () => ({
  StiPreview: () => null,
}));

function sentOptions(call: number): unknown {
  const json = vi.mocked(viewer.set_render_options).mock.calls[call]?.[0];
  return JSON.parse(json ?? 'null');
}

// The last render options sent to the viewer
function lastOptions(): unknown {
  return sentOptions(
    vi.mocked(viewer.set_render_options).mock.calls.length - 1,
  );
}

// An option of the floor selector
function floorOption(name: 'Ground' | 'Roof') {
  return within(screen.getByLabelText('floor')).getByRole('radio', { name });
}

// The viewer is only started once, so this is the status callback of all tests
function onStatus(status: unknown) {
  const callback = vi.mocked(viewer.start).mock.calls[0]?.[3];
  if (!callback) {
    throw new Error('The viewer was not started');
  }
  act(() => callback(status));
}

// The editor event callback of all tests
function onEditorEvent(event: unknown) {
  const callback = vi.mocked(viewer.start).mock.calls[0]?.[4];
  if (!callback) {
    throw new Error('The viewer was not started');
  }
  act(() => callback(event));
}

// The last mode sent to the viewer
function lastMode(): any {
  const calls = vi.mocked(viewer.set_editor_mode).mock.calls;
  return JSON.parse(calls[calls.length - 1]?.[0] ?? 'null');
}

// The soldier commands sent to the viewer
function soldierCommands(): unknown[] {
  return vi
    .mocked(viewer.soldier_command)
    .mock.calls.map(([json]) => JSON.parse(json));
}

// The soldiers sent to the viewer with `set_soldier`
function sentSoldiers(): any[] {
  return vi
    .mocked(viewer.set_soldier)
    .mock.calls.map(([json]) => JSON.parse(json));
}

function itemCommands(): unknown[] {
  return vi
    .mocked(viewer.item_command)
    .mock.calls.map(([json]) => JSON.parse(json));
}

function lightCommands(): unknown[] {
  return vi
    .mocked(viewer.light_command)
    .mock.calls.map(([json]) => JSON.parse(json));
}

// The lights sent to the viewer with `set_light`
function sentLights(): unknown[] {
  return vi
    .mocked(viewer.set_light)
    .mock.calls.map(([json]) => JSON.parse(json));
}

function lightEvent(overrides: Record<string, unknown> = {}) {
  return {
    type: 'light',
    map: 'maps/A9.dat',
    light: {
      id: 7,
      gridNo: 12345,
      x: 25,
      y: 77,
      radius: 3,
      template: 'L-R03.LHT',
      time: 'night',
    },
    count: 4,
    color: [10, 20, 30],
    ambient: { basement: true, caves: false, level: 6 },
    // The template of radius 8 is missing
    radii: [1, 2, 3, 4, 5, 6, 7],
    ...overrides,
  };
}

// The edits of the selection sent to the viewer with `set_traversal`
function traversalEdits(): unknown[] {
  return vi
    .mocked(viewer.set_traversal)
    .mock.calls.map(([json]) => JSON.parse(json));
}

function traversalEvent(overrides: Record<string, unknown> = {}) {
  return {
    type: 'traversal',
    map: 'maps/A9.dat',
    exitGrids: 3,
    selected: null,
    queried: null,
    entryPoints: {
      north: 12000,
      east: null,
      south: 13000,
      west: null,
      center: 12880,
      isolated: null,
    },
    edgePoints: {
      primary: { north: 290, east: 0, south: 0, west: 0 },
      secondary: { north: 0, east: 0, south: 0, west: 0 },
      missing: ['south'],
      pending: false,
    },
    ...overrides,
  };
}

// The items sent to the viewer with `set_item`
function sentItems(): any[] {
  return vi
    .mocked(viewer.set_item)
    .mock.calls.map(([json]) => JSON.parse(json));
}

// The item files, the profiles and the lock table of the items mode
function mockItemFiles(extraItems: unknown[] = []) {
  const invokeMock = getInvokeMock();
  const items: Record<string, unknown[]> = {
    'weapons.json': [
      {
        itemIndex: 2,
        internalName: 'GLOCK_18',
        internalType: 'M_PISTOL',
        ubMagSize: 15,
        attachment_Silencer: true,
      },
    ],
    'magazines.json': [
      { itemIndex: 71, internalName: 'CLIP9_15', capacity: 15, ubPerPocket: 8 },
    ],
    'armours.json': [],
    'explosives.json': [],
    'items.json': [
      { itemIndex: 207, internalName: 'SILENCER', usItemClass: 0x10000000 },
      { itemIndex: 219, internalName: 'MONEY', usItemClass: 0x20000000 },
      { itemIndex: 233, internalName: 'SWITCH', usItemClass: 0x10000000 },
      { itemIndex: 234, internalName: 'ACTION_ITEM', usItemClass: 0x10000000 },
      {
        itemIndex: 271,
        internalName: 'KEY_1',
        usItemClass: 0x10000,
        ubClassIndex: 0,
      },
      ...extraItems,
    ],
    'mercs-profile-info.json': [{ profileID: 57, internalName: 'KINGPIN' }],
  };
  for (const [file, vanilla] of Object.entries(items)) {
    invokeMock.resolve(
      'json/read',
      { file },
      { schema: {}, vanilla: vanilla as never, value: null, patch: null },
    );
  }
  // One lock, whose key is KEY_1
  const lock = Array.from({ length: 46 }, () => 0);
  [...'Front door'].forEach((char, i) => {
    lock[i] = char.charCodeAt(0);
  });
  invokeMock.resolve(
    'resources/read',
    { file: 'binarydata/locks.bin' },
    btoa(String.fromCharCode(...lock)),
  );
  return invokeMock;
}

// The selected tile with its items, `null` if none is selected
function itemEvent(items: Record<string, unknown>[] | null) {
  return {
    type: 'item',
    map: 'maps/A9.dat',
    tile: items && 12345,
    items: (items ?? []).map((item, index) => ({
      id: 4 + index,
      gridNo: 12345,
      onRoof: false,
      visibility: 'invisible',
      flags: 0,
      nonExistChance: 0,
      ...item,
    })),
    counts: {
      items: [{ item: 2, placed: 2, quantity: 2 }],
      triggers: Array.from({ length: 27 }, (_, slot) => (slot === 1 ? 1 : 0)),
      keys: [],
    },
  };
}

function itemStack(item: number, data: number[] = [80]) {
  return {
    item,
    number: 1,
    data: [...data, ...Array.from({ length: 12 - data.length }, () => 0)],
    attachments: [],
    flags: 0,
    mission: 0,
    trap: 0,
    imprintId: 0,
    weight: 0,
    used: 0,
  };
}

// Picks an option of an antd select
async function pickOption(select: string, option: string) {
  const combobox = await screen.findByLabelText(select, { selector: 'input' });
  fireEvent.mouseDown(combobox.closest('.ant-select-selector') ?? combobox);
  await waitFor(() => {
    expect(screen.getByTitle(option)).toBeTruthy();
  });
  fireEvent.click(screen.getByTitle(option));
}

// Clicks a mode of the editor, the layers have names like the modes
function selectMode(
  getByLabelText: (label: string) => HTMLElement,
  mode: string,
) {
  fireEvent.click(within(getByLabelText('mode')).getByText(mode));
}

// The form item of a control in the sidebar, by its label
function queryFormItem(label: string): HTMLElement | null {
  return screen.queryByText(label)?.closest('.ant-form-item') ?? null;
}

function formItem(label: string): HTMLElement {
  const item = queryFormItem(label);
  expect(item).not.toBeNull();
  return item!;
}

function formItemSlider(label: string): HTMLElement {
  return formItem(label).querySelector<HTMLElement>('[role="slider"]')!;
}

function stateEvent(map: string, revision: number) {
  return {
    type: 'state',
    map,
    canUndo: revision > 0,
    undoDepth: revision,
    canRedo: false,
    redoDepth: 0,
    background: 'TEXTURE1',
    nextRoomNumber: 1,
    smoothing: 'normal',
    restrictedScroll: false,
    revision,
  };
}

// The viewer loaded the map and sent its first state and light events, the sidebar is shown
function loadMap(map: string) {
  onStatus({ state: 'loaded', map });
  onEditorEvent(lightEvent({ map, light: null }));
  onEditorEvent(stateEvent(map, 0));
}

function paletteEntry(tileId: number, subIndex: number) {
  return {
    tileId,
    typeName: 'OFSTRUCT1',
    subIndex,
    file: 'tilesets/0/trees.sti',
    subImage: subIndex - 1,
  };
}

describe('TacticalMapViewer', () => {
  beforeEach(() => {
    vi.mocked(viewer.load_map).mockClear();
    vi.mocked(viewer.new_map).mockClear();
    vi.mocked(viewer.set_render_options).mockClear();
    vi.mocked(viewer.set_editor_mode).mockClear();
    vi.mocked(viewer.undo).mockClear();
    vi.mocked(viewer.redo).mockClear();
    vi.mocked(viewer.key_event).mockClear();
    vi.mocked(viewer.release_keys).mockClear();
    vi.mocked(viewer.save_map).mockReset();
    vi.mocked(viewer.set_soldier).mockClear();
    vi.mocked(viewer.soldier_command).mockClear();
    vi.mocked(viewer.set_item).mockClear();
    vi.mocked(viewer.item_command).mockClear();
    vi.mocked(viewer.set_light).mockClear();
    vi.mocked(viewer.light_command).mockClear();
    vi.mocked(viewer.set_traversal).mockClear();
  });

  it('should select, place and edit exit grids and entry points in the traversal mode', async () => {
    const {
      getByLabelText,
      getByText,
      findByText,
      queryByLabelText,
      queryByText,
    } = renderWithTestProviders(
      <TacticalMapViewer file="maps/A9.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/A9.dat');
    });
    loadMap('maps/A9.dat');
    selectMode(getByLabelText, 'Traversal');
    await waitFor(() => {
      expect(lastMode()).toEqual({
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
    onEditorEvent(traversalEvent());
    expect(await findByText('3 exit grids')).toBeInTheDocument();
    expect(getByText('Nothing selected')).toBeInTheDocument();
    expect(queryByLabelText('edge points')).toBeNull();
    expect(queryByText(/No edge points on the/)).toBeNull();

    // The selected exit grid, its destination is edited
    onEditorEvent(
      traversalEvent({
        selected: {
          kind: 'exitGrid',
          gridNo: 1234,
          destSector: { x: 9, y: 1, z: 0 },
          destGridNo: 4321,
        },
      }),
    );
    const selectedGrid = within(await findByText('Exit grid at tile 1234'));
    expect(selectedGrid).toBeDefined();
    expect(getByLabelText('selected destination sector')).toHaveValue('A9');
    fireEvent.change(getByLabelText('selected destination sector'), {
      target: { value: 'J11' },
    });
    const destGridNo = getByLabelText('selected destination grid number');
    fireEvent.change(destGridNo, { target: { value: '500' } });
    fireEvent.keyDown(destGridNo, { key: 'Enter' });
    await waitFor(() => {
      expect(traversalEdits()).toEqual([
        {
          kind: 'exitGrid',
          gridNo: 1234,
          destSector: { x: 11, y: 10, z: 0 },
          destGridNo: 4321,
        },
        {
          kind: 'exitGrid',
          gridNo: 1234,
          destSector: { x: 9, y: 1, z: 0 },
          destGridNo: 500,
        },
      ]);
    });

    // Nor with an exit grid selected
    expect(queryByLabelText('edge points')).toBeNull();

    // The selected entry point shows the edge points of its side
    vi.mocked(viewer.set_traversal).mockClear();
    onEditorEvent(
      traversalEvent({
        selected: { kind: 'entry', point: 'north', gridNo: 12000 },
      }),
    );
    const north = await findByText('North entry point');
    expect(north).toBeInTheDocument();
    expect(queryByLabelText('North entry point grid number')).toBeNull();
    expect(
      within(getByLabelText('edge points')).getByText('290'),
    ).toBeInTheDocument();
    expect(queryByText(/No edge points on the/)).toBeNull();

    // And warns when soldiers can't enter from its side
    onEditorEvent(
      traversalEvent({
        selected: { kind: 'entry', point: 'south', gridNo: 13000 },
      }),
    );
    expect(
      await findByText(
        "No edge points on the south side: soldiers can't enter the map from there.",
      ),
    ).toBeInTheDocument();

    // The center entry point has none of its own
    onEditorEvent(
      traversalEvent({
        selected: { kind: 'entry', point: 'center', gridNo: 12880 },
      }),
    );
    expect(
      await findByText(/has no edge points of its own/),
    ).toBeInTheDocument();
    expect(queryByLabelText('edge points')).toBeNull();
    expect(traversalEdits()).toEqual([]);

    // Placing exit grids, the destination of new ones
    fireEvent.click(
      within(getByLabelText('traversal tool')).getByText('Place'),
    );
    const sector = getByLabelText('destination sector');
    fireEvent.change(sector, { target: { value: 'Q3' } });
    fireEvent.change(sector, { target: { value: 'J11' } });
    await pickOption('destination level', 'B1');
    fireEvent.change(getByLabelText('destination grid number'), {
      target: { value: '500' },
    });
    fireEvent.click(within(getByLabelText('brush')).getByText('Area'));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        tool: 'place',
        place: 'exit',
        erase: false,
        brush: { kind: 'area' },
        destSector: { x: 11, y: 10, z: 1 },
        destGridNo: 500,
      });
    });
    // Circles and lines are not offered
    expect(within(getByLabelText('brush')).queryByText('Circle')).toBeNull();

    // A right click on an exit grid copies its destination
    onEditorEvent(
      traversalEvent({
        queried: {
          gridNo: 1234,
          destSector: { x: 9, y: 1, z: 2 },
          destGridNo: 4321,
        },
      }),
    );
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        destSector: { x: 9, y: 1, z: 2 },
        destGridNo: 4321,
      });
    });
    expect(getByLabelText('destination sector')).toHaveValue('A9');
    fireEvent.click(within(getByLabelText('erase')).getByText('Remove'));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ erase: true });
    });
    expect(queryByLabelText('destination sector')).toBeNull();

    // No edge points while placing
    expect(queryByLabelText('edge points')).toBeNull();

    // Only the entry points that are not set can be placed
    fireEvent.click(
      within(getByLabelText('what to place')).getByText('Entry point'),
    );
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ place: 'entry', entry: 'east' });
    });
    const toPlace = within(getByLabelText('entry point to place'));
    expect(toPlace.queryByText('North')).toBeNull();
    expect(toPlace.queryByText('Center')).toBeNull();
    fireEvent.click(toPlace.getByText('Isolated'));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ place: 'entry', entry: 'isolated' });
    });

    // Once all are set, only exit grids are placed
    onEditorEvent(
      traversalEvent({
        entryPoints: {
          north: 1,
          east: 2,
          south: 3,
          west: 4,
          center: 5,
          isolated: 6,
        },
      }),
    );
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ place: 'exit' });
    });
    expect(queryByLabelText('what to place')).toBeNull();
  });

  it('should place, erase and edit lights in the lights mode', async () => {
    const {
      getByLabelText,
      findByLabelText,
      getByText,
      queryByLabelText,
      queryByText,
    } = renderWithTestProviders(
      <TacticalMapViewer file="maps/A9.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/A9.dat');
    });
    loadMap('maps/A9.dat');
    selectMode(getByLabelText, 'Lights');
    await waitFor(() => {
      expect(lastMode()).toEqual({
        mode: 'lights',
        tool: 'select',
        erase: false,
        brush: { kind: 'square', size: 1 },
        radius: 5,
        time: 'primeTime',
      });
    });
    expect(queryByLabelText('radius of new lights')).toBeNull();

    // The radius and time of new lights
    fireEvent.click(within(getByLabelText('light tool')).getByText('Place'));
    await pickOption('radius of new lights', '3');
    fireEvent.click(
      within(getByLabelText('time of new lights')).getByText('24h'),
    );
    await waitFor(() => {
      expect(lastMode()).toEqual({
        mode: 'lights',
        tool: 'place',
        erase: false,
        brush: { kind: 'square', size: 1 },
        radius: 3,
        time: 'always',
      });
    });
    // Placing has no selection
    onEditorEvent(lightEvent());
    expect(queryByText('Selected light')).toBeNull();
    expect(queryByText('4 lights')).toBeNull();

    // Erasing uses the brush instead of the new light
    fireEvent.click(within(getByLabelText('erase')).getByText('Remove'));
    fireEvent.click(within(getByLabelText('brush')).getByText('Circle'));
    expect(queryByText('New light')).toBeNull();
    await waitFor(() => {
      expect(lastMode()).toEqual({
        mode: 'lights',
        tool: 'place',
        erase: true,
        brush: { kind: 'circle', size: 1 },
        radius: 3,
        time: 'always',
      });
    });

    // The selected light, with the radii whose templates exist
    fireEvent.click(within(getByLabelText('light tool')).getByText('Select'));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ tool: 'select' });
    });
    expect(queryByLabelText('erase')).toBeNull();
    expect(queryByLabelText('brush')).toBeNull();
    const selected = within(await findByLabelText('selected light'));
    expect(selected.getByText('L-R03.LHT')).toBeInTheDocument();
    expect(
      selected.getByText('Tile 12345 (25, 77), Night'),
    ).toBeInTheDocument();
    expect(getByText('4 lights')).toBeInTheDocument();
    fireEvent.click(
      within(selected.getByLabelText('time of the selected light')).getByText(
        'Prime',
      ),
    );
    const radius = await screen.findByLabelText(
      'radius of the selected light',
      {
        selector: 'input',
      },
    );
    fireEvent.mouseDown(radius.closest('.ant-select-selector') ?? radius);
    await waitFor(() => {
      expect(screen.getByTitle('6')).toBeInTheDocument();
    });
    // Radii without a template can't be chosen
    expect(screen.getByTitle('8')).toHaveClass(
      'ant-select-item-option-disabled',
    );
    fireEvent.click(screen.getByTitle('6'));
    await waitFor(() => {
      expect(sentLights()).toEqual([
        { id: 7, radius: 3, time: 'primeTime' },
        { id: 7, radius: 6, time: 'night' },
      ]);
    });

    // The selected light is removed with the Delete key
    expect(queryByText('Delete')).toBeNull();
    expect(queryByText('Next light')).toBeNull();
    // The lighting of the map is set in the view mode
    expect(queryFormItem('Light color')).toBeNull();
    expect(queryFormItem('Underground Light Level')).toBeNull();
    selectMode(getByLabelText, 'Overview');
    // The light level of the basement, with the light colour
    const levelSlider = formItemSlider('Underground Light Level');
    expect(levelSlider).toHaveAttribute('aria-valuenow', '6');
    fireEvent.keyDown(levelSlider, { key: 'End', keyCode: 35 });
    fireEvent.keyUp(levelSlider, { key: 'End', keyCode: 35 });
    await waitFor(() => {
      expect(lightCommands()).toEqual([
        { kind: 'settings', color: [10, 20, 30], ambientLevel: 15 },
      ]);
    });
    // The new level is shown until the map reports it, then the map's level
    expect(levelSlider).toHaveAttribute('aria-valuenow', '15');
    onEditorEvent(
      lightEvent({ ambient: { basement: true, caves: false, level: 15 } }),
    );
    expect(levelSlider).toHaveAttribute('aria-valuenow', '15');
    onEditorEvent(lightEvent());
    expect(levelSlider).toHaveAttribute('aria-valuenow', '6');
    // The light colour keeps the level
    fireEvent.click(
      formItem('Light color').querySelector('.ant-color-picker-trigger')!,
    );
    await waitFor(() => {
      expect(
        document.querySelector('.ant-color-picker-rgb-input input'),
      ).toBeTruthy();
    });
    const red = document.querySelector('.ant-color-picker-rgb-input input');
    fireEvent.change(red!, { target: { value: '200' } });
    await waitFor(() => {
      expect(lightCommands()[1]).toEqual({
        kind: 'settings',
        color: [200, 20, 30],
        ambientLevel: 6,
      });
    });

    // Maps above ground have no light level
    onEditorEvent(lightEvent({ light: null, ambient: null }));
    await waitFor(() => {
      expect(queryFormItem('Underground Light Level')).toBeNull();
    });
    expect(queryFormItem('Light color')).not.toBeNull();
    selectMode(getByLabelText, 'Lights');
    expect(getByText('No light selected')).toBeInTheDocument();
    expect(queryByText('L-R03.LHT')).toBeNull();
  });

  it('should send the render options after loading the map and when they change', async () => {
    const { getByRole, getByText } = renderWithTestProviders(
      <TacticalMapViewer file="maps/A9.dat" onBack={() => {}} />,
    );

    await waitFor(() => {
      expect(viewer.set_render_options).toHaveBeenCalledTimes(1);
    });
    expect(viewer.load_map).toHaveBeenCalledWith('maps/A9.dat');
    const [loadOrder] = vi.mocked(viewer.load_map).mock.invocationCallOrder;
    const [optionsOrder] = vi.mocked(viewer.set_render_options).mock
      .invocationCallOrder;
    expect(loadOrder).toBeLessThan(optionsOrder ?? 0);
    expect(sentOptions(0)).toEqual(DEFAULT_RENDER_OPTIONS);
    loadMap('maps/A9.dat');
    // The map type is set in the view mode
    expect(queryFormItem('Map Type')).not.toBeNull();

    const buildings = getByRole('switch', { name: 'Buildings' });
    expect(buildings).toBeChecked();
    fireEvent.click(buildings);
    expect(buildings).not.toBeChecked();
    await waitFor(() => {
      expect(viewer.set_render_options).toHaveBeenCalledTimes(2);
    });
    expect(sentOptions(1)).toEqual({
      ...DEFAULT_RENDER_OPTIONS,
      layers: { ...DEFAULT_RENDER_OPTIONS.layers, buildings: false },
    });

    fireEvent.click(getByText('Night'));
    await waitFor(() => {
      expect(viewer.set_render_options).toHaveBeenCalledTimes(3);
    });
    expect(sentOptions(2)).toEqual({
      lighting: 'night',
      floor: 'ground',
      layers: { ...DEFAULT_RENDER_OPTIONS.layers, buildings: false },
    });

    // The floor shows the roofs
    expect(floorOption('Ground')).toBeChecked();
    fireEvent.click(floorOption('Roof'));
    await waitFor(() => {
      expect(viewer.set_render_options).toHaveBeenCalledTimes(4);
    });
    expect(sentOptions(3)).toMatchObject({ floor: 'roof', lighting: 'night' });
    expect(floorOption('Roof')).toBeChecked();
  });

  it('should show and hide the teams of the soldiers layer', async () => {
    const { getByLabelText, getByRole, queryByLabelText } =
      renderWithTestProviders(
        <TacticalMapViewer file="maps/A9.dat" onBack={() => {}} />,
      );
    await waitFor(() => {
      expect(viewer.set_render_options).toHaveBeenCalledTimes(1);
    });
    loadMap('maps/A9.dat');

    // The teams are shown once the soldiers layer is expanded
    const expand = getByRole('button', { name: 'Soldier teams' });
    expect(expand).toHaveAttribute('aria-expanded', 'false');
    expect(queryByLabelText('soldier teams')).toBeNull();
    fireEvent.click(expand);
    expect(expand).toHaveAttribute('aria-expanded', 'true');
    const teams = getByLabelText('soldier teams');
    expect(
      within(teams)
        .getAllByRole('switch')
        .map((toggle) => toggle.getAttribute('aria-label')),
    ).toEqual(['Enemies', 'Creatures', 'Rebels', 'Civilians']);

    const enemies = within(teams).getByRole('switch', { name: 'Enemies' });
    expect(enemies).toBeChecked();
    fireEvent.click(enemies);
    expect(enemies).not.toBeChecked();
    await waitFor(() => {
      expect(viewer.set_render_options).toHaveBeenCalledTimes(2);
    });
    expect(sentOptions(1)).toEqual({
      ...DEFAULT_RENDER_OPTIONS,
      layers: {
        ...DEFAULT_RENDER_OPTIONS.layers,
        soldierTeams: {
          enemy: false,
          creature: true,
          rebel: true,
          civilian: true,
        },
      },
    });
    // Hiding the whole layer keeps the teams
    fireEvent.click(getByRole('switch', { name: 'Soldiers' }));
    await waitFor(() => {
      expect(viewer.set_render_options).toHaveBeenCalledTimes(3);
    });
    expect(sentOptions(2)).toMatchObject({
      layers: { soldiers: false, soldierTeams: { enemy: false, rebel: true } },
    });

    fireEvent.click(expand);
    expect(queryByLabelText('soldier teams')).toBeNull();
  });

  it('should follow the floor that Page Up and Page Down switch in the viewer', async () => {
    const { getByLabelText, findByText, queryByText } = renderWithTestProviders(
      <TacticalMapViewer file="maps/A9.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/A9.dat');
    });
    loadMap('maps/A9.dat');
    selectMode(getByLabelText, 'Terrain');
    expect(queryByText('These tools only edit the ground floor.')).toBeNull();

    onEditorEvent({ type: 'floor', map: 'maps/A9.dat', floor: 'roof' });
    await waitFor(() => {
      expect(floorOption('Roof')).toBeChecked();
    });
    await waitFor(() => {
      expect(lastOptions()).toMatchObject({ floor: 'roof' });
    });
    // The terrain tools don't edit the roofs
    expect(
      await findByText('These tools only edit the ground floor.'),
    ).toBeInTheDocument();

    // Events of other maps are ignored
    onEditorEvent({ type: 'floor', map: 'maps/B13.dat', floor: 'ground' });
    expect(floorOption('Roof')).toBeChecked();
    onEditorEvent({ type: 'floor', map: 'maps/A9.dat', floor: 'ground' });
    await waitFor(() => {
      expect(floorOption('Ground')).toBeChecked();
    });
    await waitFor(() => {
      expect(lastOptions()).toMatchObject({ floor: 'ground' });
    });
  });

  it('should show a loading overlay until the map is loaded', async () => {
    const { queryByRole } = renderWithTestProviders(
      <TacticalMapViewer file="maps/B13.dat" onBack={() => {}} />,
    );

    expect(queryByRole('status', { name: 'loading map' })).toBeInTheDocument();
    expect(queryByRole('complementary', { name: 'map editor' })).toBeNull();
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/B13.dat');
    });

    // Statuses of other maps are ignored
    onStatus({ state: 'loaded', map: 'maps/A9.dat' });
    expect(queryByRole('status', { name: 'loading map' })).toBeInTheDocument();

    onStatus({ state: 'loaded', map: 'maps/B13.dat' });
    expect(
      queryByRole('status', { name: 'loading map' }),
    ).not.toBeInTheDocument();
    // The sidebar waits for the first state event of the map
    expect(queryByRole('complementary', { name: 'map editor' })).toBeNull();
    onEditorEvent(stateEvent('maps/A9.dat', 0));
    expect(queryByRole('complementary', { name: 'map editor' })).toBeNull();
    loadMap('maps/B13.dat');
    expect(
      queryByRole('complementary', { name: 'map editor' }),
    ).toBeInTheDocument();

    onStatus({ state: 'loading', map: 'maps/B13.dat' });
    expect(queryByRole('status', { name: 'loading map' })).toBeInTheDocument();

    onStatus({ state: 'error', map: 'maps/B13.dat', message: 'broken' });
    expect(
      queryByRole('status', { name: 'loading map' }),
    ).not.toBeInTheDocument();
  });

  it('should send the terrain modes', async () => {
    const { getByLabelText, getByRole, queryByLabelText, queryByRole } =
      renderWithTestProviders(
        <TacticalMapViewer file="maps/C5.dat" onBack={() => {}} />,
      );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/C5.dat');
    });
    loadMap('maps/C5.dat');
    await waitFor(() => {
      expect(lastMode()).toEqual({ mode: 'view' });
    });

    selectMode(getByLabelText, 'Terrain');
    await waitFor(() => {
      expect(lastMode()).toEqual({
        mode: 'terrain',
        tool: 'ground',
        erase: false,
        brush: { kind: 'square', size: 1 },
        density: 2,
        ground: expect.anything(),
        road: 0,
        cliff: 1,
      });
    });
    // Only the terrain fills
    fireEvent.click(within(getByLabelText('brush')).getByText('Fill'));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ brush: { kind: 'fill' } });
    });
    selectMode(getByLabelText, 'Objects');
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        mode: 'objects',
        erase: false,
        brush: { kind: 'square', size: 1 },
      });
    });
    expect(within(getByLabelText('brush')).queryByText('Fill')).toBe(null);
    selectMode(getByLabelText, 'Terrain');
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ brush: { kind: 'fill' } });
    });
    // The size applies to the square, circle and line brushes, the density to the objects
    expect(queryByLabelText('size')).toBe(null);
    expect(queryByRole('slider', { name: 'density' })).toBe(null);
    fireEvent.click(within(getByLabelText('brush')).getByText('Circle'));
    fireEvent.keyDown(getByLabelText('size'), {
      key: 'ArrowRight',
      keyCode: 39,
    });
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ brush: { kind: 'circle', size: 2 } });
    });
    // Roads only use the clicked tile, the brush controls are hidden
    fireEvent.click(within(getByLabelText('terrain tool')).getByText('Roads'));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        mode: 'terrain',
        tool: 'roads',
        erase: false,
        brush: { kind: 'circle', size: 2 },
        road: 0,
      });
    });
    expect(queryByLabelText('brush')).toBe(null);
    expect(queryByRole('button', { name: /Set map background/ })).toBe(null);
    // Cliffs are picked from their palette by their sub index
    onEditorEvent({
      type: 'palette',
      map: 'maps/C5.dat',
      tileset: 0,
      ground: [],
      roads: [],
      cliffs: [2, 3, 5].map((subIndex) => ({
        ...paletteEntry(200 + subIndex, subIndex),
        typeName: 'FIRSTCLIFF',
        file: 'tilesets/0/cliffs.sti',
      })),
      debris: [],
      trees: [],
      rocks: [],
      misc: [],
      furniture: [],
      genericFurniture: [],
      buildings: emptyBuildingPalettes(),
    });
    fireEvent.click(within(getByLabelText('terrain tool')).getByText('Cliffs'));
    fireEvent.click(getByRole('button', { name: 'cliffs 5 (FIRSTCLIFF)' }));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        mode: 'terrain',
        tool: 'cliffs',
        cliff: 5,
      });
    });
    expect(
      getByRole('button', { name: 'cliffs 5 (FIRSTCLIFF)' }),
    ).toHaveAttribute('aria-pressed', 'true');
    expect(queryByLabelText('brush')).toBe(null);
    // The objects mode has no tools
    selectMode(getByLabelText, 'Objects');
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ mode: 'objects' });
    });
    expect(queryByLabelText('terrain tool')).toBe(null);
    selectMode(getByLabelText, 'Overview');
    await waitFor(() => {
      expect(lastMode()).toEqual({ mode: 'view' });
    });
    // Roads are a tool of the terrain mode
    expect(within(getByLabelText('mode')).queryByText('Roads')).toBe(null);
  });

  it('should undo and redo with the buttons next to the save button', async () => {
    const { getByRole } = renderWithTestProviders(
      <TacticalMapViewer file="maps/C5.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/C5.dat');
    });
    loadMap('maps/C5.dat');

    const undoButton = getByRole('button', { name: 'Undo' });
    const redoButton = getByRole('button', { name: 'Redo' });
    expect(undoButton).toBeDisabled();
    expect(redoButton).toBeDisabled();
    // Events of other maps are ignored
    onEditorEvent({
      ...stateEvent('maps/A9.dat', 0),
      canUndo: true,
      undoDepth: 1,
      canRedo: true,
      redoDepth: 1,
    });
    expect(undoButton).toBeDisabled();
    expect(redoButton).toBeDisabled();
    onEditorEvent({
      ...stateEvent('maps/C5.dat', 0),
      canUndo: true,
      undoDepth: 1,
    });
    expect(undoButton).toBeEnabled();
    expect(redoButton).toBeDisabled();
    fireEvent.click(undoButton);
    await waitFor(() => {
      expect(viewer.undo).toHaveBeenCalledTimes(1);
    });

    onEditorEvent({
      ...stateEvent('maps/C5.dat', 1),
      canUndo: false,
      undoDepth: 0,
      canRedo: true,
      redoDepth: 1,
    });
    expect(undoButton).toBeDisabled();
    expect(redoButton).toBeEnabled();
    fireEvent.click(redoButton);
    await waitFor(() => {
      expect(viewer.redo).toHaveBeenCalledTimes(1);
    });
    expect(viewer.undo).toHaveBeenCalledTimes(1);
  });

  it('should send the buildings mode and the views of its tools', async () => {
    const { getByLabelText, getByRole, getByText } = renderWithTestProviders(
      <TacticalMapViewer file="maps/D3.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/D3.dat');
    });
    loadMap('maps/D3.dat');
    onEditorEvent({
      type: 'palette',
      map: 'maps/D3.dat',
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
          { ...paletteEntry(1, 1), typeName: 'WALL1' },
          { ...paletteEntry(2, 1), typeName: 'FLOOR1' },
          { ...paletteEntry(3, 1), typeName: 'ROOF1' },
        ],
        smartRoofs: [{ ...paletteEntry(4, 10), typeName: 'ROOF1' }],
      },
    });
    // Show the roofs, the rooms hide them again
    fireEvent.click(floorOption('Roof'));
    selectMode(getByLabelText, 'Buildings');
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        mode: 'buildings',
        tool: 'room',
        brush: { kind: 'area' },
        selections: {
          room: [
            { tileId: 1, count: 1 },
            { tileId: 2, count: 1 },
            { tileId: 3, count: 1 },
          ],
        },
      });
    });
    await waitFor(() => {
      expect(floorOption('Ground')).toBeChecked();
    });
    expect(lastOptions()).toMatchObject({ floor: 'ground' });
    expect(getByRole('switch', { name: 'Buildings' })).toBeChecked();
    expect(
      within(getByRole('group', { name: 'Walls' })).getByTitle(
        'trees 1 (WALL1)',
      ),
    ).toBeTruthy();

    // Smart roofs show the roofs
    const tools = getByLabelText('tool');
    fireEvent.click(within(tools).getByText('Smart'));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ tool: 'smart' });
    });
    fireEvent.click(
      within(getByRole('group', { name: 'Roofs' })).getByTitle(
        'trees 10 (ROOF1)',
      ),
    );
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        tool: 'smart',
        smart: { kind: 'tile', tileId: 4 },
      });
    });
    await waitFor(() => {
      expect(floorOption('Roof')).toBeChecked();
    });
    expect(lastOptions()).toMatchObject({ floor: 'roof' });

    // Room numbers show the numbers and start at the next free number
    fireEvent.click(within(tools).getByText('Room numbers'));
    await waitFor(() => {
      expect(getByRole('switch', { name: 'Room numbers' })).toBeChecked();
    });
    onEditorEvent({
      type: 'state',
      map: 'maps/D3.dat',
      canUndo: false,
      undoDepth: 0,
      canRedo: false,
      redoDepth: 0,
      background: 'TEXTURE1',
      nextRoomNumber: 6,
      revision: 0,
      smoothing: 'normal',
      restrictedScroll: false,
    });
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        tool: 'roomNumber',
        roomNumber: 6,
      });
    });
    expect(getByRole('spinbutton', { name: 'room number' })).toHaveValue('6');

    // Cave maps only have the caves tool
    onEditorEvent({
      type: 'state',
      map: 'maps/D3.dat',
      canUndo: false,
      undoDepth: 0,
      canRedo: false,
      redoDepth: 0,
      background: 'TEXTURE1',
      nextRoomNumber: 6,
      revision: 0,
      smoothing: 'caves',
      restrictedScroll: false,
    });
    // Room numbers are allowed on cave maps
    expect(lastMode()).toMatchObject({ tool: 'roomNumber' });
    await waitFor(() => {
      expect(within(getByLabelText('tool')).getByText('Caves')).toBeTruthy();
    });
    expect(within(getByLabelText('tool')).queryByText('New room')).toBe(null);

    // The map type is set in the view mode, basements can roof all rooms
    expect(queryFormItem('Map Type')).toBe(null);
    selectMode(getByLabelText, 'Overview');
    fireEvent.click(within(formItem('Map Type')).getByText('Basement'));
    await waitFor(() => {
      expect(viewer.set_map_smoothing).toHaveBeenCalledWith('basement');
    });
    onEditorEvent({
      type: 'state',
      map: 'maps/D3.dat',
      canUndo: true,
      undoDepth: 1,
      canRedo: false,
      redoDepth: 0,
      background: 'TEXTURE1',
      nextRoomNumber: 6,
      revision: 0,
      smoothing: 'basement',
      restrictedScroll: false,
    });
    fireEvent.click(getByText('Roof all rooms'));
    await waitFor(() => {
      expect(viewer.roof_all_rooms).toHaveBeenCalledTimes(1);
    });

    // The scroll limit is independent of the map type
    const scrollLimit = formItem('Scroll Limit');
    expect(
      within(scrollLimit).getByRole('radio', { name: 'Whole map' }),
    ).toBeChecked();
    fireEvent.click(within(scrollLimit).getByText('Basement area'));
    await waitFor(() => {
      expect(viewer.set_restricted_scroll).toHaveBeenCalledWith(true);
    });
    onEditorEvent({
      type: 'state',
      map: 'maps/D3.dat',
      canUndo: true,
      undoDepth: 2,
      canRedo: false,
      redoDepth: 0,
      background: 'TEXTURE1',
      nextRoomNumber: 6,
      revision: 0,
      smoothing: 'basement',
      restrictedScroll: true,
    });
    await waitFor(() => {
      expect(
        within(formItem('Scroll Limit')).getByRole('radio', {
          name: 'Basement area',
        }),
      ).toBeChecked();
    });
  });

  it('should show the roofs when the tiles have a roof selected', async () => {
    const { getByLabelText, getByRole } = renderWithTestProviders(
      <TacticalMapViewer file="maps/D3.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/D3.dat');
    });
    loadMap('maps/D3.dat');
    onEditorEvent({
      type: 'palette',
      map: 'maps/D3.dat',
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
        walls: [{ ...paletteEntry(4, 2), typeName: 'WALL1' }],
        roofs: [{ ...paletteEntry(5, 3), typeName: 'ROOF1' }],
      },
    });
    selectMode(getByLabelText, 'Buildings');
    // The tiles start with the first wall
    fireEvent.click(within(getByLabelText('tool')).getByText('Tiles'));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        tool: 'tiles',
        selections: { tiles: [{ tileId: 4, count: 1 }] },
      });
    });
    await waitFor(() => {
      expect(floorOption('Ground')).toBeChecked();
    });
    fireEvent.click(
      within(getByRole('group', { name: 'Roofs · ROOF1' })).getByRole(
        'button',
        { name: 'trees 3 (ROOF1)' },
      ),
    );
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        selections: { tiles: [{ tileId: 5, count: 1 }] },
      });
    });
    await waitFor(() => {
      expect(floorOption('Roof')).toBeChecked();
    });
    await waitFor(() => {
      expect(lastOptions()).toMatchObject({ floor: 'roof' });
    });
  });

  it('should edit the doors the viewer asks for', async () => {
    const { findByText, getByText } = renderWithTestProviders(
      <TacticalMapViewer file="maps/D4.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/D4.dat');
    });
    loadMap('maps/D4.dat');
    onEditorEvent({
      type: 'door',
      map: 'maps/D4.dat',
      gridNo: 805,
      door: null,
    });
    expect(await findByText('Door lock')).toBeInTheDocument();
    fireEvent.click(getByText('Locked'));
    fireEvent.click(getByText('Okay'));
    await waitFor(() => {
      expect(viewer.set_door).toHaveBeenCalledTimes(1);
    });
    expect(
      JSON.parse(vi.mocked(viewer.set_door).mock.calls[0]?.[0] ?? 'null'),
    ).toEqual({
      gridNo: 805,
      door: { lockId: 0, trap: null, locked: true },
    });
  });

  it('should place, erase and select soldiers in the soldiers mode', async () => {
    const {
      getByLabelText,
      getByRole,
      findByText,
      getByText,
      queryByLabelText,
      queryByRole,
      queryByText,
    } = renderWithTestProviders(
      <TacticalMapViewer file="maps/A9.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/A9.dat');
    });
    loadMap('maps/A9.dat');
    onEditorEvent(soldierPaletteEvent());
    selectMode(getByLabelText, 'Soldiers');
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        mode: 'soldiers',
        tool: 'select',
        erase: false,
        brush: { kind: 'square', size: 1 },
        team: 'enemy',
      });
    });
    // Selecting has no team or erase controls
    expect(queryByLabelText('team')).toBeNull();
    expect(queryByLabelText('erase')).toBeNull();
    expect(getByText('No soldier selected')).toBeInTheDocument();

    // The team of new soldiers
    fireEvent.click(within(getByLabelText('soldier tool')).getByText('Place'));
    fireEvent.click(within(getByLabelText('team')).getByText(/Civilians/));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ tool: 'place', team: 'civilian' });
    });
    // The shown teams are part of the layers
    expect(queryByLabelText('shown teams')).toBeNull();
    expect(lastMode()).not.toHaveProperty('visibleTeams');
    expect(queryByText('No soldier selected')).toBeNull();

    // The floor decides where soldiers go, the soldiers mode has no roof switch
    expect(queryByRole('switch', { name: 'on roofs' })).toBeNull();
    expect(lastMode()).not.toHaveProperty('onRoof');
    // The soldier tools edit the roofs
    fireEvent.click(floorOption('Roof'));
    await waitFor(() => {
      expect(lastOptions()).toMatchObject({ floor: 'roof' });
    });
    expect(queryByText('These tools only edit the ground floor.')).toBeNull();

    // Erasing uses the brush instead of the team
    fireEvent.click(within(getByLabelText('erase')).getByText('Remove'));
    fireEvent.click(within(getByLabelText('brush')).getByText('Circle'));
    expect(queryByLabelText('team')).toBeNull();
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        tool: 'place',
        erase: true,
        brush: { kind: 'circle', size: 1 },
      });
    });

    // Selecting an enemy switches to its team
    fireEvent.click(within(getByLabelText('soldier tool')).getByText('Select'));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ tool: 'select' });
    });
    expect(queryByLabelText('brush')).toBeNull();
    onEditorEvent(soldierEvent(soldier({ onRoof: true })));
    expect(await findByText('Slot #20')).toBeInTheDocument();
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ team: 'enemy' });
    });
    expect(queryByRole('button', { name: 'Delete' })).toBeNull();
    expect(queryByRole('button', { name: 'Next' })).toBeNull();
    expect(queryByRole('button', { name: 'Copy' })).toBeNull();
    expect(queryByRole('button', { name: 'Paste' })).toBeNull();
    fireEvent.click(getByRole('button', { name: 'Find' }));
    await waitFor(() => {
      expect(soldierCommands()).toEqual([{ kind: 'center', gridNo: 12345 }]);
    });

    onEditorEvent({
      type: 'notice',
      map: 'maps/A9.dat',
      message: "A soldier can't stand there",
    });
    expect(await findByText("A soldier can't stand there")).toBeInTheDocument();
    onEditorEvent(soldierEvent(null));
    expect(await findByText('No soldier selected')).toBeInTheDocument();
    expect(queryByText('Slot #20')).toBe(null);
  });

  it('should edit the selected soldier in the general tab', async () => {
    const {
      getByLabelText,
      getAllByLabelText,
      findByText,
      getByText,
      queryByLabelText,
      queryByText,
    } = renderWithTestProviders(
      <TacticalMapViewer file="maps/A9.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/A9.dat');
    });
    loadMap('maps/A9.dat');
    onEditorEvent(soldierPaletteEvent());
    selectMode(getByLabelText, 'Soldiers');
    onEditorEvent(soldierEvent(soldier()));
    expect(await findByText('Normal')).toBeInTheDocument();
    // Normal placements only have the general values
    expect(queryByLabelText('soldier tab')).toBe(null);

    await pickOption('orders', 'Point patrol');
    await waitFor(() => {
      expect(sentSoldiers()).toHaveLength(1);
    });
    expect(sentSoldiers()[0]).toEqual({
      id: 7,
      orders: 'pointPatrol',
      attitude: 'defensive',
      direction: 'northWest',
      relativeAttributeLevel: 2,
      relativeEquipmentLevel: 2,
      soldierClass: 3,
      civilianGroup: 0,
      body: null,
      priorityExistence: false,
      hasKeys: false,
      detailed: null,
    });
    // New soldiers get the orders too
    await waitFor(() => {
      expect(lastMode().defaults).toMatchObject({ orders: 'pointPatrol' });
    });

    fireEvent.click(within(getByLabelText('class')).getByText('Elite'));
    await pickOption('body type', 'Reg Female');
    fireEvent.click(getByText('Has keys'));
    await waitFor(() => {
      expect(sentSoldiers()).toHaveLength(4);
    });
    expect(sentSoldiers().slice(1)).toMatchObject([
      { soldierClass: 2 },
      { body: 3 },
      { hasKeys: true },
    ]);

    fireEvent.click(within(getByLabelText('placement')).getByText('Detailed'));
    await waitFor(() => {
      expect(soldierCommands()).toEqual([
        { kind: 'detailed', detailed: true, profile: null },
      ]);
    });

    // A profile decides the existence and the relative levels, they are hidden then
    onEditorEvent(
      soldierEvent(
        soldier({
          detailed: { ...detailedPlacement(), profile: 57 },
        }),
      ),
    );
    await waitFor(() => {
      expect(queryByLabelText('relative attributes')).toBe(null);
    });
    expect(queryByLabelText('relative equipment')).toBe(null);
    expect(queryByLabelText('class')).toBe(null);
    expect(queryByText('Priority existence')).toBe(null);
    expect(queryByLabelText('soldier tab')).toBe(null);
    expect(getAllByLabelText('orders')).not.toHaveLength(0);

    // Civilians have a group instead of a class
    onEditorEvent(soldierEvent(soldier({ team: 'civilian', id: 8 })));
    await pickOption('civilian group', 'KINGPIN');
    await waitFor(() => {
      expect(sentSoldiers()[sentSoldiers().length - 1]).toMatchObject({
        id: 8,
        civilianGroup: 2,
      });
    });
  });

  it('should place, erase and select items in the items mode', async () => {
    mockItemFiles();
    // The clear buttons of the search boxes break role queries in jsdom, the palette is queried
    // by its labels
    const {
      getByLabelText,
      getAllByLabelText,
      queryByLabelText,
      findByLabelText,
      findByText,
      findByTitle,
      queryByText,
    } = renderWithTestProviders(
      <TacticalMapViewer file="maps/A9.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/A9.dat');
    });
    loadMap('maps/A9.dat');
    selectMode(getByLabelText, 'Items');
    await waitFor(() => {
      expect(lastMode()).toEqual({
        mode: 'items',
        tool: 'select',
        erase: false,
        brush: { kind: 'square', size: 1 },
        choice: null,
        trapLevel: 9,
        action: 12,
      });
    });
    // Selecting has no palette and no erase controls
    expect(queryByLabelText('item category')).toBeNull();
    expect(queryByLabelText('erase')).toBeNull();
    onEditorEvent(itemEvent(null));
    expect(await findByText('No tile selected')).toBeInTheDocument();
    expect(await findByText('2 items')).toBeInTheDocument();

    fireEvent.click(within(getByLabelText('item tool')).getByText('Place'));
    fireEvent.click(await findByLabelText('GLOCK_18'));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        tool: 'place',
        choice: { kind: 'item', item: 2 },
      });
    });
    // The pistols on the map
    expect(await findByTitle('2')).toBeInTheDocument();

    // The triggers and actions, and what new actions do
    await pickOption('item category', 'Triggers');
    fireEvent.click(await findByLabelText('Panic Trigger2'));
    await pickOption('action of new actions', '3x3 Hidden Pit');
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        choice: { kind: 'trigger', slot: 2 },
        action: 9,
      });
    });

    // A key for each lock of the lock table
    await pickOption('item category', 'Keys');
    fireEvent.click(await findByLabelText('Front door'));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        choice: { kind: 'key', lockId: 0, item: 271 },
      });
    });

    // Erasing uses the brush instead of the palette
    fireEvent.click(within(getByLabelText('erase')).getByText('Remove'));
    fireEvent.click(within(getByLabelText('brush')).getByText('Circle'));
    expect(queryByLabelText('item category')).toBeNull();
    await waitFor(() => {
      expect(lastMode()).toMatchObject({
        tool: 'place',
        erase: true,
        brush: { kind: 'circle', size: 1 },
      });
    });

    // The selected tile lists all its items
    fireEvent.click(within(getByLabelText('item tool')).getByText('Select'));
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ tool: 'select' });
    });
    expect(queryByLabelText('brush')).toBeNull();
    onEditorEvent(
      itemEvent([
        { stack: itemStack(2, [80, 0, 10]) },
        { stack: itemStack(2, [60, 0, 10]) },
      ]),
    );
    const selected = within(await findByLabelText('selected tile'));
    expect(selected.getByText('2 items on the tile')).toBeInTheDocument();
    expect(selected.getAllByText('GLOCK_18')).toHaveLength(2);
    // There are no buttons for the keys
    expect(queryByText('Delete')).toBeNull();
    expect(queryByText('Next tile')).toBeNull();

    // Each item can be removed and edited on its own
    const [, removeSecond] = selected.getAllByLabelText('remove GLOCK_18');
    expect(removeSecond).toBeDefined();
    fireEvent.click(removeSecond as HTMLElement);
    fireEvent.click(selected.getByText('Tile 12345'));
    await waitFor(() => {
      expect(itemCommands()).toEqual([
        { kind: 'delete', id: 5 },
        { kind: 'center' },
      ]);
    });
    const values = getAllByLabelText('item values');
    expect(values).toHaveLength(2);
    const existChance = within(values[1] as HTMLElement).getByLabelText(
      'Exist chance',
    );
    fireEvent.change(existChance, { target: { value: '60' } });
    fireEvent.blur(existChance);
    await waitFor(() => {
      expect(sentItems()).toHaveLength(1);
    });
    expect(sentItems()[0]).toMatchObject({ id: 5, nonExistChance: 40 });
    expect(sentItems()[0].stack.data[0]).toBe(60);

    onEditorEvent({
      type: 'notice',
      map: 'maps/A9.dat',
      message: 'The items would sink in the water',
    });
    expect(
      await findByText('The items would sink in the water'),
    ).toBeInTheDocument();
  });

  it('should edit the values of the selected item', async () => {
    mockItemFiles([
      { itemIndex: 50, internalName: 'UNDER_GLAUNCHER', usItemClass: 0x10 },
      { itemIndex: 240, internalName: 'OWNERSHIP', usItemClass: 0x10000000 },
    ]);
    const { getByLabelText, findByLabelText, findByText, queryByText } =
      renderWithTestProviders(
        <TacticalMapViewer file="maps/A9.dat" onBack={() => {}} />,
      );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/A9.dat');
    });
    loadMap('maps/A9.dat');
    onEditorEvent(soldierPaletteEvent());
    selectMode(getByLabelText, 'Items');
    onEditorEvent(itemEvent([{ stack: itemStack(2, [80, 0, 10]) }]));
    const values = () => within(getByLabelText('item values'));
    await findByLabelText('item values');

    // Values are sent when they are entered
    const status = values().getByLabelText('Status');
    fireEvent.change(status, { target: { value: '95' } });
    expect(viewer.set_item).not.toHaveBeenCalled();
    fireEvent.blur(status);
    await waitFor(() => {
      expect(sentItems()).toHaveLength(1);
    });
    expect(sentItems()[0]).toMatchObject({ id: 4, visibility: 'invisible' });
    expect(sentItems()[0].stack.data.slice(0, 3)).toEqual([95, 0, 10]);

    // The attachments the gun takes
    expect(values().queryByText('Bipod')).not.toBeInTheDocument();
    fireEvent.click(values().getByText('Silencer'));
    await waitFor(() => {
      expect(sentItems()).toHaveLength(2);
    });
    expect(sentItems()[1].stack.attachments).toEqual([
      { item: 207, status: 80 },
    ]);

    // The exist chance only changes the item, new items always exist
    const existChance = values().getByLabelText('Exist chance');
    fireEvent.change(existChance, { target: { value: '60' } });
    fireEvent.blur(existChance);
    await waitFor(() => {
      expect(sentItems()).toHaveLength(3);
    });
    expect(sentItems()[2].nonExistChance).toBe(40);
    expect(lastMode()).not.toHaveProperty('existChance');

    fireEvent.click(values().getByText('Hidden'));
    fireEvent.click(values().getByText('Realistic'));
    await waitFor(() => {
      expect(sentItems()).toHaveLength(5);
    });
    expect(sentItems()[3].visibility).toBe('hiddenItem');
    expect(sentItems()[4].flags).toBe(0x100);

    // Actions can't be hidden, their action becomes the one of new actions
    onEditorEvent(
      itemEvent([
        {
          id: 5,
          visibility: 'buried',
          stack: { ...itemStack(234, [100, 2, 137, 0, 52, 1, 3]), trap: 9 },
        },
      ]),
    );
    await waitFor(() => {
      expect(queryByText('Hidden')).not.toBeInTheDocument();
    });
    await pickOption('action', '3x3 Hidden Pit');
    await waitFor(() => {
      expect(sentItems()).toHaveLength(6);
    });
    expect(sentItems()[5]).toMatchObject({ id: 5, visibility: 'buried' });
    expect(sentItems()[5].stack.data.slice(0, 8)).toEqual([
      100, 2, 0, 0, 52, 1, 4, 0,
    ]);
    await waitFor(() => {
      expect(lastMode()).toMatchObject({ action: 9 });
    });

    // Panic triggers can be alarm triggers
    onEditorEvent(
      itemEvent([{ id: 6, stack: itemStack(233, [100, 0, 0, 0, 127, 1]) }]),
    );
    fireEvent.click(await findByText('Alarm trigger'));
    await waitFor(() => {
      expect(sentItems()).toHaveLength(7);
    });
    expect(sentItems()[6].stack.flags).toBe(0x40);

    // Ownership items name a civilian group
    onEditorEvent(itemEvent([{ id: 7, stack: itemStack(240, [200, 0]) }]));
    await waitFor(() => {
      expect(queryByText('Alarm trigger')).not.toBeInTheDocument();
    });
    await pickOption('owner civilian group', 'REBEL');
    await waitFor(() => {
      expect(sentItems()).toHaveLength(8);
    });
    expect(sentItems()[7].stack.data.slice(0, 2)).toEqual([200, 1]);
  });

  it('should edit the attributes, items, colours and profile of detailed soldiers', async () => {
    const invokeMock = getInvokeMock();
    const items: Record<string, unknown[]> = {
      'weapons.json': [
        { itemIndex: 2, internalName: 'GLOCK_18', ubMagSize: 15 },
      ],
      'magazines.json': [],
      'armours.json': [],
      'explosives.json': [],
      'items.json': [{ itemIndex: 202, internalName: 'MEDICKIT' }],
      'mercs-profile-info.json': [{ profileID: 57, internalName: 'KINGPIN' }],
    };
    for (const [file, vanilla] of Object.entries(items)) {
      invokeMock.resolve(
        'json/read',
        { file },
        { schema: {}, vanilla: vanilla as never, value: null, patch: null },
      );
    }
    const { getByLabelText, findByText, getByText, queryByLabelText } =
      renderWithTestProviders(
        <TacticalMapViewer file="maps/A9.dat" onBack={() => {}} />,
      );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/A9.dat');
    });
    loadMap('maps/A9.dat');
    onEditorEvent(soldierPaletteEvent());
    selectMode(getByLabelText, 'Soldiers');
    const detailed = detailedPlacement();
    onEditorEvent(soldierEvent(soldier({ detailed })));
    const tabs = () => within(getByLabelText('soldier tab'));
    await findByText('General');

    // Attributes are sent when they are entered
    fireEvent.click(tabs().getByText('Attributes'));
    const agility = getByLabelText('Agility');
    fireEvent.change(agility, { target: { value: '80' } });
    expect(viewer.set_soldier).not.toHaveBeenCalled();
    fireEvent.blur(agility);
    await waitFor(() => {
      expect(sentSoldiers()).toHaveLength(1);
    });
    expect(sentSoldiers()[0].detailed.stats).toMatchObject({
      agility: 80,
      life: null,
    });

    // A new item in the hand, loaded and kept by the game
    fireEvent.click(tabs().getByText('Inventory'));
    await pickOption('Hand', 'GLOCK_18');
    await waitFor(() => {
      expect(sentSoldiers()).toHaveLength(2);
    });
    const hand = sentSoldiers()[1].detailed.inventory[5];
    expect(hand).toMatchObject({ item: 2, number: 1, flags: 0x81 });
    expect(hand.data[2]).toBe(15);
    expect(sentSoldiers()[1].detailed.inventory[0]).toBe(null);

    // Fixing the colours starts with the ones the soldier has
    fireEvent.click(tabs().getByText('Appearance'));
    fireEvent.click(getByText('Fixed colours'));
    await waitFor(() => {
      expect(sentSoldiers()).toHaveLength(3);
    });
    expect(sentSoldiers()[2].detailed).toMatchObject({
      visible: true,
      headPalette: 'BROWNHEAD',
      vestPalette: 'REDVEST',
      pantsPalette: 'GREENPANTS',
      skinPalette: 'PINKSKIN',
    });

    // A profile placement starts with the first profile
    fireEvent.click(within(getByLabelText('placement')).getByText('Profile'));
    await waitFor(() => {
      expect(soldierCommands()).toEqual([
        { kind: 'detailed', detailed: true, profile: 57 },
      ]);
    });
    onEditorEvent(
      soldierEvent(soldier({ detailed: { ...detailed, profile: 57 } })),
    );
    await waitFor(() => {
      expect(queryByLabelText('soldier tab')).toBe(null);
    });
    expect(getByText('57 KINGPIN')).toBeInTheDocument();
    expect(queryByLabelText('relative attributes')).toBe(null);

    // Leaving the profile keeps the detailed placement
    fireEvent.click(within(getByLabelText('placement')).getByText('Detailed'));
    await waitFor(() => {
      expect(soldierCommands()[1]).toEqual({
        kind: 'detailed',
        detailed: true,
        profile: null,
      });
    });

    // Creatures have no colours, basic placements only the general tab
    onEditorEvent(soldierEvent(soldier({ team: 'creature', detailed })));
    await waitFor(() => {
      expect(tabs().queryByText('Appearance')).toBe(null);
    });
    onEditorEvent(soldierEvent(soldier()));
    await waitFor(() => {
      expect(queryByLabelText('soldier tab')).toBe(null);
    });
  });

  it('should edit the schedule of civilians and pick its tiles', async () => {
    const {
      getByLabelText,
      findByText,
      getByText,
      getByRole,
      queryByLabelText,
    } = renderWithTestProviders(
      <TacticalMapViewer file="maps/A9.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/A9.dat');
    });
    loadMap('maps/A9.dat');
    onEditorEvent(soldierPaletteEvent());
    selectMode(getByLabelText, 'Soldiers');
    const civilian = soldier({
      team: 'civilian',
      detailed: detailedPlacement(),
    });
    onEditorEvent(soldierEvent(civilian));
    await findByText('General');
    fireEvent.click(
      within(getByLabelText('soldier tab')).getByText('Schedule'),
    );

    // A door action picks the tile next to the door right away
    await pickOption('action 1', 'Lock door');
    await waitFor(() => {
      expect(soldierCommands()).toHaveLength(2);
    });
    const [schedule, pick] = soldierCommands() as any[];
    expect(schedule.kind).toBe('schedule');
    expect(schedule.rows[0]).toEqual({
      action: 'lockDoor',
      time: null,
      variance: false,
      data1: null,
      data2: null,
    });
    expect(pick).toEqual({ kind: 'pick', slot: 0, point: 'a', single: false });

    const rows = emptySchedule();
    rows[0] = { ...rows[0]!, action: 'lockDoor', data1: 1000 };
    onEditorEvent(
      soldierEvent(civilian, {
        schedule: rows,
        pick: { slot: 0, point: 'b', single: false },
      }),
    );
    expect(
      await findByText(
        /Click on the tile where you wish to move after you lock/,
      ),
    ).toBeInTheDocument();
    fireEvent.click(getByRole('button', { name: 'Cancel' }));
    await waitFor(() => {
      expect(soldierCommands()[2]).toEqual({ kind: 'cancelPick' });
    });

    // Times are sent once they are entered
    onEditorEvent(soldierEvent(civilian, { schedule: rows, pick: null }));
    const time = getByLabelText('time 1');
    fireEvent.change(time, { target: { value: '8:30' } });
    fireEvent.blur(time);
    await waitFor(() => {
      expect(soldierCommands()).toHaveLength(4);
    });
    expect((soldierCommands()[3] as any).rows[0]).toMatchObject({
      action: 'lockDoor',
      time: 510,
      data1: 1000,
    });

    // The tiles are picked again with a click and shown with Ctrl+click
    fireEvent.click(getByLabelText('tile 1A'), { ctrlKey: true });
    fireEvent.click(getByLabelText('tile 1B'));
    fireEvent.click(getByText('Clear schedule'));
    await waitFor(() => {
      expect(soldierCommands().slice(4)).toEqual([
        { kind: 'center', gridNo: 1000 },
        { kind: 'pick', slot: 0, point: 'b', single: true },
        { kind: 'clearSchedule' },
      ]);
    });

    // Civilians with a profile keep their schedule, without the tabs
    onEditorEvent(
      soldierEvent(
        { ...civilian, detailed: { ...civilian.detailed!, profile: 57 } },
        { schedule: rows, pick: null },
      ),
    );
    await waitFor(() => {
      expect(queryByLabelText('soldier tab')).toBe(null);
    });
    expect(getByLabelText('time 1')).toBeInTheDocument();

    // Only civilians have schedules
    onEditorEvent(soldierEvent({ ...civilian, team: 'enemy', id: 9 }));
    await waitFor(() => {
      expect(
        within(getByLabelText('soldier tab')).queryByText('Schedule'),
      ).toBe(null);
    });
  });

  it('should select palette tiles and count shift-clicks', async () => {
    const { getByRole, getByLabelText, queryByRole } = renderWithTestProviders(
      <TacticalMapViewer file="maps/D2.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/D2.dat');
    });
    loadMap('maps/D2.dat');
    onEditorEvent({
      type: 'palette',
      map: 'maps/D2.dat',
      tileset: 0,
      ground: [],
      roads: [],
      cliffs: [],
      debris: [],
      trees: [paletteEntry(100, 1), paletteEntry(103, 4)],
      rocks: [],
      misc: [],
      furniture: [],
      genericFurniture: [],
      buildings: emptyBuildingPalettes(),
    });
    selectMode(getByLabelText, 'Objects');

    const trees = getByRole('group', { name: 'Trees & bushes' });
    // Groups without tiles are hidden
    expect(within(getByLabelText('map editor')).queryByText('Rocks')).toBe(
      null,
    );
    // The first tile is selected once the palette arrives
    await waitFor(() => {
      expect(lastMode()?.objects).toEqual([{ tileId: 100, count: 1 }]);
    });
    const second = within(trees).getByRole('button', {
      name: 'trees 4 (OFSTRUCT1)',
    });
    fireEvent.click(second, { shiftKey: true });
    fireEvent.click(second, { shiftKey: true });
    await waitFor(() => {
      expect(lastMode().objects).toEqual([
        { tileId: 100, count: 1 },
        { tileId: 103, count: 2 },
      ]);
    });
    // A click without shift selects only that tile
    fireEvent.click(
      within(trees).getByRole('button', { name: 'trees 1 (OFSTRUCT1)' }),
    );
    await waitFor(() => {
      expect(lastMode().objects).toEqual([{ tileId: 100, count: 1 }]);
    });
    fireEvent.click(second, { shiftKey: true });
    fireEvent.click(second, { shiftKey: true });
    await waitFor(() => {
      expect(lastMode().objects).toEqual([
        { tileId: 100, count: 1 },
        { tileId: 103, count: 2 },
      ]);
    });
    fireEvent.contextMenu(second);
    fireEvent.contextMenu(
      within(trees).getByRole('button', { name: 'trees 1 (OFSTRUCT1)' }),
    );
    await waitFor(() => {
      expect(lastMode().objects).toEqual([{ tileId: 103, count: 1 }]);
    });

    fireEvent.click(getByRole('button', { name: 'Clear' }));
    await waitFor(() => {
      expect(lastMode().objects).toEqual([]);
    });

    // The tiles can be searched by their file and type names
    fireEvent.change(getByLabelText('search objects'), {
      target: { value: 'ofstruct1' },
    });
    expect(within(trees).getAllByRole('button')).toHaveLength(2);
    fireEvent.change(getByLabelText('search objects'), {
      target: { value: 'trees 4' },
    });
    expect(within(trees).getAllByRole('button')).toHaveLength(1);
    expect(
      within(trees).getByRole('button', { name: 'trees 4 (OFSTRUCT1)' }),
    ).toBeInTheDocument();
    fireEvent.change(getByLabelText('search objects'), {
      target: { value: 'nothing' },
    });
    expect(
      within(getByLabelText('map editor')).getByText('No matching tiles'),
    ).toBeInTheDocument();
    expect(queryByRole('group', { name: 'Trees & bushes' })).toBe(null);
  });

  it('should forward keys pressed outside of the canvas, except while typing', async () => {
    const { unmount } = renderWithTestProviders(
      <TacticalMapViewer file="maps/E4.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/E4.dat');
    });

    fireEvent.keyDown(document.body, { code: 'KeyW', key: 'w' });
    fireEvent.keyUp(document.body, { code: 'KeyW', key: 'w' });
    expect(viewer.key_event).toHaveBeenNthCalledWith(
      1,
      'KeyW',
      'w',
      true,
      false,
    );
    expect(viewer.key_event).toHaveBeenNthCalledWith(
      2,
      'KeyW',
      'w',
      false,
      false,
    );

    // The canvas receives its keys itself
    const canvas = document.getElementById('ja2-map-viewer');
    expect(canvas).not.toBe(null);
    fireEvent.keyDown(canvas!, { code: 'KeyA', key: 'a' });
    // Shortcuts of the app and typing don't move the map, but releases are forwarded
    fireEvent.keyDown(document.body, { code: 'KeyS', key: 's', ctrlKey: true });
    const input = document.body.appendChild(document.createElement('input'));
    fireEvent.keyDown(input, { code: 'KeyD', key: 'd' });
    expect(viewer.key_event).toHaveBeenCalledTimes(2);
    fireEvent.keyUp(input, { code: 'KeyD', key: 'd' });
    expect(viewer.key_event).toHaveBeenLastCalledWith(
      'KeyD',
      'd',
      false,
      false,
    );

    input.remove();

    // Inputs that don't take typed text don't block keys
    for (const setup of [
      (el: HTMLInputElement) => (el.type = 'checkbox'),
      (el: HTMLInputElement) => (el.type = 'radio'),
      (el: HTMLInputElement) => (el.readOnly = true),
    ]) {
      const other = document.body.appendChild(document.createElement('input'));
      setup(other);
      fireEvent.keyDown(other, { code: 'KeyE', key: 'e' });
      expect(viewer.key_event).toHaveBeenLastCalledWith(
        'KeyE',
        'e',
        true,
        false,
      );
      other.remove();
    }
    expect(viewer.key_event).toHaveBeenCalledTimes(6);

    fireEvent.blur(window);
    expect(viewer.release_keys).toHaveBeenCalledTimes(1);
    unmount();
    expect(viewer.release_keys).toHaveBeenCalledTimes(2);
    fireEvent.keyDown(document.body, { code: 'KeyW', key: 'w' });
    expect(viewer.key_event).toHaveBeenCalledTimes(6);
  });

  it('should handle keys pressed on a closed select as if nothing was focused', async () => {
    const { getByRole, queryByRole } = renderWithTestProviders(
      <>
        <TacticalMapViewer file="maps/E5.dat" onBack={() => {}} />
        <Select
          aria-label="test select"
          options={[{ value: 'a', label: 'A' }]}
        />
      </>,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/E5.dat');
    });

    const select = getByRole('combobox', { name: 'test select' });
    select.focus();
    fireEvent.keyDown(select, { code: 'ArrowDown', key: 'ArrowDown' });
    expect(viewer.key_event).toHaveBeenLastCalledWith(
      'ArrowDown',
      'ArrowDown',
      true,
      false,
    );
    expect(document.activeElement).not.toBe(select);
    expect(select).toHaveAttribute('aria-expanded', 'false');
    expect(queryByRole('listbox')).toBeNull();

    select.focus();
    fireEvent.keyDown(select, { code: 'KeyZ', key: 'z', ctrlKey: true });
    await waitFor(() => {
      expect(viewer.undo).toHaveBeenCalledTimes(1);
    });
    expect(select).toHaveAttribute('aria-expanded', 'false');
  });

  it('should undo with Ctrl+Z and redo with Ctrl+Y or Ctrl+Shift+Z, except while typing', async () => {
    renderWithTestProviders(
      <TacticalMapViewer file="maps/F8.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/F8.dat');
    });

    fireEvent.keyDown(document.body, { code: 'KeyZ', key: 'z', ctrlKey: true });
    await waitFor(() => {
      expect(viewer.undo).toHaveBeenCalledTimes(1);
    });

    const input = document.body.appendChild(document.createElement('input'));
    fireEvent.keyDown(input, {
      code: 'KeyZ',
      key: 'z',
      ctrlKey: true,
    });
    fireEvent.keyDown(input, { code: 'KeyY', key: 'y', ctrlKey: true });
    fireEvent.keyDown(document.body, {
      code: 'KeyZ',
      key: 'Z',
      ctrlKey: true,
      shiftKey: true,
    });
    await waitFor(() => {
      expect(viewer.redo).toHaveBeenCalledTimes(1);
    });
    fireEvent.keyDown(document.body, { code: 'KeyY', key: 'y', ctrlKey: true });
    await waitFor(() => {
      expect(viewer.redo).toHaveBeenCalledTimes(2);
    });
    // Give a wrongly sent undo or redo the time to arrive
    await act(() => Promise.resolve());
    expect(viewer.undo).toHaveBeenCalledTimes(1);
    expect(viewer.redo).toHaveBeenCalledTimes(2);
    input.remove();

    // Checkboxes keep the focus after clicking them, but don't block undo
    const checkbox = document.body.appendChild(document.createElement('input'));
    checkbox.type = 'checkbox';
    fireEvent.keyDown(checkbox, { code: 'KeyZ', key: 'z', ctrlKey: true });
    await waitFor(() => {
      expect(viewer.undo).toHaveBeenCalledTimes(2);
    });
    checkbox.remove();
  });

  it('should save the map once it was changed', async () => {
    const invokeMock = getInvokeMock();
    const { getByRole } = renderWithTestProviders(
      <TacticalMapViewer file="maps/G1.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/G1.dat');
    });
    const saveButton = getByRole('button', { name: 'Save' });
    expect(saveButton).toBeDisabled();
    onStatus({ state: 'loaded', map: 'maps/G1.dat' });
    onEditorEvent(stateEvent('maps/G1.dat', 0));
    expect(saveButton).toBeDisabled();
    // Changes of other maps are ignored
    onEditorEvent(stateEvent('maps/A9.dat', 2));
    expect(saveButton).toBeDisabled();

    onEditorEvent(stateEvent('maps/G1.dat', 2));
    expect(saveButton).toBeEnabled();
    vi.mocked(viewer.save_map).mockResolvedValueOnce({
      map: 'maps/G1.dat',
      revision: 2,
      data: new Uint8Array([1, 2, 3]),
    });
    invokeMock.resolve(
      'resources/persist',
      { file: 'maps/G1.dat', data: 'AQID' },
      null,
    );
    fireEvent.click(saveButton);
    await waitFor(() => {
      expect(window.electronAPI.invoke).toHaveBeenCalledWith({
        name: 'resources/persist',
        input: { file: 'maps/G1.dat', data: 'AQID' },
      });
    });
    await waitFor(() => {
      expect(saveButton).toBeDisabled();
    });
    expect(viewer.save_map).toHaveBeenCalledTimes(1);

    // Ctrl+S saves too
    onEditorEvent(stateEvent('maps/G1.dat', 3));
    expect(saveButton).toBeEnabled();
    vi.mocked(viewer.save_map).mockResolvedValueOnce({
      map: 'maps/G1.dat',
      revision: 3,
      data: new Uint8Array([1, 2, 3]),
    });
    fireEvent.keyDown(document.body, { code: 'KeyS', key: 's', ctrlKey: true });
    await waitFor(() => {
      expect(viewer.save_map).toHaveBeenCalledTimes(2);
    });
    await waitFor(() => {
      expect(saveButton).toBeDisabled();
    });
  });

  it('should create a new map that can be saved right away', async () => {
    const invokeMock = getInvokeMock();
    const { getByRole, findByText } = renderWithTestProviders(
      <TacticalMapViewer
        file="maps/G3.dat"
        newMap={{ tileset: 1, smoothing: 'caves' }}
        onBack={() => {}}
      />,
    );
    await waitFor(() => {
      expect(viewer.new_map).toHaveBeenCalledWith(
        'maps/G3.dat',
        JSON.stringify({ tileset: 1, smoothing: 'caves' }),
      );
    });
    expect(viewer.load_map).not.toHaveBeenCalled();
    expect(await findByText('maps/G3.dat (new)')).toBeInTheDocument();
    const saveButton = getByRole('button', { name: 'Save' });
    expect(saveButton).toBeDisabled();

    // The map was never saved
    loadMap('maps/G3.dat');
    expect(saveButton).toBeEnabled();
    vi.mocked(viewer.save_map).mockResolvedValueOnce({
      map: 'maps/G3.dat',
      revision: 0,
      data: new Uint8Array([1, 2, 3]),
    });
    invokeMock.resolve(
      'resources/persist',
      { file: 'maps/G3.dat', data: 'AQID' },
      null,
    );
    fireEvent.click(saveButton);
    await waitFor(() => {
      expect(saveButton).toBeDisabled();
    });
    expect(window.electronAPI.invoke).toHaveBeenCalledWith({
      name: 'resources/persist',
      input: { file: 'maps/G3.dat', data: 'AQID' },
    });
    expect(await findByText('maps/G3.dat')).toBeInTheDocument();
  });

  it('should show why the map could not be saved', async () => {
    const { findByTitle, getByRole } = renderWithTestProviders(
      <TacticalMapViewer file="maps/G2.dat" onBack={() => {}} />,
    );
    await waitFor(() => {
      expect(viewer.load_map).toHaveBeenCalledWith('maps/G2.dat');
    });
    onStatus({ state: 'loaded', map: 'maps/G2.dat' });
    onEditorEvent(stateEvent('maps/G2.dat', 1));
    vi.mocked(viewer.save_map).mockRejectedValueOnce(
      new Error('too many doors'),
    );
    const saveButton = getByRole('button', { name: 'Save' });
    fireEvent.click(saveButton);
    expect(await findByTitle('too many doors')).toBeInTheDocument();
    // It can be saved again
    expect(saveButton).toBeEnabled();
  });
});
