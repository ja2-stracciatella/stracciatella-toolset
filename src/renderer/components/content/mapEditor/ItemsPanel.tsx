import { DeleteOutlined } from '@ant-design/icons';
import {
  Alert,
  Badge,
  Button,
  Empty,
  Flex,
  Input,
  InputNumber,
  Select,
  Typography,
} from 'antd';
import {
  ChangeEvent,
  Dispatch,
  memo,
  useCallback,
  useMemo,
  useState,
} from 'react';
import {
  EditorItemEvent,
  ItemChoice,
  ItemCommand,
  itemCommand,
  ItemTool,
  WorldItem,
} from '../../../lib/mapViewer';
import { Loader } from '../../common/Loader';
import { StiPreview } from '../StiPreview';
import { BRUSHES, BrushControls } from './BrushControls';
import { EraseControl } from './EraseControl';
import {
  ACTION_ITEM_NAMES,
  CATEGORY_LABELS,
  editorCategories,
  ITEM_CATEGORIES,
  ItemCategory,
  ItemPaletteEntry,
  placedCount,
} from './itemData';
import { ItemStatsEditor } from './ItemStatsEditor';
import { currentChoice, ItemModeAction, ItemModeState } from './itemMode';
import { LockInfo } from './lockTable';
import { ItemInfo, useItemCatalog } from './soldierData';
import type { BrushKind } from './terrainMode';
import { ToolOption, ToolSelect } from './ToolSelect';
import { PALETTE_GRID_STYLE } from './TilePalette';

const TOOLS: ToolOption<ItemTool>[] = [
  {
    label: 'Select',
    value: 'select',
    description: [
      'Selects a tile with all its items on the floor to edit them, moves, copies and removes ' +
        'them.',
      'Drag to move the items, Ctrl+drag (Cmd+drag on macOS) to copy them. Delete removes all ' +
        'items of the selected tile, Esc deselects it.',
    ],
  },
  {
    label: 'Place',
    value: 'place',
    description: [
      'Places the chosen item on the floor, on the roof floor only on flat roofs. Remove ' +
        'removes the items on the floor of the tiles of the brush.',
    ],
  },
];

function reportError(e: unknown) {
  console.error('Map editor request failed', e);
}

/** Sends an item command to the viewer */
export function sendItemCommand(command: ItemCommand) {
  itemCommand(command).catch(reportError);
}

const CATEGORY_OPTIONS = ITEM_CATEGORIES.map((category) => ({
  value: category,
  label: CATEGORY_LABELS[category],
}));

const ACTION_OPTIONS = ACTION_ITEM_NAMES.map((label, value) => ({
  value,
  label,
}));

function sameChoice(a: ItemChoice | null, b: ItemChoice): boolean {
  return a !== null && JSON.stringify(a) === JSON.stringify(b);
}

interface ItemTileProps {
  entry: ItemPaletteEntry;
  selected: boolean;
  placed: { placed: number; quantity: number } | null;
  onSelect: (entry: ItemPaletteEntry) => unknown;
}

const LABEL_STYLE = {
  display: 'block',
  width: 96,
  fontSize: 11,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
} as const;
const IMAGE_STYLE = {
  display: 'flex',
  justifyContent: 'center',
  alignItems: 'center',
  width: 96,
  height: 48,
} as const;

const ItemTile = memo(function ItemTile({
  entry,
  selected,
  placed,
  onSelect,
}: ItemTileProps) {
  const onClick = useCallback(() => onSelect(entry), [entry, onSelect]);
  const style = useMemo(
    () => ({
      padding: 2,
      cursor: 'pointer',
      border: `2px solid ${selected ? '#1677ff' : 'transparent'}`,
      borderRadius: 4,
      background: 'none',
    }),
    [selected],
  );
  // Like the original: the number of items, and the items in the stacks if they differ
  const count =
    placed === null
      ? 0
      : placed.placed === placed.quantity
        ? placed.placed
        : `${placed.placed}(${placed.quantity})`;
  return (
    <Badge count={count} size="small">
      <button
        type="button"
        style={style}
        onClick={onClick}
        aria-label={entry.label}
        aria-pressed={selected}
        title={entry.label}
      >
        <div style={IMAGE_STYLE}>
          {entry.graphic ? (
            <StiPreview
              file={entry.graphic.file}
              subimage={entry.graphic.subImage}
              size="48px"
              upscale
              transparentOutline
            />
          ) : null}
        </div>
        <span style={LABEL_STYLE}>{entry.label}</span>
      </button>
    </Badge>
  );
});

