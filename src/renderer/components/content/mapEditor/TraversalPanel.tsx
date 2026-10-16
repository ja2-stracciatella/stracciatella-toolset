import {
  Alert,
  Flex,
  Input,
  InputNumber,
  Segmented,
  Select,
  Typography,
} from 'antd';
import {
  ChangeEvent,
  Dispatch,
  KeyboardEvent,
  ReactNode,
  useCallback,
  useMemo,
  useState,
} from 'react';
import {
  EDGE_SIDES,
  EdgeSide,
  EditorTraversalEvent,
  EntryPoint,
  SectorId,
  SelectedTraversal,
  setTraversal,
  TraversalEdit,
  TraversalPlace,
  TraversalTool,
} from '../../../lib/mapViewer';
import { BRUSHES, BrushControls } from './BrushControls';
import { EraseControl } from './EraseControl';
import type { BrushKind } from './terrainMode';
import { ToolOption, ToolSelect } from './ToolSelect';
import {
  availableEntryPoints,
  formatSector,
  MAX_GRID_NO,
  MAX_SECTOR_LEVEL,
  parseSector,
  placedEntry,
  TraversalModeAction,
  TraversalModeState,
} from './traversalMode';

const EDGE_HINT =
  'Soldiers enter the map from the edge points of a side. They are generated again after ' +
  'edits that can move them, and saved with the map.';

const CENTER_HINT =
  'The center entry point has no edge points of its own, soldiers enter from the edge points ' +
  'of the side entry points.';

const TOOLS: ToolOption<TraversalTool>[] = [
  {
    label: 'Select',
    value: 'select',
    description: [
      'Selects exit grids and entry points to see them and their edge points, moves, copies and ' +
        'removes them.',
      'Clicking again selects the next one of the tile. Drag to move it, Ctrl+drag (Cmd+drag on ' +
        'macOS) to copy an exit grid, both replace the exit grid of the target tile. Delete ' +
        'removes the selected exit grid or clears the selected entry point, Esc deselects it.',
    ],
  },
  {
    label: 'Place',
    value: 'place',
    description: [
      'Paints exit grids to the destination, or places the chosen entry point. Remove removes ' +
        'the exit grids of the tiles of the brush. Entry points are moved away from the top and ' +
        'bottom rows of the screen, like the game does.',
      'Right click on an exit grid copies its destination.',
    ],
  },
];

const PLACE_OPTIONS: { label: string; value: TraversalPlace }[] = [
  { label: 'Exit grid', value: 'exit' },
  { label: 'Entry point', value: 'entry' },
];

const TRAVERSAL_BRUSHES = BRUSHES.filter(
  (brush) => brush.value === 'square' || brush.value === 'area',
);

const LEVEL_OPTIONS = Array.from({ length: MAX_SECTOR_LEVEL + 1 }, (_, z) => ({
  label: z === 0 ? 'Surface' : `B${z}`,
  value: z,
}));

export const ENTRY_LABELS: Record<EntryPoint, string> = {
  north: 'North',
  east: 'East',
  south: 'South',
  west: 'West',
  center: 'Center',
  isolated: 'Isolated',
};

const NUMBER_STYLE = { width: '100%' } as const;
const TABLE_STYLE = { width: '100%', textAlign: 'left' } as const;

function reportError(e: unknown) {
  console.error('Map editor request failed', e);
}

/** Edits the selected exit grid in the viewer */
function sendTraversalEdit(edit: TraversalEdit) {
  setTraversal(edit).catch(reportError);
}

interface SectorInputProps {
  x: number;
  y: number;
  onChange: (x: number, y: number) => unknown;
  label: string;
}

