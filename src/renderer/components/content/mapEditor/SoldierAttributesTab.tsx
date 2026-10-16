import { Flex, InputNumber, Typography } from 'antd';
import { useCallback, useMemo, useState } from 'react';
import { DetailedPlacement, SoldierStats } from '../../../lib/mapViewer';
import { SoldierEditFn } from './soldierEdit';

/** The attributes in the order of the original, with their range */
const STATS: [keyof SoldierStats, string, number, number][] = [
  ['expLevel', 'Exp. level', 1, 9],
  ['life', 'Life', 0, 100],
  ['lifeMax', 'Life max', 1, 100],
  ['marksmanship', 'Marksmanship', 0, 100],
  ['strength', 'Strength', 0, 100],
  ['agility', 'Agility', 0, 100],
  ['dexterity', 'Dexterity', 0, 100],
  ['wisdom', 'Wisdom', 0, 100],
  ['leadership', 'Leadership', 0, 100],
  ['explosive', 'Explosives', 0, 100],
  ['medical', 'Medical', 0, 100],
  ['mechanical', 'Mechanical', 0, 100],
  ['morale', 'Morale', 0, 100],
];

/**
 * The attributes the same way as `ExtractAndUpdateMercAttributes`: empty values are chosen by
 * the game, the others clamped and the life at most the life maximum
 */
export function clampStats(stats: SoldierStats): SoldierStats {
  const clamped = { ...stats };
  for (const [key, , min, max] of STATS) {
    const value = clamped[key];
    if (value !== null) {
      clamped[key] = Math.max(min, Math.min(max, Math.round(value)));
    }
  }
  if (
    clamped.life !== null &&
    clamped.lifeMax !== null &&
    clamped.life > clamped.lifeMax
  ) {
    clamped.life = clamped.lifeMax;
  }
  return clamped;
}

interface StatInputProps {
  stat: keyof SoldierStats;
  label: string;
  min: number;
  max: number;
  value: number | null;
  onCommit: (stat: keyof SoldierStats, value: number | null) => unknown;
}

/** An attribute, sent to the viewer when the input loses the focus or on Enter */
function StatInput({ stat, label, min, max, value, onCommit }: StatInputProps) {
  // What is typed, until it is committed. The value of the viewer otherwise.
  const [draft, setDraft] = useState<{ typed: number | null } | null>(null);
  const shown = draft ? draft.typed : value;
  const onChange = useCallback(
    (typed: number | null) => setDraft({ typed }),
    [],
  );
  const commit = useCallback(() => {
    if (draft) {
      setDraft(null);
      if (draft.typed !== value) {
        onCommit(stat, draft.typed);
      }
    }
  }, [draft, onCommit, stat, value]);
  const style = useMemo(() => ({ width: 72 }) as const, []);
  return (
    <Flex justify="space-between" align="center" gap="small">
      <Typography.Text>{label}</Typography.Text>
      <InputNumber<number>
        size="small"
        style={style}
        min={min}
        max={max}
        precision={0}
        value={shown}
        placeholder="Random"
        onChange={onChange}
        onBlur={commit}
        onPressEnter={commit}
        aria-label={label}
      />
    </Flex>
  );
}

export interface SoldierAttributesTabProps {
  detailed: DetailedPlacement;
  edit: SoldierEditFn;
}

/** The fixed attributes of a detailed placement, empty ones are chosen by the game */
export function SoldierAttributesTab({
  detailed,
  edit,
}: SoldierAttributesTabProps) {
  const onCommit = useCallback(
    (stat: keyof SoldierStats, value: number | null) =>
      edit({
        detailed: {
          ...detailed,
          stats: clampStats({ ...detailed.stats, [stat]: value }),
        },
      }),
    [detailed, edit],
  );
  return (
    <Flex vertical gap={4}>
      {STATS.map(([stat, label, min, max]) => (
        <StatInput
          key={stat}
          stat={stat}
          label={label}
          min={min}
          max={max}
          value={detailed.stats[stat]}
          onCommit={onCommit}
        />
      ))}
      <Typography.Text type="secondary">
        Empty attributes are chosen by the game.
      </Typography.Text>
    </Flex>
  );
}