interface ItemPaletteProps {
  category: ItemCategory;
  entries: ItemPaletteEntry[];
  choice: ItemChoice | null;
  event: EditorItemEvent | null;
  onSelect: (entry: ItemPaletteEntry) => unknown;
}

/** The items of a category, which can be searched by their names */
function ItemPalette({
  category,
  entries,
  choice,
  event,
  onSelect,
}: ItemPaletteProps) {
  const [search, setSearch] = useState('');
  const onSearchChange = useCallback(
    (e: ChangeEvent<HTMLInputElement>) => setSearch(e.target.value),
    [],
  );
  const shown = useMemo(() => {
    const query = search.trim().toLowerCase();
    return query
      ? entries.filter((entry) => entry.label.toLowerCase().includes(query))
      : entries;
  }, [entries, search]);
  const counts = event?.counts ?? null;
  const label = CATEGORY_LABELS[category];
  return (
    <Flex vertical gap="small">
      <Input
        size="small"
        allowClear
        placeholder="Search items"
        value={search}
        onChange={onSearchChange}
        aria-label={`search ${label}`}
      />
      {shown.length === 0 ? (
        <Empty description="No items" image={Empty.PRESENTED_IMAGE_SIMPLE} />
      ) : (
        <div style={PALETTE_GRID_STYLE} role="group" aria-label={label}>
          {shown.map((entry) => (
            <ItemTile
              key={entry.key}
              entry={entry}
              selected={sameChoice(choice, entry.choice)}
              placed={placedCount(entry, counts)}
              onSelect={onSelect}
            />
          ))}
        </div>
      )}
    </Flex>
  );
}

interface SelectedItemProps {
  item: WorldItem;
  catalog: ItemInfo[] | null;
  locks: LockInfo[] | null;
  civilianGroups: string[];
  dispatch: Dispatch<ItemModeAction>;
}

/** An item of the selected tile: its name, a button that removes it and its values */
function SelectedItem({
  item,
  catalog,
  locks,
  civilianGroups,
  dispatch,
}: SelectedItemProps) {
  const name = useMemo(
    () =>
      catalog?.find((info) => info.index === item.stack.item)?.name ??
      `Item ${item.stack.item}`,
    [catalog, item.stack.item],
  );
  const onRemove = useCallback(
    () => sendItemCommand({ kind: 'delete', id: item.id }),
    [item.id],
  );
  return (
    <Flex vertical gap="small" role="region" aria-label={name}>
      <Flex justify="space-between" align="center">
        <Typography.Text strong>{name}</Typography.Text>
        <Button
          size="small"
          type="text"
          danger
          icon={<DeleteOutlined />}
          onClick={onRemove}
          aria-label={`remove ${name}`}
        />
      </Flex>
      <ItemStatsEditor
        item={item}
        catalog={catalog}
        locks={locks}
        civilianGroups={civilianGroups}
        dispatch={dispatch}
      />
    </Flex>
  );
}

export interface ItemsPanelProps {
  state: ItemModeState;
  dispatch: Dispatch<ItemModeAction>;
  /** The locks of the selected mod, `null` while they load */
  locks: LockInfo[] | null;
  /** The names of the civilian groups, for ownership items */
  civilianGroups: string[];
  /** The selected tile, its items and the counts, `null` until the viewer sent them */
  itemEvent: EditorItemEvent | null;
  /** Why the last edit did not happen */
  notice: string | null;
  onDismissNotice: () => unknown;
}