/** A sector like `J11`, only valid sectors are taken */
function SectorInput({ x, y, onChange, label }: SectorInputProps) {
  const [text, setText] = useState(() => formatSector({ x, y }));
  // Follows the mode, e.g. when an exit grid was copied
  const [shown, setShown] = useState({ x, y });
  if (shown.x !== x || shown.y !== y) {
    setShown({ x, y });
    const parsed = parseSector(text);
    if (!parsed || parsed.x !== x || parsed.y !== y) {
      setText(formatSector({ x, y }));
    }
  }
  const onTextChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const value = event.target.value;
      setText(value);
      const parsed = parseSector(value);
      if (parsed) {
        onChange(parsed.x, parsed.y);
      }
    },
    [onChange],
  );
  const onBlur = useCallback(() => setText(formatSector({ x, y })), [x, y]);
  return (
    <Input
      value={text}
      onChange={onTextChange}
      onBlur={onBlur}
      status={parseSector(text) ? undefined : 'error'}
      placeholder="A1 to P16"
      aria-label={label}
    />
  );
}

interface DestinationFieldsProps {
  sector: SectorId;
  onSectorChange: (x: number, y: number) => unknown;
  onLevelChange: (level: number) => unknown;
  /** What is shown for the grid number */
  gridNo: ReactNode;
  /** Prefix of the labels, e.g. `destination` */
  label: string;
}

/** The sector, level and grid number of a destination */
function DestinationFields({
  sector,
  onSectorChange,
  onLevelChange,
  gridNo,
  label,
}: DestinationFieldsProps) {
  return (
    <Flex vertical gap="small">
      <Flex gap="small">
        <SectorInput
          x={sector.x}
          y={sector.y}
          onChange={onSectorChange}
          label={`${label} sector`}
        />
        <Select<number>
          options={LEVEL_OPTIONS}
          value={sector.z}
          onChange={onLevelChange}
          aria-label={`${label} level`}
        />
      </Flex>
      <Typography.Text>Grid number</Typography.Text>
      {gridNo}
    </Flex>
  );
}

interface GridNoInputProps {
  gridNo: number;
  onCommit: (gridNo: number) => unknown;
  label: string;
}

/**
 * A grid number that is taken when the input loses the focus or Enter is pressed, so typing
 * doesn't make an undo step per key. Keyed by the grid number, so it follows the viewer.
 */
function GridNoInput({ gridNo, onCommit, label }: GridNoInputProps) {
  const [value, setValue] = useState<number | null>(gridNo);
  const commit = useCallback(() => {
    if (value !== null && value !== gridNo) {
      onCommit(value);
    }
  }, [gridNo, onCommit, value]);
  const onKeyDown = useCallback(
    (event: KeyboardEvent<HTMLInputElement>) => {
      if (event.key === 'Enter') {
        commit();
      }
    },
    [commit],
  );
  return (
    <InputNumber<number>
      min={0}
      max={MAX_GRID_NO}
      precision={0}
      value={value}
      onChange={setValue}
      onBlur={commit}
      onKeyDown={onKeyDown}
      style={NUMBER_STYLE}
      aria-label={label}
    />
  );
}

type SelectedExitGridValue = Extract<SelectedTraversal, { kind: 'exitGrid' }>;

/** The destination of the selected exit grid */
function SelectedExitGrid({ grid }: { grid: SelectedExitGridValue }) {
  const { gridNo, destSector, destGridNo } = grid;
  const send = useCallback(
    (sector: SectorId, dest: number) =>
      sendTraversalEdit({
        kind: 'exitGrid',
        gridNo,
        destSector: sector,
        destGridNo: dest,
      }),
    [gridNo],
  );
  const onSectorChange = useCallback(
    (x: number, y: number) => send({ ...destSector, x, y }, destGridNo),
    [destGridNo, destSector, send],
  );
  const onLevelChange = useCallback(
    (z: number) => send({ ...destSector, z }, destGridNo),
    [destGridNo, destSector, send],
  );
  const onGridNoCommit = useCallback(
    (dest: number) => send(destSector, dest),
    [destSector, send],
  );
  return (
    <Flex vertical gap="small" role="region" aria-label="selected exit grid">
      <Typography.Text strong>{`Exit grid at tile ${gridNo}`}</Typography.Text>
      <DestinationFields
        sector={destSector}
        onSectorChange={onSectorChange}
        onLevelChange={onLevelChange}
        label="selected destination"
        gridNo={
          <GridNoInput
            key={destGridNo}
            gridNo={destGridNo}
            onCommit={onGridNoCommit}
            label="selected destination grid number"
          />
        }
      />
    </Flex>
  );
}

