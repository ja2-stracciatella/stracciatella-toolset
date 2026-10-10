import { Form, Segmented, Slider } from 'antd';
import { useMemo } from 'react';
import { MAX_BRUSH_SIZE, type BrushKind } from './terrainMode';

export const BRUSHES: { label: string; value: BrushKind; title: string }[] = [
  { label: 'Square', value: 'square', title: 'Square around the cursor' },
  { label: 'Circle', value: 'circle', title: 'Circle around the cursor' },
  {
    label: 'Line',
    value: 'line',
    title: 'Line from where the mouse is pressed',
  },
  {
    label: 'Area',
    value: 'area',
    title: 'Rectangle from where the mouse is pressed',
  },
];

/** Brushes that use the size, the line as its width */
const SIZED_BRUSHES: BrushKind[] = ['square', 'circle', 'line'];

const SIZE_MARKS = { 1: '1', [MAX_BRUSH_SIZE]: String(MAX_BRUSH_SIZE) };

export interface BrushControlsProps {
  brushes: typeof BRUSHES;
  brush: BrushKind;
  size: number;
  onBrushChange: (brush: BrushKind) => unknown;
  onSizeChange: (size: number) => unknown;
}

/** The shape and the size of the brush, the size only for brushes that use it */
export function BrushControls({
  brushes,
  brush,
  size,
  onBrushChange,
  onSizeChange,
}: BrushControlsProps) {
  const fields = useMemo(
    () => [
      { name: ['brush'], value: brush },
      { name: ['size'], value: size },
    ],
    [brush, size],
  );

  return (
    <Form name="brush" layout="vertical" fields={fields}>
      <Form.Item label="Brush" name="brush">
        <Segmented<BrushKind>
          options={brushes}
          onChange={onBrushChange}
          aria-label="brush"
        />
      </Form.Item>
      {SIZED_BRUSHES.includes(brush) ? (
        <Form.Item label="Brush Size" name="size">
          <Slider
            min={1}
            max={MAX_BRUSH_SIZE}
            marks={SIZE_MARKS}
            onChange={onSizeChange}
            aria-label="size"
            ariaLabelForHandle="size"
          />
        </Form.Item>
      ) : null}
    </Form>
  );
}
