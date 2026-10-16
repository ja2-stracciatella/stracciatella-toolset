import { Button, Flex, Popconfirm, Slider, Typography } from 'antd';
import { Dispatch, useCallback, useMemo } from 'react';
import {
  EditorPaletteEvent,
  TerrainModeName,
  TerrainTool,
} from '../../../lib/mapViewer';
import { BRUSHES, BrushControls } from './BrushControls';
import { EraseControl } from './EraseControl';
import { textureLabel, TextureSwatches } from './TextureSwatches';
import { TilePalette } from './TilePalette';
import { ToolOption, ToolSelect } from './ToolSelect';
import {
  BrushKind,
  effectiveBrush,
  objectGroups,
  TerrainModeAction,
  TerrainModeState,
} from './terrainMode';

const DENSITY_MARKS = { 1: '1%', 100: '100%' };

function formatDensity(density?: number): string {
  return `${density ?? 0}%`;
}

const TOOLS: ToolOption<TerrainTool>[] = [
  {
    label: 'Ground',
    value: 'ground',
    description: [
      'Paints the ground with textures, each tile gets one of the textures at random by their ' +
        'weights. The fill brush paints the connected texture, Remove removes the topmost ' +
        'texture. Set map background replaces the ground of the whole map.',
    ],
  },
  {
    label: 'Roads',
    value: 'roads',
    description: [
      'Places the selected road piece on the clicked tile. Remove removes the road of the tile.',
    ],
  },
  {
    label: 'Cliffs',
    value: 'cliffs',
    description: [
      'Places the selected cliff piece on the clicked tile. Remove removes the cliff of the ' +
        'tile.',
    ],
  },
];

/** Only the ground tool fills */
const TERRAIN_BRUSHES: typeof BRUSHES = [
  ...BRUSHES,
  { label: 'Fill', value: 'fill', title: 'Fill the connected texture' },
];

export interface TerrainPanelProps {
  /** The mode of the editor */
  mode: TerrainModeName;
  state: TerrainModeState;
  dispatch: Dispatch<TerrainModeAction>;
  /** `null` until the map is loaded */
  palette: EditorPaletteEvent | null;
  /** Replaces the ground of the whole map by a texture, by its type name */
  onSetBackground: (typeName: string) => unknown;
}

