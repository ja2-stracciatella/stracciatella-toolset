import {
  DownOutlined,
  EyeInvisibleOutlined,
  EyeOutlined,
  RightOutlined,
} from '@ant-design/icons';
import { Button, Divider, Flex, Segmented, Typography } from 'antd';
import {
  Fragment,
  ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import {
  DoorLock,
  EditorDoorEvent,
  EditorItemEvent,
  EditorLightEvent,
  EditorMode,
  EditorPaletteEvent,
  EditorSoldierEvent,
  EditorTraversalEvent,
  ExitGrid,
  MapFloor,
  MapLayers,
  MapSmoothing,
  MapLighting,
  MapRenderOptions,
  roofAllRooms,
  setDoor,
  setEditorMode,
  setMapBackground,
  setMapSmoothing,
  setRestrictedScroll,
  SOLDIER_TEAMS,
  SoldierPaletteEvent,
  SoldierTeam,
  subscribeEditorEvents,
} from '../../../lib/mapViewer';
import { BuildingPanel } from './BuildingPanel';
import { DoorLockModal } from './DoorLockModal';
import { ItemsPanel } from './ItemsPanel';
import { toItemMode, useItemMode } from './itemMode';
import { LightsPanel } from './LightsPanel';
import { toLightMode, useLightMode } from './lightMode';
import { useLockTable } from './lockTable';
import { OverviewPanel } from './OverviewPanel';
import {
  allowsTool,
  BuildingViews,
  buildingViews,
  isSmartRoof,
  selectedKinds,
  toBuildingMode,
  useBuildingMode,
} from './buildingMode';
import { TOOLS } from './BuildingPanel';
import { SoldiersPanel, TEAM_LABELS } from './SoldiersPanel';
import { toSoldierMode, useSoldierMode } from './soldierMode';
import { TerrainPanel } from './TerrainPanel';
import { toEditorMode, useTerrainMode } from './terrainMode';
import { TraversalPanel } from './TraversalPanel';
import {
  placedEntry,
  toTraversalMode,
  useTraversalMode,
} from './traversalMode';

/** The layers that are shown or hidden as a whole */
type ToggledLayer = Exclude<keyof MapLayers, 'soldierTeams'>;

const LAYER_LABELS: [ToggledLayer, string][] = [
  ['terrain', 'Terrain'],
  ['objects', 'Objects'],
  ['buildings', 'Buildings'],
  ['soldiers', 'Soldiers'],
  ['items', 'Items'],
  ['roomNumbers', 'Room numbers'],
];

const FLOOR_OPTIONS: { label: string; value: MapFloor }[] = [
  { label: 'Ground', value: 'ground' },
  { label: 'Roof', value: 'roof' },
];

/** The modes that only edit the ground, see `is_ground_only` in `src-map-editor` */
const GROUND_ONLY_MODES: EditorModeName[] = [
  'terrain',
  'objects',
  'lights',
  'traversal',
];

const GROUND_ONLY_HINT = 'These tools only edit the ground floor.';

const LIGHTING_OPTIONS: { label: string; value: MapLighting }[] = [
  { label: 'Day', value: 'day' },
  { label: 'Night', value: 'night' },
];

/** Only viewing, or what the mouse places */
export type EditorModeName = EditorMode['mode'];

export const MODE_OPTIONS: { label: string; value: EditorModeName }[] = [
  { label: 'Overview', value: 'view' },
  { label: 'Terrain', value: 'terrain' },
  { label: 'Objects', value: 'objects' },
  { label: 'Buildings', value: 'buildings' },
  { label: 'Lights', value: 'lights' },
  { label: 'Traversal', value: 'traversal' },
  { label: 'Soldiers', value: 'soldiers' },
  { label: 'Items', value: 'items' },
];

const NO_GROUPS: string[] = [];

const MODE_LABELS = Object.fromEntries(
  MODE_OPTIONS.map(({ label, value }) => [value, label]),
) as Record<EditorModeName, string>;

const TEAMS_STYLE = { paddingLeft: 24 } as const;

interface VisibilityToggleProps {
  label: string;
  visible: boolean;
  /** Shows the label as hidden, while what contains it is hidden */
  dimmed?: boolean;
  onToggle: () => unknown;
  /** Shown before the label */
  prefix?: ReactNode;
}

/** A label with an eye to show or hide what it names */
function VisibilityToggle({
  label,
  visible,
  dimmed = false,
  onToggle,
  prefix,
}: VisibilityToggleProps) {
  return (
    <Flex justify="space-between" align="center">
      <Flex align="center" gap={4}>
        {prefix}
        <Typography.Text type={visible && !dimmed ? undefined : 'secondary'}>
          {label}
        </Typography.Text>
      </Flex>
      <Button
        type="text"
        size="small"
        icon={visible ? <EyeOutlined /> : <EyeInvisibleOutlined />}
        onClick={onToggle}
        role="switch"
        aria-checked={visible}
        aria-label={label}
      />
    </Flex>
  );
}

interface LayerToggleProps {
  layer: ToggledLayer;
  label: string;
  visible: boolean;
  onChange: (layer: ToggledLayer, visible: boolean) => unknown;
  /** Shown before the label */
  prefix?: ReactNode;
}

/** A layer with an eye to show or hide it */
function LayerToggle({
  layer,
  label,
  visible,
  onChange,
  prefix,
}: LayerToggleProps) {
  const onToggle = useCallback(
    () => onChange(layer, !visible),
    [layer, onChange, visible],
  );
  return (
    <VisibilityToggle
      label={label}
      visible={visible}
      onToggle={onToggle}
      prefix={prefix}
    />
  );
}

interface TeamToggleProps {
  team: SoldierTeam;
  visible: boolean;
  /** Whether the whole soldiers layer is hidden */
  dimmed: boolean;
  onChange: (team: SoldierTeam, visible: boolean) => unknown;
}

/** A team of the soldiers layer with an eye to show or hide its soldiers */
function TeamToggle({ team, visible, dimmed, onChange }: TeamToggleProps) {
  const onToggle = useCallback(
    () => onChange(team, !visible),
    [onChange, team, visible],
  );
  return (
    <VisibilityToggle
      label={TEAM_LABELS[team]}
      visible={visible}
      dimmed={dimmed}
      onToggle={onToggle}
    />
  );
}

export interface MapRenderToolbarProps {
  options: MapRenderOptions;
  onChange: (options: MapRenderOptions) => unknown;
}

/**
 * Controls which layers of the map are shown, the floor that is shown and edited, and the
 * lighting
 */
export function MapRenderToolbar({ options, onChange }: MapRenderToolbarProps) {
  const [teamsExpanded, setTeamsExpanded] = useState(false);
  const onLayerChange = useCallback(
    (layer: ToggledLayer, visible: boolean) =>
      onChange({ ...options, layers: { ...options.layers, [layer]: visible } }),
    [onChange, options],
  );
  const onTeamChange = useCallback(
    (team: SoldierTeam, visible: boolean) =>
      onChange({
        ...options,
        layers: {
          ...options.layers,
          soldierTeams: { ...options.layers.soldierTeams, [team]: visible },
        },
      }),
    [onChange, options],
  );
  const onExpandTeams = useCallback(
    () => setTeamsExpanded((expanded) => !expanded),
    [],
  );
  const teamsCaret = useMemo(
    () => (
      <Button
        type="text"
        size="small"
        icon={teamsExpanded ? <DownOutlined /> : <RightOutlined />}
        onClick={onExpandTeams}
        aria-expanded={teamsExpanded}
        aria-label="Soldier teams"
      />
    ),
    [onExpandTeams, teamsExpanded],
  );
  const onFloorChange = useCallback(
    (floor: MapFloor) => onChange({ ...options, floor }),
    [onChange, options],
  );
  const onLightingChange = useCallback(
    (lighting: MapLighting) => onChange({ ...options, lighting }),
    [onChange, options],
  );
  return (
    <Flex vertical gap="small">
      <Flex vertical gap={0} role="group" aria-label="layers">
        {LAYER_LABELS.map(([layer, label]) => (
          <Fragment key={layer}>
            <LayerToggle
              layer={layer}
              label={label}
              visible={options.layers[layer]}
              onChange={onLayerChange}
              prefix={layer === 'soldiers' ? teamsCaret : undefined}
            />
            {layer === 'soldiers' && teamsExpanded ? (
              <Flex
                vertical
                gap={0}
                role="group"
                aria-label="soldier teams"
                style={TEAMS_STYLE}
              >
                {SOLDIER_TEAMS.map((team) => (
                  <TeamToggle
                    key={team}
                    team={team}
                    visible={options.layers.soldierTeams[team]}
                    dimmed={!options.layers.soldiers}
                    onChange={onTeamChange}
                  />
                ))}
              </Flex>
            ) : null}
          </Fragment>
        ))}
      </Flex>
      <Segmented<MapFloor>
        block
        options={FLOOR_OPTIONS}
        value={options.floor}
        onChange={onFloorChange}
        aria-label="floor"
      />
      <Segmented<MapLighting>
        block
        options={LIGHTING_OPTIONS}
        value={options.lighting}
        onChange={onLightingChange}
        aria-label="lighting"
      />
    </Flex>
  );
}

function reportError(e: unknown) {
  console.error('Map editor request failed', e);
}

function sameExitGrid(a: ExitGrid | null, b: ExitGrid | null) {
  return (
    a === b ||
    (a !== null &&
      b !== null &&
      a.gridNo === b.gridNo &&
      a.destGridNo === b.destGridNo &&
      a.destSector.x === b.destSector.x &&
      a.destSector.y === b.destSector.y &&
      a.destSector.z === b.destSector.z)
  );
}

export interface MapEditorSidebarProps {
  /** The map shown by the viewer, events of other maps are ignored */
  file: string;
  mode: EditorModeName;
  renderOptions: MapRenderOptions;
  onRenderOptionsChange: (options: MapRenderOptions) => unknown;
}

/** The controls of the map viewer: what it shows and the editing modes */
export function MapEditorSidebar({
  file,
  mode,
  renderOptions,
  onRenderOptionsChange,
}: MapEditorSidebarProps) {
  const [terrain, terrainDispatch] = useTerrainMode();
  const [building, buildingDispatch] = useBuildingMode();
  const [soldier, soldierDispatch] = useSoldierMode();
  const [item, itemDispatch] = useItemMode();
  const [light, lightDispatch] = useLightMode();
  const [traversal, traversalDispatch] = useTraversalMode();
  const [palette, setPalette] = useState<EditorPaletteEvent | null>(null);
  const [soldierPalette, setSoldierPalette] =
    useState<SoldierPaletteEvent | null>(null);
  const [soldierEvent, setSoldierEvent] = useState<EditorSoldierEvent | null>(
    null,
  );
  const [itemEvent, setItemEvent] = useState<EditorItemEvent | null>(null);
  const [lightEvent, setLightEvent] = useState<EditorLightEvent | null>(null);
  const [traversalEvent, setTraversalEvent] =
    useState<EditorTraversalEvent | null>(null);
  // The exit grid that was right clicked last, its values were copied into the mode
  const lastQueried = useRef<ExitGrid | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  // The previous soldier event, to follow what changed in the viewer
  const lastSoldierEvent = useRef<EditorSoldierEvent | null>(null);
  const [nextRoomNumber, setNextRoomNumber] = useState<number | null>(null);
  const [smoothing, setSmoothing] = useState<MapSmoothing | null>(null);
  const [restrictedScroll, setRestrictedScrollState] = useState(false);
  const [doorRequest, setDoorRequest] = useState<EditorDoorEvent | null>(null);
  // A new modal for every request
  const [doorRequests, setDoorRequests] = useState(0);
  const renderOptionsRef = useRef(renderOptions);
  useEffect(() => {
    renderOptionsRef.current = renderOptions;
  }, [renderOptions]);

  // New rooms get the next free number, it goes up after a new room is numbered
  useEffect(() => {
    if (nextRoomNumber !== null) {
      buildingDispatch({ type: 'setRoomNumber', roomNumber: nextRoomNumber });
    }
  }, [buildingDispatch, nextRoomNumber]);

  useEffect(() => {
    return subscribeEditorEvents((event) => {
      if (event.map !== file) {
        return;
      }
      if (event.type === 'palette') {
        setPalette(event);
        terrainDispatch({ type: 'initSelections', palette: event });
        buildingDispatch({ type: 'initSelections', palette: event });
      } else if (event.type === 'door') {
        setDoorRequest(event);
        setDoorRequests((n) => n + 1);
      } else if (event.type === 'soldierPalette') {
        setSoldierPalette(event);
      } else if (event.type === 'soldier') {
        const last = lastSoldierEvent.current;
        lastSoldierEvent.current = event;
        setSoldierEvent(event);
        // Selecting a soldier switches to its team, like the original
        const selected = event.soldier;
        if (selected?.team && selected.id !== last?.soldier?.id) {
          soldierDispatch({ type: 'setTeam', team: selected.team });
        }
      } else if (event.type === 'item') {
        setItemEvent(event);
      } else if (event.type === 'light') {
        setLightEvent(event);
      } else if (event.type === 'traversal') {
        setTraversalEvent(event);
        // A right click on an exit grid copies its destination, like the original
        if (
          event.queried &&
          !sameExitGrid(lastQueried.current, event.queried)
        ) {
          traversalDispatch({ type: 'copyExitGrid', exitGrid: event.queried });
        }
        lastQueried.current = event.queried;
      } else if (event.type === 'notice') {
        setNotice(event.message);
      } else if (event.type === 'floor') {
        // Page Up and Page Down in the viewer
        const current = renderOptionsRef.current;
        if (current.floor !== event.floor) {
          onRenderOptionsChange({ ...current, floor: event.floor });
        }
      } else {
        setNextRoomNumber(event.nextRoomNumber);
        setSmoothing(event.smoothing);
        setRestrictedScrollState(event.restrictedScroll);
      }
    });
  }, [
    buildingDispatch,
    file,
    onRenderOptionsChange,
    soldierDispatch,
    terrainDispatch,
    traversalDispatch,
  ]);

  // The map type decides the tools, switch to one it allows
  useEffect(() => {
    if (smoothing === null || allowsTool(smoothing, building.tool)) {
      return;
    }
    const allowed = TOOLS.find((option) => allowsTool(smoothing, option.value));
    if (allowed) {
      buildingDispatch({ type: 'setTool', tool: allowed.value });
    }
  }, [building.tool, buildingDispatch, smoothing]);

  const placedEntryPoint = placedEntry(traversal, traversalEvent?.entryPoints);

  // The viewer only edits in the editing modes
  useEffect(() => {
    let editorMode: EditorMode;
    if (mode === 'view') {
      editorMode = { mode: 'view' };
    } else if (mode === 'buildings') {
      editorMode = toBuildingMode(building);
    } else if (mode === 'soldiers') {
      editorMode = toSoldierMode(soldier);
    } else if (mode === 'items') {
      editorMode = toItemMode(item);
    } else if (mode === 'lights') {
      editorMode = toLightMode(light);
    } else if (mode === 'traversal') {
      editorMode = toTraversalMode(traversal, placedEntryPoint);
    } else {
      editorMode = toEditorMode(terrain, mode);
    }
    setEditorMode(editorMode).catch((e: unknown) => {
      console.error('Could not set the editor mode', e);
    });
  }, [
    building,
    item,
    light,
    mode,
    placedEntryPoint,
    soldier,
    terrain,
    traversal,
  ]);

  // The lock names of the keys and the door locks
  const locks = useLockTable(mode === 'items' || doorRequest !== null);

  // The building tools show the views they need, like the original editor
  const roofsSelected = useMemo(() => {
    if (palette === null) {
      return false;
    }
    if (building.tool === 'smart') {
      return isSmartRoof(building.smart, palette.buildings);
    }
    return selectedKinds(building.selections.tiles, palette.buildings).has(
      'roofs',
    );
  }, [building.selections.tiles, building.smart, building.tool, palette]);
  useEffect(() => {
    if (mode !== 'buildings') {
      return;
    }
    const current = renderOptionsRef.current;
    const { layers, floor } = buildingViews(
      building.tool,
      smoothing === 'basement',
      roofsSelected,
    );
    const changed =
      current.floor !== floor ||
      (Object.keys(layers) as (keyof BuildingViews['layers'])[]).some(
        (layer) => current.layers[layer] !== layers[layer],
      );
    if (changed) {
      onRenderOptionsChange({
        ...current,
        layers: { ...current.layers, ...layers },
        floor,
      });
    }
  }, [building.tool, mode, onRenderOptionsChange, roofsSelected, smoothing]);

  const onDismissNotice = useCallback(() => setNotice(null), []);
  const onSetBackground = useCallback((typeName: string) => {
    setMapBackground(typeName).catch(reportError);
  }, []);
  const onSmoothingChange = useCallback((value: MapSmoothing) => {
    setMapSmoothing(value).catch(reportError);
  }, []);
  const onRestrictedScrollChange = useCallback((value: boolean) => {
    setRestrictedScroll(value).catch(reportError);
  }, []);
  const onRoofAllRooms = useCallback(() => {
    roofAllRooms().catch(reportError);
  }, []);
  const onSaveDoor = useCallback((gridNo: number, door: DoorLock | null) => {
    setDoor(gridNo, door).catch(reportError);
  }, []);
  const onCloseDoor = useCallback(() => setDoorRequest(null), []);

  const style = useMemo(
    () =>
      ({
        width: 360,
        flexShrink: 0,
        overflowY: 'auto',
        paddingRight: 8,
      }) as const,
    [],
  );
  const dividerStyle = useMemo(() => ({ margin: '8px 0' }) as const, []);

  // Stays mounted while the map loads to receive its first events, the
  // first state event comes after the map and its palette and light events
  if (smoothing === null) {
    return null;
  }

  return (
    <aside style={style} aria-label="map editor">
      <DoorLockModal
        key={doorRequests}
        request={doorRequest}
        locks={locks}
        onSave={onSaveDoor}
        onClose={onCloseDoor}
      />
      <Divider orientation="left" style={dividerStyle}>
        Layers
      </Divider>
      <MapRenderToolbar
        options={renderOptions}
        onChange={onRenderOptionsChange}
      />
      <Divider orientation="left" style={dividerStyle}>
        {MODE_LABELS[mode]}
      </Divider>
      {renderOptions.floor === 'roof' && GROUND_ONLY_MODES.includes(mode) ? (
        <Typography.Paragraph type="secondary">
          {GROUND_ONLY_HINT}
        </Typography.Paragraph>
      ) : null}
      {mode === 'view' && lightEvent ? (
        <OverviewPanel
          smoothing={smoothing}
          onSmoothingChange={onSmoothingChange}
          restrictedScroll={restrictedScroll}
          onRestrictedScrollChange={onRestrictedScrollChange}
          onRoofAllRooms={onRoofAllRooms}
          lightEvent={lightEvent}
        />
      ) : null}
      {mode === 'buildings' ? (
        <BuildingPanel
          state={building}
          dispatch={buildingDispatch}
          palette={palette}
          smoothing={smoothing}
        />
      ) : null}
      {mode === 'soldiers' ? (
        <SoldiersPanel
          state={soldier}
          dispatch={soldierDispatch}
          palette={soldierPalette}
          soldierEvent={soldierEvent}
          notice={notice}
          onDismissNotice={onDismissNotice}
        />
      ) : null}
      {mode === 'items' ? (
        <ItemsPanel
          state={item}
          dispatch={itemDispatch}
          locks={locks}
          civilianGroups={soldierPalette?.civilianGroups ?? NO_GROUPS}
          itemEvent={itemEvent}
          notice={notice}
          onDismissNotice={onDismissNotice}
        />
      ) : null}
      {mode === 'lights' ? (
        <LightsPanel
          state={light}
          dispatch={lightDispatch}
          lightEvent={lightEvent}
          lighting={renderOptions.lighting}
          notice={notice}
          onDismissNotice={onDismissNotice}
        />
      ) : null}
      {mode === 'traversal' ? (
        <TraversalPanel
          state={traversal}
          dispatch={traversalDispatch}
          traversalEvent={traversalEvent}
        />
      ) : null}
      {mode !== 'view' &&
      mode !== 'buildings' &&
      mode !== 'soldiers' &&
      mode !== 'items' &&
      mode !== 'lights' &&
      mode !== 'traversal' ? (
        <TerrainPanel
          mode={mode}
          state={terrain}
          dispatch={terrainDispatch}
          palette={palette}
          onSetBackground={onSetBackground}
        />
      ) : null}
    </aside>
  );
}
