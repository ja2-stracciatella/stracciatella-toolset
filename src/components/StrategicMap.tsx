import { useMemo } from "react";

import "./StrategicMap.css";

interface StrategicMapProps {
  highlightedSectorIds: string[];
  onSectorClick?: (sectorId: string) => unknown;
}

let tilePrefabs: Array<{ x: number; y: number; sectorId: string }> = [];
for (let y = 0; y < 16; y++) {
  for (let x = 0; x < 16; x++) {
    tilePrefabs.push({
      x: x,
      y: y,
      sectorId: String.fromCharCode(97 + y) + (x + 1).toString(),
    });
  }
}

export function StrategicMap({
  highlightedSectorIds,
  onSectorClick,
}: StrategicMapProps) {
  const tiles = useMemo(() => {
    return tilePrefabs.map((p) => {
      const highlighted = highlightedSectorIds.includes(p.sectorId);
      return (
        <div
          className={`strategic-map-tile ${p.sectorId} ${
            highlighted ? "highlighted" : ""
          }`}
          onClick={() => onSectorClick && onSectorClick(p.sectorId)}
        ></div>
      );
    });
  }, [highlightedSectorIds, onSectorClick]);

  return <div className="strategic-map">{tiles}</div>;
}
