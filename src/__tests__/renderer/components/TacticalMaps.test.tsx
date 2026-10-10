import { fireEvent, waitFor, within } from '@testing-library/react';
import { isDeepEqual } from 'remeda';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TacticalMaps } from '../../../renderer/components/content/TacticalMaps';
import {
  attach,
  createMap,
  detach,
  listTilesets,
  loadMap,
  regenerateRadarMap,
  setRenderOptions,
  subscribeStatus,
} from '../../../renderer/lib/mapViewer';
import { getInvokeMock } from '../test-utils/invoke';
import { renderWithTestProviders } from '../test-utils/render';

// The real module needs the built wasm viewer
vi.mock('../../../renderer/lib/mapViewer', async (importOriginal) => {
  const original =
    await importOriginal<typeof import('../../../renderer/lib/mapViewer')>();
  return {
    DEFAULT_RENDER_OPTIONS: original.DEFAULT_RENDER_OPTIONS,
    DOOR_TRAP_KINDS: original.DOOR_TRAP_KINDS,
    SOLDIER_TEAMS: original.SOLDIER_TEAMS,
    SOLDIER_ORDERS: original.SOLDIER_ORDERS,
    SOLDIER_ATTITUDES: original.SOLDIER_ATTITUDES,
    DIRECTIONS: original.DIRECTIONS,
    LIGHT_RADII: original.LIGHT_RADII,
    ENTRY_POINTS: original.ENTRY_POINTS,
    EDGE_SIDES: original.EDGE_SIDES,
    radarMapFile: original.radarMapFile,
    regenerateRadarMap: vi.fn(),
    attach: vi.fn(),
    detach: vi.fn(),
    loadMap: vi.fn(),
    createMap: vi.fn(),
    listTilesets: vi.fn(),
    setRenderOptions: vi.fn(),
    subscribeStatus: vi.fn(),
    setEditorMode: vi.fn(() => Promise.resolve()),
    undo: vi.fn(() => Promise.resolve()),
    redo: vi.fn(() => Promise.resolve()),
    setMapBackground: vi.fn(() => Promise.resolve()),
    setMapSmoothing: vi.fn(() => Promise.resolve()),
    roofAllRooms: vi.fn(() => Promise.resolve()),
    setDoor: vi.fn(() => Promise.resolve()),
    setSoldier: vi.fn(() => Promise.resolve()),
    soldierCommand: vi.fn(() => Promise.resolve()),
    setLight: vi.fn(() => Promise.resolve()),
    lightCommand: vi.fn(() => Promise.resolve()),
    setTraversal: vi.fn(() => Promise.resolve()),
    traversalCommand: vi.fn(() => Promise.resolve()),
    subscribeEditorEvents: vi.fn(() => () => {}),
  };
});

function setupInvokes() {
  const invokeMock = getInvokeMock();
  invokeMock.resolve('resources/list', { path: 'maps', modOnly: false }, [
    { type: 'File', path: 'a9.dat' },
    { type: 'File', path: 'a9_b1.dat' },
    { type: 'File', path: 'b13.dat' },
    { type: 'File', path: 'readme.txt' },
  ]);
  invokeMock.resolve('resources/list', { path: 'maps', modOnly: true }, [
    { type: 'File', path: 'b13.dat' },
  ]);
  invokeMock.resolve(
    'image/render',
    { file: 'interface/map_1.sti', subimage: 0 },
    'data:image/png;base64,',
  );
  invokeMock.resolve(
    'image/render',
    { file: 'interface/mine_1.sti', subimage: 0 },
    'data:image/png;base64,',
  );
  // The underground map of A9 has no radar map
  invokeMock.resolve('resources/exists', { files: ['radarmaps/a9.sti'] }, [
    true,
  ]);
  invokeMock.resolve('resources/exists', { files: ['radarmaps/a9_b1.sti'] }, [
    false,
  ]);
  invokeMock.resolve('resources/exists', { files: ['radarmaps/b13.sti'] }, [
    true,
  ]);
  invokeMock.resolve(
    'image/render',
    { file: 'radarmaps/a9.sti', subimage: 0 },
    'data:image/png;base64,YTk=',
  );
  invokeMock.resolve(
    'image/render',
    { file: 'radarmaps/b13.sti', subimage: 0 },
    'data:image/png;base64,YjEz',
  );
  return invokeMock;
}

