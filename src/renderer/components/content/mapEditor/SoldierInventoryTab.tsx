import {
  Button,
  Checkbox,
  type CheckboxChangeEvent,
  Flex,
  Select,
  Typography,
} from 'antd';
import { useCallback, useMemo } from 'react';
import { DetailedPlacement, ItemStack } from '../../../lib/mapViewer';
import { Loader } from '../../common/Loader';
import {
  ItemInfo,
  newItemStack,
  OBJECT_UNDROPPABLE,
  useItemCatalog,
} from './soldierData';
import { SoldierEditFn } from './soldierEdit';

/** The slots the editor edits, by their index in the inventory (`gbMercSlotTypes`) */
export const INVENTORY_SLOTS: [number, string][] = [
  [0, 'Helmet'],
  [1, 'Vest'],
  [2, 'Legs'],
  [5, 'Hand'],
  [6, 'Second hand'],
  [7, 'Big pocket 1'],
  [8, 'Big pocket 2'],
  [9, 'Big pocket 3'],
  [10, 'Big pocket 4'],
];

interface InventorySlotProps {
  slot: number;
  label: string;
  stack: ItemStack | null;
  options: { value: number; label: string }[];
  items: Map<number, ItemInfo>;
  onChange: (slot: number, stack: ItemStack | null) => unknown;
}

function InventorySlot({
  slot,
  label,
  stack,
  options,
  items,
  onChange,
}: InventorySlotProps) {
  const droppable = stack !== null && (stack.flags & OBJECT_UNDROPPABLE) === 0;
  const slotOptions = useMemo(() => {
    // Items that the item files don't have
    if (stack && !items.has(stack.item)) {
      return [{ value: stack.item, label: `Item ${stack.item}` }, ...options];
    }
    return options;
  }, [items, options, stack]);
  const onItemChange = useCallback(
    (item: number) => {
      // The slot stays droppable or not, new slots are not droppable like in the original
      const undroppable = stack ? !droppable : true;
      onChange(slot, newItemStack(item, items.get(item), undroppable));
    },
    [droppable, items, onChange, slot, stack],
  );
  const onDroppableChange = useCallback(
    (e: CheckboxChangeEvent) => {
      if (stack) {
        const flags = e.target.checked
          ? stack.flags & ~OBJECT_UNDROPPABLE
          : stack.flags | OBJECT_UNDROPPABLE;
        onChange(slot, { ...stack, flags });
      }
    },
    [onChange, slot, stack],
  );
  const onClear = useCallback(() => onChange(slot, null), [onChange, slot]);
  const selectStyle = useMemo(() => ({ flexGrow: 1, minWidth: 0 }), []);
  return (
    <Flex vertical gap={2}>
      <Typography.Text type="secondary">{label}</Typography.Text>
      <Flex gap="small" align="center">
        <Select<number>
          size="small"
          showSearch
          optionFilterProp="label"
          style={selectStyle}
          options={slotOptions}
          value={stack?.item}
          placeholder="Nothing"
          onChange={onItemChange}
          aria-label={label}
        />
        {stack !== null ? (
          <>
            <Checkbox checked={droppable} onChange={onDroppableChange}>
              Droppable
            </Checkbox>
            <Button
              size="small"
              onClick={onClear}
              aria-label={`Clear ${label}`}
            >
              Clear
            </Button>
          </>
        ) : null}
      </Flex>
    </Flex>
  );
}

export interface SoldierInventoryTabProps {
  detailed: DetailedPlacement;
  edit: SoldierEditFn;
}

/**
 * The items of a detailed placement. The game keeps them instead of generating equipment.
 */
export function SoldierInventoryTab({
  detailed,
  edit,
}: SoldierInventoryTabProps) {
  const catalog = useItemCatalog();
  const items = useMemo(
    () => new Map((catalog ?? []).map((item) => [item.index, item])),
    [catalog],
  );
  const options = useMemo(
    () =>
      (catalog ?? []).map((item) => ({ value: item.index, label: item.name })),
    [catalog],
  );
  const onChange = useCallback(
    (slot: number, stack: ItemStack | null) => {
      const inventory = [...detailed.inventory];
      inventory[slot] = stack;
      edit({ detailed: { ...detailed, inventory } });
    },
    [detailed, edit],
  );
  if (!catalog) {
    return <Loader />;
  }
  return (
    <Flex vertical gap="small">
      {INVENTORY_SLOTS.map(([slot, label]) => (
        <InventorySlot
          key={slot}
          slot={slot}
          label={label}
          stack={detailed.inventory[slot] ?? null}
          options={options}
          items={items}
          onChange={onChange}
        />
      ))}
    </Flex>
  );
}
