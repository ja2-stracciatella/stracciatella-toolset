import { Button, ColorPicker, Form, Segmented, Slider } from 'antd';
import type { ColorPickerProps } from 'antd';
import { useCallback, useMemo, useState } from 'react';
import { EditorLightEvent, MapSmoothing } from '../../../lib/mapViewer';
import { sendLightCommand } from './LightsPanel';

const SMOOTHING_OPTIONS: { label: string; value: MapSmoothing }[] = [
  { label: 'Normal', value: 'normal' },
  { label: 'Basement', value: 'basement' },
  { label: 'Caves', value: 'caves' },
];

/** `ubRestrictedScrollID` 0 or 1, the game supports no other scroll restrictions */
const SCROLL_OPTIONS: { label: string; value: boolean }[] = [
  { label: 'Whole map', value: false },
  { label: 'Basement area', value: true },
];

/** The light levels of the original editor */
const MIN_LIGHT_LEVEL = 1;
const MAX_LIGHT_LEVEL = 15;
const LEVEL_MARKS = {
  [MIN_LIGHT_LEVEL]: String(MIN_LIGHT_LEVEL),
  [MAX_LIGHT_LEVEL]: String(MAX_LIGHT_LEVEL),
};

/** The level the slider was moved to, and the light event of the map when it was */
interface DraggedLevel {
  level: number;
  from: EditorLightEvent;
}

export interface OverviewPanelProps {
  smoothing: MapSmoothing;
  onSmoothingChange: (smoothing: MapSmoothing) => unknown;
  /** Whether the game only scrolls over the middle of the map */
  restrictedScroll: boolean;
  onRestrictedScrollChange: (restrictedScroll: boolean) => unknown;
  onRoofAllRooms: () => unknown;
  /** The lighting of the map */
  lightEvent: EditorLightEvent;
}

/** The overview mode: the settings of the whole map */
export function OverviewPanel({
  smoothing,
  onSmoothingChange,
  restrictedScroll,
  onRestrictedScrollChange,
  onRoofAllRooms,
  lightEvent,
}: OverviewPanelProps) {
  const { color } = lightEvent;
  const level = lightEvent.ambient?.level ?? null;
  const colorValue = useMemo(
    () => `rgb(${color[0]}, ${color[1]}, ${color[2]})`,
    [color],
  );
  const onColorChange = useCallback<
    NonNullable<ColorPickerProps['onChangeComplete']>
  >(
    (value) => {
      const { r, g, b } = value.toRgb();
      sendLightCommand({
        kind: 'settings',
        color: [Math.round(r), Math.round(g), Math.round(b)],
        ambientLevel: level,
      });
    },
    [level],
  );
  // Every new level rebuilds the scene and can be undone, so it is only sent once the slider is
  // released. It shows the moved level until the map reports its lighting again.
  const [dragged, setDragged] = useState<DraggedLevel | null>(null);
  const shownLevel =
    dragged !== null && dragged.from === lightEvent ? dragged.level : level;
  const onLevelChange = useCallback(
    (value: number) => setDragged({ level: value, from: lightEvent }),
    [lightEvent],
  );
  const onLevelChangeComplete = useCallback(
    (value: number) => {
      setDragged({ level: value, from: lightEvent });
      if (value !== level) {
        sendLightCommand({ kind: 'settings', color, ambientLevel: value });
      }
    },
    [color, level, lightEvent],
  );

  const fields = useMemo(
    () => [
      { name: ['map-type'], value: smoothing },
      { name: ['scroll-limit'], value: restrictedScroll },
      { name: ['light-color'], value: colorValue },
      { name: ['underground-light-level'], value: shownLevel },
    ],
    [smoothing, restrictedScroll, colorValue, shownLevel],
  );

  return (
    <Form name="overview" layout="vertical" fields={fields}>
      <Form.Item label="Map Type" name="map-type">
        <Segmented<MapSmoothing>
          block
          options={SMOOTHING_OPTIONS}
          onChange={onSmoothingChange}
        />
      </Form.Item>
      <Form.Item label="Scroll Limit" name="scroll-limit">
        <Segmented<boolean>
          block
          options={SCROLL_OPTIONS}
          onChange={onRestrictedScrollChange}
        />
      </Form.Item>
      <Form.Item label="Light color" name="light-color">
        <ColorPicker
          disabledAlpha
          showText
          format="rgb"
          onChangeComplete={onColorChange}
        />
      </Form.Item>
      {level !== null ? (
        <Form.Item
          label="Underground Light Level"
          name="underground-light-level"
        >
          <Slider
            min={MIN_LIGHT_LEVEL}
            max={MAX_LIGHT_LEVEL}
            marks={LEVEL_MARKS}
            onChange={onLevelChange}
            onChangeComplete={onLevelChangeComplete}
          />
        </Form.Item>
      ) : null}
      {smoothing === 'basement' ? (
        <Button onClick={onRoofAllRooms}>Roof all rooms</Button>
      ) : null}
    </Form>
  );
}