/** The inputs of the invoke calls of `name` so far */
function invokeInputs(name: string) {
  return vi
    .mocked(window.electronAPI.invoke)
    .mock.calls.map(([payload]) => payload)
    .filter((payload) => payload.name === name)
    .map((payload) => payload.input);
}

function sectorTile(container: HTMLElement, sectorId: string) {
  const tile = container.querySelector(`.strategic-map-tile.${sectorId}`);
  if (!tile) {
    throw new Error(`sector tile ${sectorId} not found`);
  }
  return tile;
}

describe('TacticalMaps', () => {
  beforeEach(() => {
    vi.mocked(attach).mockReset().mockResolvedValue();
    vi.mocked(detach).mockReset();
    vi.mocked(loadMap).mockReset().mockResolvedValue();
    vi.mocked(createMap).mockReset().mockResolvedValue();
    vi.mocked(listTilesets)
      .mockReset()
      .mockResolvedValue({
        default: 0,
        tilesets: [
          { id: 0, name: 'GENERIC 1' },
          { id: 1, name: 'CAVES 1' },
          { id: 2, name: 'FARM 1' },
        ],
      });
    vi.mocked(setRenderOptions).mockReset().mockResolvedValue();
    vi.mocked(regenerateRadarMap).mockReset();
    vi.mocked(subscribeStatus)
      .mockReset()
      .mockReturnValue(() => {});
  });

  it('should highlight sectors with maps', async () => {
    setupInvokes();
    const { container } = renderWithTestProviders(<TacticalMaps />);

    await waitFor(() => {
      expect(
        sectorTile(container, 'A9').classList.contains('highlighted'),
      ).toBe(true);
    });
    expect(sectorTile(container, 'B13').classList).toContain('highlighted');
    expect(sectorTile(container, 'C5').classList).not.toContain('highlighted');
  });

  it('should show the selected file and where it comes from', async () => {
    setupInvokes();
    const { container, findByText, getByText, getByRole } =
      renderWithTestProviders(<TacticalMaps />);
    await waitFor(() => {
      expect(sectorTile(container, 'B13').classList).toContain('highlighted');
    });

    fireEvent.click(sectorTile(container, 'A9'));
    expect(await findByText('maps/A9.dat')).toBeInTheDocument();
    expect(getByText('Vanilla')).toBeInTheDocument();

    fireEvent.click(sectorTile(container, 'B13'));
    expect(await findByText('maps/B13.dat')).toBeInTheDocument();
    expect(getByText('Mod')).toBeInTheDocument();

    fireEvent.click(sectorTile(container, 'C5'));
    expect(await findByText('No map')).toBeInTheDocument();
    expect(getByRole('button', { name: 'Open map' })).toBeDisabled();
  });

  it('should use the underground map file on other levels', async () => {
    setupInvokes();
    const { container, findByText, getByRole } = renderWithTestProviders(
      <TacticalMaps />,
    );
    await waitFor(() => {
      expect(sectorTile(container, 'A9').classList).toContain('highlighted');
    });

    fireEvent.click(getByRole('button', { name: 'minus' }));
    fireEvent.click(sectorTile(container, 'A9'));

    expect(await findByText('maps/A9_b1.dat')).toBeInTheDocument();
  });

  it('should open the selected map in the viewer', async () => {
    setupInvokes();
    const { container, findByRole, getByRole } = renderWithTestProviders(
      <TacticalMaps />,
    );
    await waitFor(() => {
      expect(sectorTile(container, 'A9').classList).toContain('highlighted');
    });

    fireEvent.click(sectorTile(container, 'A9'));
    fireEvent.click(getByRole('button', { name: 'Open map' }));

    await waitFor(() => {
      expect(loadMap).toHaveBeenCalledWith('maps/A9.dat');
    });
    expect(attach).toHaveBeenCalledWith(expect.any(HTMLDivElement));

    fireEvent.click(await findByRole('button', { name: /Back to overview/ }));

    expect(detach).toHaveBeenCalled();
    expect(getByRole('button', { name: 'Open map' })).toBeInTheDocument();
    // Wait for the overview to finish loading again
    await waitFor(() => {
      expect(sectorTile(container, 'A9').classList).toContain('highlighted');
      expect(
        container.querySelector<HTMLElement>('.strategic-map')?.style
          .backgroundImage,
      ).toContain('data:image/png');
    });
  });
  it('should show the mini map of the selected map', async () => {
    setupInvokes();
    const { container, findByRole, queryByRole } = renderWithTestProviders(
      <TacticalMaps />,
    );
    await waitFor(() => {
      expect(sectorTile(container, 'A9').classList).toContain('highlighted');
    });

    fireEvent.click(sectorTile(container, 'A9'));

    expect(await findByRole('img', { name: 'Mini map' })).toHaveAttribute(
      'src',
      'data:image/png;base64,YTk=',
    );
    // Vanilla maps keep their mini maps
    expect(
      queryByRole('button', { name: 'Regenerate mini map' }),
    ).not.toBeInTheDocument();
  });

  it('should tell when a map has no mini map', async () => {
    setupInvokes();
    const { container, findByText, getByRole } = renderWithTestProviders(
      <TacticalMaps />,
    );
    await waitFor(() => {
      expect(sectorTile(container, 'A9').classList).toContain('highlighted');
    });

    fireEvent.click(getByRole('button', { name: 'minus' }));
    fireEvent.click(sectorTile(container, 'A9'));

    expect(await findByText('No mini map')).toBeInTheDocument();
    expect(invokeInputs('image/render')).not.toContainEqual({
      file: 'radarmaps/a9_b1.sti',
      subimage: 0,
    });
  });

  it('should regenerate the mini map of maps of the mod', async () => {
    setupInvokes();
    vi.mocked(regenerateRadarMap).mockResolvedValue('radarmaps/b13.sti');
    const { container, findByRole } = renderWithTestProviders(<TacticalMaps />);
    await waitFor(() => {
      expect(sectorTile(container, 'B13').classList).toContain('highlighted');
    });

    fireEvent.click(sectorTile(container, 'B13'));
    await findByRole('img', { name: 'Mini map' });
    const renders = () =>
      invokeInputs('image/render').filter((input) =>
        isDeepEqual(input, { file: 'radarmaps/b13.sti', subimage: 0 }),
      ).length;
    expect(renders()).toBe(1);

    fireEvent.click(
      await findByRole('button', { name: 'Regenerate mini map' }),
    );

    expect(regenerateRadarMap).toHaveBeenCalledWith('maps/B13.dat');
    // The new mini map is loaded
    await waitFor(() => {
      expect(renders()).toBe(2);
    });
    expect(await findByRole('img', { name: 'Mini map' })).toBeInTheDocument();
  });

  it('should show why the mini map could not be regenerated', async () => {
    setupInvokes();
    vi.mocked(regenerateRadarMap).mockRejectedValue(
      new Error('The map could not be loaded'),
    );
    const { container, findByRole, findByText } = renderWithTestProviders(
      <TacticalMaps />,
    );
    await waitFor(() => {
      expect(sectorTile(container, 'B13').classList).toContain('highlighted');
    });

    fireEvent.click(sectorTile(container, 'B13'));
    fireEvent.click(
      await findByRole('button', { name: 'Regenerate mini map' }),
    );

    expect(
      await findByText('Could not regenerate the mini map'),
    ).toBeInTheDocument();
    expect(await findByText('The map could not be loaded')).toBeInTheDocument();
  });

  it('should only allow a new map for a selected sector', async () => {
    setupInvokes();
    const { container, getByRole } = renderWithTestProviders(<TacticalMaps />);
    await waitFor(() => {
      expect(sectorTile(container, 'A9').classList).toContain('highlighted');
    });
    expect(getByRole('button', { name: 'New map' })).toBeDisabled();

    fireEvent.click(sectorTile(container, 'C5'));
    expect(getByRole('button', { name: 'New map' })).toBeEnabled();
  });

  it('should choose the type and the tileset of new maps', async () => {
    setupInvokes();
    const { container, findByRole, getByRole } = renderWithTestProviders(
      <TacticalMaps />,
    );
    await waitFor(() => {
      expect(sectorTile(container, 'A9').classList).toContain('highlighted');
    });

    // Sectors without a map have nothing to replace
    fireEvent.click(sectorTile(container, 'C5'));
    fireEvent.click(getByRole('button', { name: 'New map' }));
    let dialog = await findByRole('dialog');
    const tileset = await within(dialog).findByRole('combobox', {
      name: 'tileset',
    });
    expect(within(dialog).getByTitle('0 – GENERIC 1')).toBeInTheDocument();
    expect(
      within(dialog).getByRole('radio', { name: 'Outdoor' }),
    ).toBeChecked();
    expect(
      within(dialog).queryByText(
        'Saving replaces the existing map of this sector',
      ),
    ).not.toBeInTheDocument();

    // Caves use the caves tileset
    fireEvent.click(within(dialog).getByText('Caves'));
    expect(within(dialog).getByTitle('1 – CAVES 1')).toBeInTheDocument();
    // Unless another one was chosen
    fireEvent.mouseDown(tileset);
    fireEvent.click(await within(document.body).findByTitle('2 – FARM 1'));
    fireEvent.click(within(dialog).getByText('Outdoor'));
    fireEvent.click(within(dialog).getByText('Caves'));
    expect(within(dialog).getByTitle('2 – FARM 1')).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    await waitFor(() => {
      expect(container.ownerDocument.querySelector('.ant-modal-wrap')).toBe(
        null,
      );
    });

    // Underground maps start as basements, and saving replaces their map
    fireEvent.click(getByRole('button', { name: 'minus' }));
    fireEvent.click(sectorTile(container, 'A9'));
    fireEvent.click(getByRole('button', { name: 'New map' }));
    dialog = await findByRole('dialog');
    await within(dialog).findByRole('combobox', { name: 'tileset' });
    expect(
      within(dialog).getByRole('radio', { name: 'Basement' }),
    ).toBeChecked();
    expect(
      within(dialog).getByText(
        'Saving replaces the existing map of this sector',
      ),
    ).toBeInTheDocument();
  });

  it('should open a new map in the viewer', async () => {
    setupInvokes();
    const { container, findByRole, getByRole, findByText } =
      renderWithTestProviders(<TacticalMaps />);
    await waitFor(() => {
      expect(sectorTile(container, 'A9').classList).toContain('highlighted');
    });

    fireEvent.click(sectorTile(container, 'A9'));
    fireEvent.click(getByRole('button', { name: 'New map' }));
    const dialog = await findByRole('dialog');
    await within(dialog).findByRole('combobox', { name: 'tileset' });
    fireEvent.click(within(dialog).getByText('Basement'));
    fireEvent.click(within(dialog).getByRole('button', { name: 'Create' }));

    await waitFor(() => {
      expect(createMap).toHaveBeenCalledWith('maps/A9.dat', {
        tileset: 0,
        smoothing: 'basement',
      });
    });
    expect(loadMap).not.toHaveBeenCalled();
    expect(await findByText('maps/A9.dat (new)')).toBeInTheDocument();
  });
});
