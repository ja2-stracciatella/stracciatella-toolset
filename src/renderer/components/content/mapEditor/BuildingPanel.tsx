import {
  Checkbox,
  type CheckboxChangeEvent,
  Flex,
  InputNumber,
  Typography,
} from 'antd';
import { Dispatch, useCallback, useMemo } from 'react';
import {
  BuildingPalettes,
  BuildingTool,
  EditorPaletteEvent,
  MapSmoothing,
  PaletteEntry,
  SmartValue,
} from '../../../lib/mapViewer';
import { BRUSHES, BrushControls } from './BrushControls';
import { EraseControl } from './EraseControl';
import {
  allowsTool,
  BuildingModeAction,
  BuildingModeState,
  canErase,
  hasSlantRoofOnly,
  isFloorsOnly,
  isSmart,
  MAX_ROOM_NUMBER,
  SelectionPalette,
  selectedKinds,
  TILE_PALETTES,
  toolPalette,
  usesBrush,
  usesPalette,
} from './buildingMode';
import { TileGroup, TilePalette } from './TilePalette';
import { ToolDescription, ToolOption, ToolSelect } from './ToolSelect';
import type { BrushKind } from './terrainMode';

/** The tools, in the order of the buttons of the original's buildings taskbar */
export const TOOLS: ToolOption<BuildingTool>[] = [
  {
    label: 'Tiles',
    value: 'tiles',
    description: [
      'Places the selected building tiles: walls, doors, windows and roofs on the clicked tile, ' +
        'floors with any brush, decals only onto walls and fences. Remove erases the layers of ' +
        'the selected tiles.',
    ],
  },
  {
    label: 'Smart',
    value: 'smart',
    description: [
      'Places walls that fit the walls around them, doors, windows and broken walls that ' +
        'replace the walls of the tile, and roofs that replace the roof of the clicked building. ' +
        'Remove restores doors, windows and broken walls and removes walls.',
    ],
  },
  {
    label: 'Room',
    value: 'room',
    description: [
      'Adds the area of the brush as a room with walls, floor and roof, or extends the building ' +
        'it overlaps. Remove removes the area from the buildings and closes them with walls.',
    ],
  },
  {
    label: 'Select',
    value: 'select',
    description: [
      'Selects, moves, copies and removes whole buildings. Room numbers are not copied.',
      'Drag a building to move it, Ctrl+drag (Cmd+drag on macOS) to copy it. Delete removes the ' +
        'selected building.',
    ],
  },
  {
    label: 'Caves',
    value: 'caves',
    description: [
      'Fills the area of the brush with rock.',
      'Right click carves a cave out of the rock.',
    ],
  },
  {
    label: 'Locks',
    value: 'doorLocks',
    description: [
      'Edits the lock and trap of the clicked door. Doors with locks have a key.',
    ],
  },
  {
    label: 'Room numbers',
    value: 'roomNumber',
    description: [
      'Gives the tiles of the brush the room number, which tells the game which tiles belong to ' +
        'a room. A new room gets the next free number. Remove clears the room numbers.',
    ],
  },
];

/** Buildings can't be moved or copied in basements */
const BASEMENT_SELECT: ToolDescription = [
  "Selects and removes whole buildings. Buildings can't be moved or copied in basements.",
  'Delete removes the selected building.',
];

/** The smart palettes, shown as the groups of the smart palette */
const SMART_GROUPS: [keyof BuildingPalettes, string][] = [
  ['smartWalls', 'Walls'],
  ['smartDoors', 'Doors'],
  ['smartWindows', 'Windows'],
  ['smartBrokenWalls', 'Broken walls'],
  ['smartRoofs', 'Roofs'],
];

interface SmartPickerProps {
  palettes: BuildingPalettes;
  smart: SmartValue | null;
  smoothing: MapSmoothing;
  dispatch: Dispatch<BuildingModeAction>;
}

/**
 * Picks the value of the smart tool from the smart palettes. Roofs can only be replaced on normal
 * maps, so the others don't show them.
 */
