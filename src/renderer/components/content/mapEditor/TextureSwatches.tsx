import { Badge } from 'antd';
import { MouseEvent, useCallback, useMemo } from 'react';
import { PaletteEntry, TextureWeight } from '../../../lib/mapViewer';
import { StiPreview } from '../StiPreview';
import { PALETTE_GRID_STYLE } from './TilePalette';

/** Names of the ground textures, by their type name */
const TEXTURE_LABELS: Record<string, string> = {
  TEXTURE1: 'Texture 1',
  TEXTURE2: 'Texture 2',
  TEXTURE3: 'Texture 3',
  TEXTURE4: 'Texture 4',
  TEXTURE5: 'Texture 5',
  TEXTURE6: 'Texture 6',
  TEXTURE7: 'Texture 7',
  WATER1: 'Water',
  'DEEP WATER': 'Deep water',
};

/** Name of a ground texture of the palette */
export function textureLabel(entry: PaletteEntry): string {
  return TEXTURE_LABELS[entry.typeName] ?? entry.typeName;
}

interface SwatchProps {
  entry: PaletteEntry;
  label: string;
  weight: number;
  selected: boolean;
  onClick: (typeName: string, addWeight: boolean) => unknown;
  onRemoveWeight: (typeName: string) => unknown;
}

function Swatch({
  entry,
  label,
  weight,
  selected,
  onClick,
  onRemoveWeight,
}: SwatchProps) {
  const { typeName } = entry;
  const handleClick = useCallback(
    (e: MouseEvent) => onClick(typeName, e.shiftKey),
    [typeName, onClick],
  );
  const handleContextMenu = useCallback(
    (e: MouseEvent) => {
      e.preventDefault();
      onRemoveWeight(typeName);
    },
    [typeName, onRemoveWeight],
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
  return (
    <Badge count={weight} size="small">
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
}

export interface TextureSwatchesProps {
  /** The first tile of each ground texture */
  entries: PaletteEntry[];
  /** Type name of the selected texture */
  selected: string | null;
  weights: TextureWeight[];
  /** Click selects only this texture, shift-click adds weight */
  onSelect: (typeName: string) => unknown;
  onAddWeight: (typeName: string) => unknown;
  /** Right-click removes weight */
  onRemoveWeight: (typeName: string) => unknown;
}

/**
 * The ground textures. Without weights, the selected texture is painted, otherwise each texture
 * by its weight.
 */
export function TextureSwatches({
  entries,
  selected,
  weights,
  onSelect,
  onAddWeight,
  onRemoveWeight,
}: TextureSwatchesProps) {
  const weighted = useMemo(() => weights.some((w) => w.weight > 0), [weights]);
  const weightOf = useMemo(
    () => new Map(weights.map((w) => [w.typeName, w.weight])),
    [weights],
  );
  const onClick = useCallback(
    (typeName: string, addWeight: boolean) =>
      addWeight ? onAddWeight(typeName) : onSelect(typeName),
    [onAddWeight, onSelect],
  );
  return (
    <div style={PALETTE_GRID_STYLE} role="group" aria-label="ground textures">
      {entries.map((entry) => (
        <Swatch
          key={entry.typeName}
          entry={entry}
          label={textureLabel(entry)}
          weight={weightOf.get(entry.typeName) ?? 0}
          selected={weighted ? false : entry.typeName === selected}
          onClick={onClick}
          onRemoveWeight={onRemoveWeight}
        />
      ))}
    </div>
  );
}
