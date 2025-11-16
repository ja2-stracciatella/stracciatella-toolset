import { useCallback, useEffect, useMemo } from 'react';
import { useImageFile } from '../../hooks/useImage';
import './StrategicMap.css';
import { Button, Flex } from 'antd';
import { MinusOutlined, PlusOutlined } from '@ant-design/icons';

export const HIGHLIGHT_COLORS = [
  '#3cb44b',
  '#4363d8',
  '#f58231',
  '#911eb4',
  '#bfef45',
  '#e6194B',
  '#42d4f4',
  '#ffe119',
  '#f032e6',
] as const;

export const DEFAULT_HIGHLIGHT_COLOR = HIGHLIGHT_COLORS[0];

export type NormalizedSectorId = [string, number];

export function sectorIdStringFromCoords(x: number, y: number): string {
  return `${String.fromCharCode(97 + y)}${(x + 1).toString()}`.toUpperCase();
}

export function coordsFromSectorIdString(
  sectorId: string,
): [number, number] | null {
  const yStr = sectorId[0];
  const xStr = sectorId.substring(1);
  if (!xStr || !yStr) {
    return null;
  }
  const x = parseInt(xStr, 10) - 1;
  const y = yStr.charCodeAt(0) - 'A'.charCodeAt(0);
  if (isNaN(x) || isNaN(y)) {
    return null;
  }
  return [x, y];
}

export interface StrategicMapProps {
  level?: number;
  selectedSectorId?: NormalizedSectorId | null;
  highlightedSectorIds?: { [color: string]: NormalizedSectorId[] };
  onSectorClick?: (sectorId: NormalizedSectorId) => unknown;
  onLevelChange?: (level: number) => unknown;
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
      sectorId: sectorIdStringFromCoords(x, y),
    });
  }
}

interface TileProps extends TilePrefab {
  level: number;
  selected: boolean;
  highlightColor?: string;
  onSectorClick?: (sectorId: NormalizedSectorId) => unknown;
}

function Tile({
  x,
  y,
  sectorId,
  level,
  selected,
  highlightColor,
  onSectorClick,
}: TileProps) {
  const className = useMemo(() => {
    return `strategic-map-tile row-${y} col-${x} ${sectorId} ${
      highlightColor ? 'highlighted' : ''
    } ${selected ? 'selected' : ''}`;
  }, [highlightColor, selected, sectorId, x, y]);
  const content = useMemo(() => {
    return highlightColor ? (
      <div
        className="highlight"
        style={{ backgroundColor: highlightColor, opacity: 0.4 }}
      />
    ) : null;
  }, [highlightColor]);
  const onClick = useCallback(() => {
    if (onSectorClick) {
      return onSectorClick([sectorId, level]);
    }
  }, [level, onSectorClick, sectorId]);
  return (
    <div className={className} onClick={onClick}>
      {content}
    </div>
  );
}

function LevelControls({
  level,
  onLevelChange,
}: {
  level: NonNullable<StrategicMapProps['level']>;
  onLevelChange: NonNullable<StrategicMapProps['onLevelChange']>;
}) {
  const levelUp = useCallback(() => {
    if (level === 0) return;
    onLevelChange(level - 1);
  }, [level, onLevelChange]);
  const levelDown = useCallback(() => {
    if (level === 3) return;
    onLevelChange(level + 1);
  }, [level, onLevelChange]);

  return (
    <Flex gap="small">
      <Button onClick={levelDown} disabled={level === 3}>
        <MinusOutlined />
      </Button>
      <Button onClick={levelUp} disabled={level === 0}>
        <PlusOutlined />
      </Button>
    </Flex>
  );
}

export function StrategicMap({
  selectedSectorId,
  highlightedSectorIds,
  onSectorClick,
  level = 0,
  onLevelChange,
}: StrategicMapProps) {
  const imageFile = useMemo(() => {
    if (level === 0) {
      return 'interface/map_1.sti';
    }
    return `interface/mine_${level}.sti`;
  }, [level]);
  const { data: image, error, refresh } = useImageFile(imageFile);
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
      const highlightColor = Object.entries(highlightedSectorIds ?? {})
        .filter(([, s]) => {
          return !!s.find(
            ([sectorId, l]) => sectorId === p.sectorId && l === level,
          );
        })
        .map(([color]) => color)[0];
      const selected =
        p.sectorId === selectedSectorId?.[0] && level === selectedSectorId?.[1];
      return (
        <Tile
          key={`${p.x}-${p.y}`}
          {...p}
          level={level}
          selected={selected}
          highlightColor={highlightColor}
          onSectorClick={onSectorClick}
        />
      );
    });
  }, [highlightedSectorIds, level, selectedSectorId, onSectorClick]);
  const levelControls = useMemo(() => {
    if (!onLevelChange) {
      return null;
    }
    return <LevelControls level={level} onLevelChange={onLevelChange} />;
  }, [level, onLevelChange]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (error) {
    throw error;
  }

  return (
    <Flex vertical gap="small">
      <div className="strategic-map" style={imageStyle}>
        {tiles}
      </div>
      {levelControls}
    </Flex>
  );
}