/** Controls of the editing modes: the brush and the tiles */
export function TerrainPanel({
  mode,
  state,
  dispatch,
  palette,
  onSetBackground,
}: TerrainPanelProps) {
  const onToolChange = useCallback(
    (tool: TerrainTool) => dispatch({ type: 'setTool', tool }),
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
  const onDensityChange = useCallback(
    (density: number) => dispatch({ type: 'setDensity', density }),
    [dispatch],
  );
  const onEraseChange = useCallback(
    (erase: boolean) => dispatch({ type: 'setErase', erase }),
    [dispatch],
  );
  const onSelectTexture = useCallback(
    (typeName: string) => dispatch({ type: 'selectTexture', typeName }),
    [dispatch],
  );
  const onAddTextureWeight = useCallback(
    (typeName: string) => dispatch({ type: 'addTextureWeight', typeName }),
    [dispatch],
  );
  const onRemoveTextureWeight = useCallback(
    (typeName: string) => dispatch({ type: 'removeTextureWeight', typeName }),
    [dispatch],
  );
  const onSelectRoad = useCallback(
    (road: number) => dispatch({ type: 'selectRoad', road }),
    [dispatch],
  );
  const cliffEntries = palette?.cliffs;
  const onSelectCliff = useCallback(
    (index: number) => {
      const entry = cliffEntries?.[index];
      if (entry) {
        dispatch({ type: 'selectCliff', cliff: entry.subIndex });
      }
    },
    [cliffEntries, dispatch],
  );
  const onSelectTile = useCallback(
    (tileId: number) => dispatch({ type: 'selectTile', tileId }),
    [dispatch],
  );
  const onAddTile = useCallback(
    (tileId: number) => dispatch({ type: 'addTile', tileId }),
    [dispatch],
  );
  const onRemoveTile = useCallback(
    (tileId: number) => dispatch({ type: 'removeTile', tileId }),
    [dispatch],
  );
  const onClearTiles = useCallback(
    () => dispatch({ type: 'clearTiles' }),
    [dispatch],
  );

  const selectedTexture = state.ground.selected;
  const onConfirmBackground = useCallback(() => {
    if (selectedTexture !== null) {
      onSetBackground(selectedTexture);
    }
  }, [onSetBackground, selectedTexture]);
  const textureName = useMemo(() => {
    const entry = palette?.ground.find((e) => e.typeName === selectedTexture);
    return entry ? textureLabel(entry) : 'the texture';
  }, [palette, selectedTexture]);

  const ground = mode === 'terrain' && state.tool === 'ground';
  const roads = mode === 'terrain' && state.tool === 'roads';
  const cliffs = mode === 'terrain' && state.tool === 'cliffs';
  const brush = effectiveBrush(state.brush, ground);
  // Roads and cliffs only use the clicked tile
  const brushEnabled = !roads && !cliffs;
  // The ground is always painted on every tile of the brush
  const densityEnabled = mode === 'objects';
  const roadGroups = useMemo(
    () => (palette ? [{ entries: palette.roads }] : []),
    [palette],
  );
  const cliffGroups = useMemo(
    () => (cliffEntries ? [{ entries: cliffEntries }] : []),
    [cliffEntries],
  );
  const selectedCliff = useMemo(
    () => cliffEntries?.findIndex((e) => e.subIndex === state.cliff) ?? -1,
    [cliffEntries, state.cliff],
  );
  const objects = useMemo(
    () => (palette ? objectGroups(palette) : []),
    [palette],
  );

  const tiles = useMemo(() => {
    if (!palette) {
      return (
        <Typography.Text type="secondary">
          The tiles are shown once the map is loaded.
        </Typography.Text>
      );
    }
    if (ground) {
      return (
        <TextureSwatches
          entries={palette.ground}
          selected={state.ground.selected}
          weights={state.ground.weights}
          onSelect={onSelectTexture}
          onAddWeight={onAddTextureWeight}
          onRemoveWeight={onRemoveTextureWeight}
        />
      );
    }
    if (roads) {
      return (
        <TilePalette
          mode="single"
          label="roads"
          groups={roadGroups}
          selected={state.road}
          onSelect={onSelectRoad}
        />
      );
    }
    if (cliffs) {
      return (
        <TilePalette
          mode="single"
          label="cliffs"
          groups={cliffGroups}
          selected={selectedCliff}
          onSelect={onSelectCliff}
        />
      );
    }
    return (
      <TilePalette
        mode="counts"
        label="objects"
        groups={objects}
        selections={state.objects}
        onSelect={onSelectTile}
        onAdd={onAddTile}
        onRemove={onRemoveTile}
        onClear={onClearTiles}
      />
    );
  }, [
    cliffGroups,
    cliffs,
    ground,
    onAddTextureWeight,
    onAddTile,
    onClearTiles,
    onRemoveTextureWeight,
    onRemoveTile,
    onSelectCliff,
    onSelectRoad,
    onSelectTexture,
    onSelectTile,
    objects,
    palette,
    roadGroups,
    roads,
    selectedCliff,
    state.ground,
    state.objects,
    state.road,
  ]);

  return (
    <Flex vertical gap="middle">
      {mode === 'terrain' ? (
        <ToolSelect<TerrainTool>
          tools={TOOLS}
          value={state.tool}
          onChange={onToolChange}
          label="terrain tool"
        />
      ) : null}
      <EraseControl erase={state.erase} onChange={onEraseChange} />

      {brushEnabled ? (
        <Flex vertical gap="small">
          <BrushControls
            brushes={ground ? TERRAIN_BRUSHES : BRUSHES}
            brush={brush}
            size={state.size}
            onBrushChange={onBrushChange}
            onSizeChange={onSizeChange}
          />
          {densityEnabled ? (
            <Flex vertical>
              <span>Density</span>
              <Slider
                min={1}
                max={100}
                marks={DENSITY_MARKS}
                value={state.density}
                onChange={onDensityChange}
                tooltip={{ formatter: formatDensity }}
                aria-label="density"
                ariaLabelForHandle="density"
              />
            </Flex>
          ) : null}
        </Flex>
      ) : null}

      {tiles}

      {ground ? (
        <Popconfirm
          title="Set the map background"
          description={`Replace the ground of the whole map by ${textureName.toLowerCase()}? This can be undone.`}
          onConfirm={onConfirmBackground}
          okText="Replace"
        >
          <Button size="small" disabled={!palette || selectedTexture === null}>
            Set map background…
          </Button>
        </Popconfirm>
      ) : null}
    </Flex>
  );
}
