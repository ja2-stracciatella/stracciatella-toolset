import { useCallback, useEffect, useMemo } from 'react';
import { useImageFile } from '../../hooks/useImage';

import './StrategicMap.css';

export interface StrategicMapProps {
  selectedSectorId?: string | null;
  highlightedSectorIds: string[];
  onSectorClick?: (sectorId: string) => unknown;
}

interface TilePrefab {
  x: number;
  y: number;
  sectorId: string;
}

const tilePrefabs: Array<TilePrefab> = [];
for (let y = 0; y < 16; y++) {
  for (let x = 0; x < 16; x++) {
    tilePrefabs.push({
      x,
      y,
      sectorId:
        `${String.fromCharCode(97 + y)}${(x + 1).toString()}`.toUpperCase(),
    });
  }
}

interface TileProps extends TilePrefab {
  selected: boolean;
  highlighted: boolean;
  onSectorClick?: (sectorId: string) => unknown;
}

function Tile({
  x,
  y,
  sectorId,
  selected,
  highlighted,
  onSectorClick,
}: TileProps) {
  const className = useMemo(() => {
    return `strategic-map-tile row-${y} col-${x} ${sectorId} ${
      highlighted ? 'highlighted' : ''
    } ${selected ? 'selected' : ''}`;
  }, [highlighted, selected, sectorId, x, y]);
  const content = useMemo(() => {
    return highlighted ? <div className="highlight" /> : null;
  }, [highlighted]);
  const onClick = useCallback(() => {
    if (onSectorClick) {
      return onSectorClick(sectorId);
    }
  }, [onSectorClick, sectorId]);
  return (
    <div className={className} onClick={onClick}>
      {content}
    </div>
  );
}

export function StrategicMap({
  selectedSectorId,
  highlightedSectorIds,
  onSectorClick,
}: StrategicMapProps) {
  const { data: image, error, refresh } = useImageFile('interface/map_1.sti');
  const imageStyle = useMemo(() => {
    if (!image) {
      return {};
    }
    return {
      backgroundImage: `url('${image}')`,
    };
  }, [image]);
  const tiles = useMemo(() => {
    return tilePrefabs.map((p) => {
      const highlighted = highlightedSectorIds
        .map((id) => id.toUpperCase())
        .includes(p.sectorId.toUpperCase());
      return (
        <Tile
          key={`${p.x}-${p.y}`}
          {...p}
          selected={p.sectorId === selectedSectorId}
          highlighted={highlighted}
          onSectorClick={onSectorClick}
        />
      );
    });
  }, [highlightedSectorIds, selectedSectorId, onSectorClick]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (error) {
    throw error;
  }

  return (
    <div className="strategic-map" style={imageStyle}>
      {tiles}
    </div>
  );
}
