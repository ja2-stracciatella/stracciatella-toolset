import { useCallback, useMemo } from "react";
import { useImageFile } from "../../hooks/useImage";

import "./StrategicMap.css";

export interface StrategicMapProps {
  highlightedSectorIds: string[];
  onSectorClick?: (sectorId: string) => unknown;
}

interface TilePrefab {
  x: number;
  y: number;
  sectorId: string;
}

let tilePrefabs: Array<TilePrefab> = [];
for (let y = 0; y < 16; y++) {
  for (let x = 0; x < 16; x++) {
    tilePrefabs.push({
      x: x,
      y: y,
      sectorId: String.fromCharCode(97 + y) + (x + 1).toString(),
    });
  }
}

interface TileProps extends TilePrefab {
  highlighted: boolean;
  onSectorClick?: (sectorId: string) => unknown;
}

function Tile({ x, y, sectorId, highlighted, onSectorClick }: TileProps) {
  const className = useMemo(() => {
    return `strategic-map-tile row-${y} col-${x} ${sectorId} ${
      highlighted ? "highlighted" : ""
    }`;
  }, [highlighted, sectorId, x, y]);
  const content = useMemo(() => {
    return highlighted ? <div className="highlight"></div> : null;
  }, [highlighted]);
  const onClick = useCallback(() => {
    if (onSectorClick) {
      return onSectorClick(sectorId);
    }
  }, [onSectorClick, sectorId]);
  return (
    <div
      className={className}
      onClick={onClick}
    >
      {content}
    </div>
  );
}

export function StrategicMap({
  highlightedSectorIds,
  onSectorClick,
}: StrategicMapProps) {
  const { data: image, error } = useImageFile("interface/map_1.sti");
  let imageStyle = useMemo(() => {
    if (!image) {
      return {};
    }
    return {
      backgroundImage: `url('${image}')`,
    };
  }, [image]);
  const tiles = useMemo(() => {
    return tilePrefabs.map((p) => {
      const highlighted = highlightedSectorIds.includes(p.sectorId);
      return <Tile {...p} highlighted={highlighted} onSectorClick={onSectorClick} />;
    });
  }, [highlightedSectorIds, onSectorClick]);

  if (error) {
    throw error;
  }

  return (
    <div className="strategic-map" style={imageStyle}>
      {tiles}
    </div>
  );
}
