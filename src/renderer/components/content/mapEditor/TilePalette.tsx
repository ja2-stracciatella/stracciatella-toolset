import { Badge, Button, Empty, Flex, Input, Typography } from 'antd';
import {
  ChangeEvent,
  memo,
  MouseEvent,
  useCallback,
  useMemo,
  useState,
} from 'react';
import { PaletteEntry, TileSelection } from '../../../lib/mapViewer';
import { StiPreview } from '../StiPreview';

/** Three columns of palette entries, aligned to both edges with even space between them */
export const PALETTE_GRID_STYLE = {
  display: 'grid',
  gridTemplateColumns: 'repeat(3, auto)',
  justifyContent: 'space-between',
  gap: 8,
} as const;

/**
 * The name of a tile, which the search matches. The image file names from JA2SET.DAT describe
 * the tiles better than the type names, so they come first like in the original editor.
 */
function tileLabel(entry: PaletteEntry): string {
  const file = entry.file.slice(entry.file.lastIndexOf('/') + 1);
  const name = file.replace(/\.sti$/i, '');
  return `${name} ${entry.subIndex} (${entry.typeName})`;
}

interface PaletteTileProps {
  entry: PaletteEntry;
  index: number;
  count: number;
  selected: boolean;
  onClick: (entry: PaletteEntry, index: number, add: boolean) => unknown;
  onRemove: (entry: PaletteEntry, index: number) => unknown;
}

const PaletteTile = memo(function PaletteTile({
  entry,
  index,
  count,
  selected,
  onClick,
  onRemove,
}: PaletteTileProps) {
  const handleClick = useCallback(
    (e: MouseEvent) => onClick(entry, index, e.shiftKey),
    [entry, index, onClick],
  );
  const handleContextMenu = useCallback(
    (e: MouseEvent) => {
      e.preventDefault();
      onRemove(entry, index);
    },
    [entry, index, onRemove],
  );
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
  const label = tileLabel(entry);
  return (
    <Badge count={count} size="small">
      <button
        type="button"
        style={style}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
        aria-label={label}
        aria-pressed={selected}
        title={label}
      >
        <StiPreview
          file={entry.file}
          subimage={entry.subImage}
          size="96px"
          upscale
        />
      </button>
    </Badge>
  );
});

/** Tiles of a palette, shown under their label if there is one */
export interface TileGroup {
  label?: string;
  entries: PaletteEntry[];
}

export type TilePaletteProps =
  | {
      /** Tiles are picked at random by their counts */
      mode: 'counts';
      label: string;
      groups: TileGroup[];
      selections: TileSelection[];
      /** Click selects only this tile */
      onSelect: (tileId: number) => unknown;
      /** Shift-click adds 1 to the count of a tile */
      onAdd: (tileId: number) => unknown;
      /** Right-click removes 1 */
      onRemove: (tileId: number) => unknown;
      onClear: () => unknown;
    }
  | {
      /** One tile is chosen, by its index in all groups */
      mode: 'single';
      label: string;
      groups: TileGroup[];
      selected: number;
      onSelect: (index: number) => unknown;
    };

const noop = () => {};

/** The tiles that the terrain modes place, which can be searched by their names */
export function TilePalette(props: TilePaletteProps) {
  const { mode, label, groups } = props;
  const [search, setSearch] = useState('');
  const onSearchChange = useCallback(
    (e: ChangeEvent<HTMLInputElement>) => setSearch(e.target.value),
    [],
  );
  const total = useMemo(
    () => groups.reduce((sum, group) => sum + group.entries.length, 0),
    [groups],
  );
  // The entries that match the search, with their index in all groups
  const shownGroups = useMemo(() => {
    const query = search.trim().toLowerCase();
    let index = 0;
    return groups
      .map((group) => ({
        label: group.label,
        entries: group.entries
          .map((entry) => ({ entry, index: index++ }))
          .filter(
            ({ entry }) =>
              !query || tileLabel(entry).toLowerCase().includes(query),
          ),
      }))
      .filter((group) => group.entries.length > 0);
  }, [groups, search]);
  const counts = useMemo(() => {
    const counts = new Map<number, number>();
    if (props.mode === 'counts') {
      for (const selection of props.selections) {
        counts.set(selection.tileId, selection.count);
      }
    }
    return counts;
  }, [props]);
  const selectedIndex = props.mode === 'single' ? props.selected : -1;
  const onAdd = props.mode === 'counts' ? props.onAdd : undefined;
  const onRemove = props.mode === 'counts' ? props.onRemove : undefined;
  const onSelectIndex = props.mode === 'single' ? props.onSelect : undefined;
  const onSelectTile = props.mode === 'counts' ? props.onSelect : undefined;
  const onClick = useCallback(
    (entry: PaletteEntry, index: number, add: boolean) => {
      if (onSelectIndex) {
        return onSelectIndex(index);
      }
      return add ? onAdd?.(entry.tileId) : onSelectTile?.(entry.tileId);
    },
    [onAdd, onSelectIndex, onSelectTile],
  );
  const onRemoveTile = useCallback(
    (entry: PaletteEntry) => onRemove?.(entry.tileId),
    [onRemove],
  );

  if (total === 0) {
    return (
      <Empty description="No tiles" image={Empty.PRESENTED_IMAGE_SIMPLE} />
    );
  }
  return (
    <Flex vertical gap="small">
      {props.mode === 'counts' ? (
        <Flex justify="space-between" align="center">
          <span>
            {counts.size} of {total} selected
          </span>
          <Button
            size="small"
            onClick={props.onClear}
            disabled={counts.size === 0}
          >
            Clear
          </Button>
        </Flex>
      ) : null}
      <Input
        size="small"
        allowClear
        placeholder="Search tiles"
        value={search}
        onChange={onSearchChange}
        aria-label={`search ${label}`}
      />
      {shownGroups.length === 0 ? (
        <Empty
          description="No matching tiles"
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      ) : null}
      {shownGroups.map((group) => (
        <Flex vertical gap="small" key={group.label ?? label}>
          {group.label ? (
            <Typography.Text>{group.label}</Typography.Text>
          ) : null}
          <div
            style={PALETTE_GRID_STYLE}
            role="group"
            aria-label={group.label ?? label}
          >
            {group.entries.map(({ entry, index }) => (
              <PaletteTile
                key={entry.tileId}
                entry={entry}
                index={index}
                count={counts.get(entry.tileId) ?? 0}
                selected={
                  mode === 'single'
                    ? index === selectedIndex
                    : counts.has(entry.tileId)
                }
                onClick={onClick}
                onRemove={mode === 'counts' ? onRemoveTile : noop}
              />
            ))}
          </div>
        </Flex>
      ))}
    </Flex>
  );
}