function SmartPicker({
  palettes,
  smart,
  smoothing,
  dispatch,
}: SmartPickerProps) {
  const groups = useMemo(
    () =>
      SMART_GROUPS.filter(
        ([palette]) => palette !== 'smartRoofs' || smoothing === 'normal',
      ).map(([palette, label]) => ({
        label,
        entries: palettes[palette],
      })),
    [palettes, smoothing],
  );
  const entries = useMemo(
    () => groups.flatMap((group) => group.entries),
    [groups],
  );
  const hole = smart?.kind === 'hole';
  const selected =
    smart?.kind === 'tile'
      ? entries.findIndex((e) => e.tileId === smart.tileId)
      : -1;
  const onSelect = useCallback(
    (index: number) => {
      const entry = entries[index];
      if (entry) {
        dispatch({
          type: 'setSmart',
          smart: { kind: 'tile', tileId: entry.tileId },
        });
      }
    },
    [dispatch, entries],
  );
  const onHoleChange = useCallback(
    (e: CheckboxChangeEvent) => {
      const first = palettes.smartBrokenWalls[0] ?? entries[0];
      dispatch({
        type: 'setSmart',
        smart: e.target.checked
          ? { kind: 'hole' }
          : first
            ? { kind: 'tile', tileId: first.tileId }
            : null,
      });
    },
    [dispatch, entries, palettes],
  );
  return (
    <Flex vertical gap="small">
      <Checkbox checked={hole} onChange={onHoleChange}>
        Hole in the wall
      </Checkbox>
      <TilePalette
        mode="single"
        label="smart"
        groups={groups}
        selected={selected}
        onSelect={onSelect}
      />
    </Flex>
  );
}

/** The kinds of tiles of the room palette, by the start of their type names */
const ROOM_KINDS: [string, string][] = [
  ['WALL', 'Walls'],
  ['FLOOR', 'Floors'],
  ['SROOF', 'Slant roofs'],
  ['ROOF', 'Roofs'],
];

/**
 * The tiles of a palette, grouped by their tile types or kinds. The tiles tool merges its
 * palettes, grouped by kind and tile type.
 */
export function paletteGroups(
  palettes: BuildingPalettes,
  palette: SelectionPalette,
): TileGroup[] {
  const groups: TileGroup[] = [];
  const add = (label: string, entry: PaletteEntry) => {
    const last = groups[groups.length - 1];
    if (last?.label === label) {
      last.entries.push(entry);
    } else {
      groups.push({ label, entries: [entry] });
    }
  };
  if (palette === 'tiles') {
    for (const [kind, label] of TILE_PALETTES) {
      for (const entry of palettes[kind]) {
        add(`${label} · ${entry.typeName}`, entry);
      }
    }
    return groups;
  }
  for (const entry of palettes[palette]) {
    if (palette === 'room') {
      const kind = ROOM_KINDS.find(([prefix]) =>
        entry.typeName.startsWith(prefix),
      );
      add(kind?.[1] ?? entry.typeName, entry);
    } else {
      add(entry.typeName, entry);
    }
  }
  return groups;
}

export interface BuildingPanelProps {
  state: BuildingModeState;
  dispatch: Dispatch<BuildingModeAction>;
  /** `null` until the map is loaded */
  palette: EditorPaletteEvent | null;
  /** How the editor smooths the map */
  smoothing: MapSmoothing;
}

