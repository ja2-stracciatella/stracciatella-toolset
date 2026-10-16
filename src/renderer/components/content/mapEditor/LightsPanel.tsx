import { Alert, Flex, Segmented, Select, Typography } from 'antd';
import { Dispatch, useCallback, useMemo } from 'react';
import {
  EditorLightEvent,
  LIGHT_RADII,
  LightCommand,
  lightCommand,
  LightTime,
  LightTool,
  MapLight,
  MapLighting,
  setLight,
} from '../../../lib/mapViewer';
import { BRUSHES, BrushControls } from './BrushControls';
import { EraseControl } from './EraseControl';
import { LightModeAction, LightModeState } from './lightMode';
import type { BrushKind } from './terrainMode';
import { ToolOption, ToolSelect } from './ToolSelect';

const TOOLS: ToolOption<LightTool>[] = [
  {
    label: 'Select',
    value: 'select',
    description: [
      'Selects lights to change their radius and time, moves, copies and removes them.',
      'Clicking a light again selects the next light of the tile. Drag to move it, Ctrl+drag ' +
        '(Cmd+drag on macOS) to copy it, both replace the lights of the target tile. Delete ' +
        'removes the selected light, Esc deselects it.',
    ],
  },
  {
    label: 'Place',
    value: 'place',
    description: [
      'Places new lights with the chosen radius and time, a new light replaces the lights of ' +
        'the tile. Remove removes the lights of the tiles of the brush.',
    ],
  },
];

const NIGHT_HINT =
  'The map is shown at day: only the 24h lights are on. Switch the lighting to night to see ' +
  'the prime time and night lights.';

export const TIME_OPTIONS: { label: string; value: LightTime }[] = [
  { label: 'Prime', value: 'primeTime' },
  { label: 'Night', value: 'night' },
  { label: '24h', value: 'always' },
];

const TIME_LABELS = Object.fromEntries(
  TIME_OPTIONS.map(({ label, value }) => [value, label]),
) as Record<LightTime, string>;

function reportError(e: unknown) {
  console.error('Map editor request failed', e);
}

/** Sends a light command to the viewer */
export function sendLightCommand(command: LightCommand) {
  lightCommand(command).catch(reportError);
}

/** The radii 1 to 8, the ones without a template can't be chosen */
function useRadiusOptions(radii: number[] | undefined) {
  return useMemo(
    () =>
      LIGHT_RADII.map((radius) => ({
        label: `${radius}`,
        value: radius,
        disabled: radii !== undefined && !radii.includes(radius),
      })),
    [radii],
  );
}

interface SelectedLightProps {
  light: MapLight;
  radii: number[] | undefined;
}

/** The position, template, radius and time of the selected light */
function SelectedLight({ light, radii }: SelectedLightProps) {
  const radiusOptions = useRadiusOptions(radii);
  const onRadiusChange = useCallback(
    (radius: number) => {
      setLight({ id: light.id, radius, time: light.time }).catch(reportError);
    },
    [light.id, light.time],
  );
  const onTimeChange = useCallback(
    (time: LightTime) => {
      setLight({ id: light.id, radius: light.radius, time }).catch(reportError);
    },
    [light.id, light.radius],
  );
  return (
    <Flex vertical gap="small" role="region" aria-label="selected light">
      <Typography.Text strong>{light.template}</Typography.Text>
      <Typography.Text type="secondary">
        {`Tile ${light.gridNo} (${light.x}, ${light.y}), ${TIME_LABELS[light.time]}`}
      </Typography.Text>
      <Typography.Text>Radius</Typography.Text>
      <Select<number>
        options={radiusOptions}
        value={light.radius ?? undefined}
        placeholder={light.template}
        onChange={onRadiusChange}
        aria-label="radius of the selected light"
      />
      <Typography.Text>Time</Typography.Text>
      <Segmented<LightTime>
        block
        options={TIME_OPTIONS}
        value={light.time}
        onChange={onTimeChange}
        aria-label="time of the selected light"
      />
    </Flex>
  );
}

export interface LightsPanelProps {
  state: LightModeState;
  dispatch: Dispatch<LightModeAction>;
  /** The selected light and the lights of the map, `null` until the map is loaded */
  lightEvent: EditorLightEvent | null;
  /** The time of day the map is shown at */
  lighting: MapLighting;
  notice: string | null;
  onDismissNotice: () => unknown;
}

/** The lights mode: new lights and the selected light */
export function LightsPanel({
  state,
  dispatch,
  lightEvent,
  lighting,
  notice,
  onDismissNotice,
}: LightsPanelProps) {
  const radiusOptions = useRadiusOptions(lightEvent?.radii);
  const onToolChange = useCallback(
    (tool: LightTool) => dispatch({ type: 'setTool', tool }),
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
  const onRadiusChange = useCallback(
    (radius: number) => dispatch({ type: 'setRadius', radius }),
    [dispatch],
  );
  const onTimeChange = useCallback(
    (time: LightTime) => dispatch({ type: 'setTime', time }),
    [dispatch],
  );

  const selected = lightEvent?.light ?? null;
  let tool;
  if (state.tool === 'select') {
    tool = (
      <Flex vertical gap="small">
        <Typography.Text type="secondary">
          {`${lightEvent?.count ?? 0} lights`}
        </Typography.Text>
        <Typography.Text strong>Selected light</Typography.Text>
        {selected ? (
          <SelectedLight
            key={selected.id}
            light={selected}
            radii={lightEvent?.radii}
          />
        ) : (
          <Typography.Text type="secondary">No light selected</Typography.Text>
        )}
      </Flex>
    );
  } else if (state.erase) {
    tool = (
      <Flex vertical gap="small">
        <BrushControls
          brushes={BRUSHES}
          brush={state.brush}
          size={state.size}
          onBrushChange={onBrushChange}
          onSizeChange={onSizeChange}
        />
      </Flex>
    );
  } else {
    tool = (
      <Flex vertical gap="small">
        <Typography.Text strong>New light</Typography.Text>
        <Typography.Text>Radius</Typography.Text>
        <Select<number>
          options={radiusOptions}
          value={state.radius}
          onChange={onRadiusChange}
          aria-label="radius of new lights"
        />
        <Typography.Text>Time</Typography.Text>
        <Segmented<LightTime>
          block
          options={TIME_OPTIONS}
          value={state.time}
          onChange={onTimeChange}
          aria-label="time of new lights"
        />
      </Flex>
    );
  }
  return (
    <Flex vertical gap="small">
      <ToolSelect<LightTool>
        tools={TOOLS}
        value={state.tool}
        onChange={onToolChange}
        label="light tool"
      />
      {state.tool === 'place' ? (
        <EraseControl erase={state.erase} onChange={onEraseChange} />
      ) : null}
      {tool}
      {notice ? (
        <Alert
          type="warning"
          showIcon
          closable
          message={notice}
          onClose={onDismissNotice}
        />
      ) : null}
      {lighting === 'day' ? (
        <Typography.Text type="secondary">{NIGHT_HINT}</Typography.Text>
      ) : null}
    </Flex>
  );
}
