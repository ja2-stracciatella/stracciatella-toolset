import { useCallback, useEffect, useMemo } from 'react';
import { useImageFile } from '../../hooks/useImage';

import './StrategicMap.css';
import { Button, Flex } from 'antd';
import { MinusOutlined, PlusOutlined } from '@ant-design/icons';

export type NormalizedSectorId = [string, number];

export interface StrategicMapProps {
  level?: number;
  selectedSectorId?: NormalizedSectorId | null;
  highlightedSectorIds: NormalizedSectorId[];
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
      sectorId:
        `${String.fromCharCode(97 + y)}${(x + 1).toString()}`.toUpperCase(),
    });
  }
}

interface TileProps extends TilePrefab {
  level: number;
  selected: boolean;
  highlighted: boolean;
  onSectorClick?: (sectorId: NormalizedSectorId) => unknown;
}

function Tile({
  x,
  y,
  sectorId,
  level,
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
      const highlighted: boolean = !!highlightedSectorIds.find(
        ([sectorId, l]) => sectorId === p.sectorId && l === level,
      );
      const selected =
        p.sectorId === selectedSectorId?.[0] && level === selectedSectorId?.[1];
      return (
        <Tile
          key={`${p.x}-${p.y}`}
          {...p}
          level={level}
          selected={selected}
          highlighted={highlighted}
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