/** Controls of the buildings mode: the tool, the brush and the tiles */
export function BuildingPanel({
  state,
  dispatch,
  palette,
  smoothing,
}: BuildingPanelProps) {
  const { tool } = state;
  const selectionPalette = toolPalette(tool);
  const onToolChange = useCallback(
    (value: BuildingTool) => dispatch({ type: 'setTool', tool: value }),
    [dispatch],
  );
  const onEraseChange = useCallback(
    (erase: boolean) => dispatch({ type: 'setErase', erase }),
    [dispatch],
  );
  const onBrushChange = useCallback(
    (brush: BrushKind) => dispatch({ type: 'setBrush', brush }),
    [dispatch],
  );
  const onSizeChange = useCallback(
    (size: number) => dispatch({ type: 'setSize', size }),
    [dispatch],
  );
  const onSelectTile = useCallback(
    (tileId: number) =>
      dispatch({ type: 'selectTile', palette: selectionPalette, tileId }),
    [dispatch, selectionPalette],
  );
  const onAddTile = useCallback(
    (tileId: number) =>
      dispatch({ type: 'addTile', palette: selectionPalette, tileId }),
    [dispatch, selectionPalette],
  );
  const onRemoveTile = useCallback(
    (tileId: number) =>
      dispatch({ type: 'removeTile', palette: selectionPalette, tileId }),
    [dispatch, selectionPalette],
  );
  const onRoomNumberChange = useCallback(
    (roomNumber: number | null) =>
      dispatch({ type: 'setRoomNumber', roomNumber: roomNumber ?? 0 }),
    [dispatch],
  );
  const onClearTiles = useCallback(
    () => dispatch({ type: 'clearTiles', palette: selectionPalette }),
    [dispatch, selectionPalette],
  );

  const tools = useMemo(
    () =>
      TOOLS.filter((option) => allowsTool(smoothing, option.value)).map(
        (option) =>
          option.value === 'select' && smoothing === 'basement'
            ? { ...option, description: BASEMENT_SELECT }
            : option,
      ),
    [smoothing],
  );
  const groups = useMemo(
    () => (palette ? paletteGroups(palette.buildings, selectionPalette) : []),
    [palette, selectionPalette],
  );
  const floorsOnly = useMemo(
    () =>
      palette !== null &&
      isFloorsOnly(selectedKinds(state.selections.tiles, palette.buildings)),
    [palette, state.selections.tiles],
  );
  const erasable = canErase(tool);
  const erasing = state.erase && erasable;
  const slantRoof =
    tool === 'room' &&
    !erasing &&
    palette !== null &&
    hasSlantRoofOnly(state.selections.room, palette.buildings.room);

  let tiles = null;
  if (!palette) {
    tiles = (
      <Typography.Text type="secondary">
        The tiles are shown once the map is loaded.
      </Typography.Text>
    );
  } else if (tool === 'roomNumber' && !erasing) {
    tiles = (
      <Flex gap="small" align="center">
        <span>Room</span>
        <InputNumber
          size="small"
          min={0}
          max={MAX_ROOM_NUMBER}
          precision={0}
          value={state.roomNumber}
          onChange={onRoomNumberChange}
          aria-label="room number"
        />
      </Flex>
    );
  } else if (isSmart(tool) && !erasing) {
    tiles = (
      <SmartPicker
        palettes={palette.buildings}
        smart={state.smart}
        smoothing={smoothing}
        dispatch={dispatch}
      />
    );
  } else if (usesPalette(tool) && (!erasing || tool === 'tiles')) {
    // Erasing with the tiles clears the layers of the selected tiles, so they stay visible
    tiles = (
      <TilePalette
        mode="counts"
        label={selectionPalette}
        groups={groups}
        selections={state.selections[selectionPalette]}
        onSelect={onSelectTile}
        onAdd={onAddTile}
        onRemove={onRemoveTile}
        onClear={onClearTiles}
      />
    );
  }

  return (
    <Flex vertical gap="middle">
      <Flex vertical gap="small">
        <ToolSelect<BuildingTool>
          tools={tools}
          value={tool}
          onChange={onToolChange}
          label="tool"
        />
        {slantRoof ? (
          <Typography.Text type="secondary">
            Slant roofs are 8 tiles wide, the brush is a line 8 tiles wide.
          </Typography.Text>
        ) : null}
      </Flex>

      {erasable ? (
        <EraseControl erase={erasing} onChange={onEraseChange} />
      ) : null}

      {!slantRoof && usesBrush(tool, state.erase, floorsOnly) ? (
        <Flex vertical gap="small">
          <BrushControls
            brushes={BRUSHES}
            brush={state.brush}
            size={state.size}
            onBrushChange={onBrushChange}
            onSizeChange={onSizeChange}
          />
        </Flex>
      ) : null}

      {tiles}
    </Flex>
  );
}
