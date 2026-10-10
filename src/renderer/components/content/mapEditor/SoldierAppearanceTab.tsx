import {
  Checkbox,
  type CheckboxChangeEvent,
  Flex,
  Select,
  Typography,
} from 'antd';
import { useCallback, useMemo } from 'react';
import { Soldier, SoldierPaletteEvent } from '../../../lib/mapViewer';
import { SoldierEditFn } from './soldierEdit';

/** The colour sets, by the palette of the detailed placement */
const COLOURS: [
  'head' | 'skin' | 'vest' | 'pants',
  'headPalette' | 'skinPalette' | 'vestPalette' | 'pantsPalette',
  string,
][] = [
  ['head', 'headPalette', 'Hair'],
  ['skin', 'skinPalette', 'Skin'],
  ['vest', 'vestPalette', 'Vest'],
  ['pants', 'pantsPalette', 'Pants'],
];

interface ColourSelectProps {
  palette: (typeof COLOURS)[number][1];
  label: string;
  value: string;
  names: string[];
  onChange: (palette: ColourSelectProps['palette'], name: string) => unknown;
}

function ColourSelect({
  palette,
  label,
  value,
  names,
  onChange,
}: ColourSelectProps) {
  const options = useMemo(() => {
    const options = names.map((name) => ({ value: name, label: name }));
    if (value && !names.includes(value)) {
      options.unshift({ value, label: value });
    }
    return options;
  }, [names, value]);
  const onSelect = useCallback(
    (name: string) => onChange(palette, name),
    [onChange, palette],
  );
  const style = useMemo(() => ({ width: '100%' }) as const, []);
  return (
    <Flex vertical gap={2}>
      <Typography.Text type="secondary">{label}</Typography.Text>
      <Select<string>
        size="small"
        style={style}
        options={options}
        value={value || undefined}
        placeholder="Random"
        onChange={onSelect}
        aria-label={label}
      />
    </Flex>
  );
}

export interface SoldierAppearanceTabProps {
  soldier: Soldier;
  palette: SoldierPaletteEvent | null;
  edit: SoldierEditFn;
}

/**
 * The colours of a detailed placement (`MERC_APPEARANCEMODE`): with fixed colours the soldier
 * keeps them, otherwise the game chooses them
 */
export function SoldierAppearanceTab({
  soldier,
  palette,
  edit,
}: SoldierAppearanceTabProps) {
  const detailed = soldier.detailed;
  const drawn = soldier.drawnPalette;
  const onVisibleChange = useCallback(
    (e: CheckboxChangeEvent) => {
      if (!detailed) {
        return;
      }
      // Fixing the colours starts with the ones the soldier has (`MercsToggleColorModeCallback`)
      const fixed =
        e.target.checked && drawn
          ? {
              headPalette: drawn.head,
              skinPalette: drawn.skin,
              vestPalette: drawn.vest,
              pantsPalette: drawn.pants,
            }
          : {};
      edit({ detailed: { ...detailed, ...fixed, visible: e.target.checked } });
    },
    [detailed, drawn, edit],
  );
  const onColourChange = useCallback(
    (key: (typeof COLOURS)[number][1], name: string) => {
      if (detailed) {
        edit({ detailed: { ...detailed, [key]: name } });
      }
    },
    [detailed, edit],
  );
  if (!detailed) {
    return null;
  }
  return (
    <Flex vertical gap="small">
      <Checkbox checked={detailed.visible} onChange={onVisibleChange}>
        Fixed colours
      </Checkbox>
      {detailed.visible
        ? COLOURS.map(([set, key, label]) => (
            <ColourSelect
              key={key}
              palette={key}
              label={label}
              value={detailed[key]}
              names={palette?.colours[set] ?? []}
              onChange={onColourChange}
            />
          ))
        : null}
      <Typography.Text type="secondary">
        The game keeps the vest and the pants of soldiers without a uniform, the
        hair and the skin are chosen anew.
      </Typography.Text>
    </Flex>
  );
}