const SIDE_LABELS: Record<EdgeSide, string> = {
  north: 'North',
  east: 'East',
  south: 'South',
  west: 'West',
};

type EdgePointsValue = EditorTraversalEvent['edgePoints'] | undefined;

interface EdgePointsProps {
  /** The sides whose edge points are listed */
  sides: readonly EdgeSide[];
  /** Whether the primary or the isolated (secondary) numbers are listed */
  sets: readonly ('primary' | 'secondary')[];
  edgePoints: EdgePointsValue;
}

const SET_LABELS = { primary: 'Primary', secondary: 'Isolated' } as const;

/** The numbers of edge points of the sides, and the ones soldiers can't enter */
function EdgePoints({ sides, sets, edgePoints }: EdgePointsProps) {
  const missing = edgePoints?.missing;
  const warning = useMemo(() => {
    const shown = sets.includes('primary')
      ? missing?.filter((side) => sides.includes(side))
      : undefined;
    return shown && shown.length > 0
      ? `No edge points on the ${shown
          .map((side) => SIDE_LABELS[side].toLowerCase())
          .join(', ')} side: soldiers can't enter the map from there.`
      : null;
  }, [missing, sets, sides]);
  return (
    <Flex vertical gap="small">
      <table style={TABLE_STYLE} aria-label="edge points">
        <thead>
          <tr>
            <th>Side</th>
            {sets.map((set) => (
              <th key={set}>{SET_LABELS[set]}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {sides.map((side) => (
            <tr key={side}>
              <td>{SIDE_LABELS[side]}</td>
              {sets.map((set) => (
                <td key={set}>{edgePoints?.[set][side] ?? '-'}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {warning ? <Alert type="warning" showIcon message={warning} /> : null}
      {edgePoints?.pending ? (
        <Typography.Text type="secondary">Updating…</Typography.Text>
      ) : null}
      <Typography.Text type="secondary">{EDGE_HINT}</Typography.Text>
    </Flex>
  );
}

const BOTH_SETS = ['primary', 'secondary'] as const;
const SECONDARY_SET = ['secondary'] as const;

type SelectedEntryPointValue = Extract<SelectedTraversal, { kind: 'entry' }>;

interface SelectedEntryPointProps {
  entry: SelectedEntryPointValue;
  edgePoints: EdgePointsValue;
}

/** The selected entry point and the edge points soldiers entering there use */
function SelectedEntryPoint({ entry, edgePoints }: SelectedEntryPointProps) {
  const { point } = entry;
  const sides = useMemo(
    () => (point === 'isolated' || point === 'center' ? EDGE_SIDES : [point]),
    [point],
  );
  let details;
  if (point === 'center') {
    details = <Typography.Text type="secondary">{CENTER_HINT}</Typography.Text>;
  } else {
    details = (
      <EdgePoints
        sides={sides}
        sets={point === 'isolated' ? SECONDARY_SET : BOTH_SETS}
        edgePoints={edgePoints}
      />
    );
  }
  return (
    <Flex vertical gap="small" role="region" aria-label="selected entry point">
      <Typography.Text
        strong
      >{`${ENTRY_LABELS[point]} entry point`}</Typography.Text>
      {details}
    </Flex>
  );
}

export interface TraversalPanelProps {
  state: TraversalModeState;
  dispatch: Dispatch<TraversalModeAction>;
  /** The exit grids, entry points and edge points, `null` until the map is loaded */
  traversalEvent: EditorTraversalEvent | null;
}

/** The traversal mode: exit grids, entry points and their edge points */
export function TraversalPanel({
  state,
  dispatch,
  traversalEvent,
}: TraversalPanelProps) {
  const onToolChange = useCallback(
    (tool: TraversalTool) => dispatch({ type: 'setTool', tool }),
    [dispatch],
  );
  const onPlaceChange = useCallback(
    (place: TraversalPlace) => dispatch({ type: 'setPlace', place }),
    [dispatch],
  );
  const onEraseChange = useCallback(
    (erase: boolean) => dispatch({ type: 'setErase', erase }),
    [dispatch],
  );
  const onBrushChange = useCallback(
    (brush: BrushKind) => {
      if (brush === 'square' || brush === 'area') {
        dispatch({ type: 'setBrush', brush });
      }
    },
    [dispatch],
  );
  const onSizeChange = useCallback(
    (size: number) => dispatch({ type: 'setSize', size }),
    [dispatch],
  );
  const onSectorChange = useCallback(
    (x: number, y: number) => dispatch({ type: 'setSector', x, y }),
    [dispatch],
  );
  const onLevelChange = useCallback(
    (level: number) => dispatch({ type: 'setLevel', level }),
    [dispatch],
  );
  const onGridNoChange = useCallback(
    (gridNo: number | null) => {
      if (gridNo !== null) {
        dispatch({ type: 'setDestGridNo', gridNo });
      }
    },
    [dispatch],
  );
  const onEntryChange = useCallback(
    (entry: EntryPoint) => dispatch({ type: 'setEntry', entry }),
    [dispatch],
  );

  const entryPoints = traversalEvent?.entryPoints;
  const entry = placedEntry(state, entryPoints);
  const entryOptions = useMemo(
    () =>
      availableEntryPoints(entryPoints).map((point) => ({
        label: ENTRY_LABELS[point],
        value: point,
      })),
    [entryPoints],
  );
  // Without entry points to place, the place tool paints exit grids
  const place = entry === null ? 'exit' : state.place;

  let tool;
  if (state.tool === 'select') {
    const selected = traversalEvent?.selected ?? null;
    let selection;
    if (selected?.kind === 'exitGrid') {
      selection = <SelectedExitGrid key={selected.gridNo} grid={selected} />;
    } else if (selected?.kind === 'entry') {
      selection = (
        <SelectedEntryPoint
          key={selected.point}
          entry={selected}
          edgePoints={traversalEvent?.edgePoints}
        />
      );
    } else {
      selection = (
        <Typography.Text type="secondary">Nothing selected</Typography.Text>
      );
    }
    tool = (
      <Flex vertical gap="small">
        <Typography.Text type="secondary">
          {`${traversalEvent?.exitGrids ?? 0} exit grids`}
        </Typography.Text>
        <Typography.Text strong>Selected</Typography.Text>
        {selection}
      </Flex>
    );
  } else if (place === 'entry' && entry !== null) {
    tool = (
      <Flex vertical gap="small">
        <Typography.Text strong>Entry point</Typography.Text>
        <Segmented<EntryPoint>
          block
          options={entryOptions}
          value={entry}
          onChange={onEntryChange}
          aria-label="entry point to place"
        />
      </Flex>
    );
  } else {
    tool = (
      <Flex vertical gap="small">
        <EraseControl erase={state.erase} onChange={onEraseChange} />
        <BrushControls
          brushes={TRAVERSAL_BRUSHES}
          brush={state.brush}
          size={state.size}
          onBrushChange={onBrushChange}
          onSizeChange={onSizeChange}
        />
        {state.erase ? null : (
          <>
            <Typography.Text strong>Destination</Typography.Text>
            <DestinationFields
              sector={state.destSector}
              onSectorChange={onSectorChange}
              onLevelChange={onLevelChange}
              label="destination"
              gridNo={
                <InputNumber<number>
                  min={0}
                  max={MAX_GRID_NO}
                  precision={0}
                  value={state.destGridNo}
                  onChange={onGridNoChange}
                  style={NUMBER_STYLE}
                  aria-label="destination grid number"
                />
              }
            />
          </>
        )}
      </Flex>
    );
  }
  return (
    <Flex vertical gap="small">
      <ToolSelect<TraversalTool>
        tools={TOOLS}
        value={state.tool}
        onChange={onToolChange}
        label="traversal tool"
      />
      {state.tool === 'place' && entry !== null ? (
        <Segmented<TraversalPlace>
          block
          options={PLACE_OPTIONS}
          value={place}
          onChange={onPlaceChange}
          aria-label="what to place"
        />
      ) : null}
      {tool}
    </Flex>
  );
}