const NUMBER_STYLE = { width: 80 } as const;
const SELECT_STYLE = { width: 220 } as const;

/** The items mode: the items palette, the defaults of new items and the selected tile */
export function ItemsPanel({
  state,
  dispatch,
  locks,
  civilianGroups,
  itemEvent,
  notice,
  onDismissNotice,
}: ItemsPanelProps) {
  const catalog = useItemCatalog();
  const categories = useMemo(
    () => (catalog && locks ? editorCategories(catalog, locks) : null),
    [catalog, locks],
  );
  const choice = currentChoice(state);
  const placed = useMemo(
    () =>
      itemEvent?.counts.items.reduce((sum, count) => sum + count.placed, 0) ??
      0,
    [itemEvent],
  );

  const onToolChange = useCallback(
    (tool: ItemTool) => dispatch({ type: 'setTool', tool }),
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
  const onCategoryChange = useCallback(
    (category: ItemCategory) => dispatch({ type: 'setCategory', category }),
    [dispatch],
  );
  const onSelect = useCallback(
    (entry: ItemPaletteEntry) =>
      dispatch({
        type: 'choose',
        category: state.category,
        choice: entry.choice,
      }),
    [dispatch, state.category],
  );
  const onTrapLevelChange = useCallback(
    (trapLevel: number | null) =>
      dispatch({ type: 'setTrapLevel', trapLevel: trapLevel ?? 0 }),
    [dispatch],
  );
  const onActionChange = useCallback(
    (action: number) => dispatch({ type: 'setAction', action }),
    [dispatch],
  );
  const onCenter = useCallback(() => sendItemCommand({ kind: 'center' }), []);

  const tile = itemEvent?.tile ?? null;
  let tool;
  if (state.tool === 'select') {
    tool = (
      <Flex vertical gap="small">
        <Typography.Text type="secondary">{`${placed} items`}</Typography.Text>
        {tile !== null && itemEvent ? (
          <Flex vertical gap="small" role="region" aria-label="selected tile">
            <Flex justify="space-between" align="center">
              <Typography.Text strong>
                {`${itemEvent.items.length} items on the tile`}
              </Typography.Text>
              <Button size="small" type="link" onClick={onCenter}>
                {`Tile ${tile}`}
              </Button>
            </Flex>
            {itemEvent.items.map((item) => (
              <SelectedItem
                key={item.id}
                item={item}
                catalog={catalog}
                locks={locks}
                civilianGroups={civilianGroups}
                dispatch={dispatch}
              />
            ))}
          </Flex>
        ) : (
          <Typography.Text type="secondary">No tile selected</Typography.Text>
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
        <Select<ItemCategory>
          size="small"
          options={CATEGORY_OPTIONS}
          value={state.category}
          onChange={onCategoryChange}
          virtual={false}
          aria-label="item category"
        />
        {categories ? (
          <ItemPalette
            key={state.category}
            category={state.category}
            entries={categories[state.category]}
            choice={choice}
            event={itemEvent}
            onSelect={onSelect}
          />
        ) : (
          <Loader />
        )}
        {state.category === 'triggers' ? (
          <>
            <Flex gap="small" align="center">
              <Typography.Text>Trap level</Typography.Text>
              <InputNumber
                size="small"
                min={0}
                max={20}
                precision={0}
                value={state.trapLevel}
                onChange={onTrapLevelChange}
                style={NUMBER_STYLE}
                aria-label="trap level of new actions"
              />
            </Flex>
            <Flex gap="small" align="center">
              <Typography.Text>Action</Typography.Text>
              <Select<number>
                size="small"
                style={SELECT_STYLE}
                options={ACTION_OPTIONS}
                value={state.action}
                onChange={onActionChange}
                virtual={false}
                aria-label="action of new actions"
              />
            </Flex>
          </>
        ) : null}
      </Flex>
    );
  }
  return (
    <Flex vertical gap="small">
      <ToolSelect<ItemTool>
        tools={TOOLS}
        value={state.tool}
        onChange={onToolChange}
        label="item tool"
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
    </Flex>
  );
}
