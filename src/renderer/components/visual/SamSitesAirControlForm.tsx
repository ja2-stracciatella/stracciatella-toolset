import { clone, isArray } from 'remeda';
import { useCallback, useMemo, useState } from 'react';
import {
  coordsFromSectorIdString,
  HIGHLIGHT_COLORS,
  NormalizedSectorId,
  sectorIdStringFromCoords,
} from '../content/StrategicMap';
import { Badge, Select, Space, Typography } from 'antd';
import { VisualStrategicMapFormWrapper } from './VisualStrategicMapFormWrapper';
import { useFileJsonValue } from '../../hooks/useFileJsonValue';
import { useFileJsonUpdate } from '../../hooks/useFileJsonUpdate';
import { useFileJsonDiskValue } from '../../hooks/useFileJsonDiskValue';
import { JsonRoot } from 'src/common/invokables/jsons';

const SAM_SITES_FILE = 'strategic-map-sam-sites.json';
const EXTRA_FILES = [SAM_SITES_FILE];

interface SamSitesAirControlProps {
  file: string;
}

function useSamSites(): JsonRoot | null {
  return useFileJsonDiskValue(SAM_SITES_FILE);
}

function SamSiteSelect({
  selectedSector,
  onChange,
}: {
  selectedSector: NormalizedSectorId | null;
  onChange: (value: NormalizedSectorId) => unknown;
}) {
  const values = useSamSites();
  const options = useMemo(() => {
    if (!values) return [];
    if (!isArray(values)) return [];

    return values.map((site: any, index) => {
      const color = HIGHLIGHT_COLORS[index % HIGHLIGHT_COLORS.length];
      return {
        label: (
          <Space>
            <Badge color={color} />
            {site.sector}
          </Space>
        ),
        value: site.sector,
        color,
      };
    });
  }, [values]);
  const handleChange = useCallback(
    (value: string) => {
      onChange([value, 0]);
    },
    [onChange],
  );

  return (
    <>
      <Typography.Paragraph>
        Select a SAM site below and click on the map to change sectors.
      </Typography.Paragraph>
      <Select
        options={options}
        value={selectedSector?.[0] ?? null}
        onChange={handleChange}
        style={{ minWidth: '150px' }}
      />
    </>
  );
}

export function SamSitesAirControlForm({ file }: SamSitesAirControlProps) {
  const [selectedSector, setSelectedSector] =
    useState<NormalizedSectorId | null>(null);
  const value = useFileJsonValue(file);
  const samSites = useSamSites();
  const update = useFileJsonUpdate(file);
  const selectedIndex = useMemo(() => {
    if (!samSites || !isArray(samSites)) return null;
    const index = samSites.findIndex((item: any) =>
      selectedSector ? item.sector === selectedSector[0] : false,
    );
    return index === -1 ? null : index;
  }, [selectedSector, samSites]);
  const highlightedSectorIds = useMemo(() => {
    if (!value || !isArray(value)) return;
    const result: { [color: string]: NormalizedSectorId[] } = {};

    for (let y = 0; y < 16; y++) {
      for (let x = 0; x < 16; x++) {
        const idx = (value[y] as number[])[x] ?? 0;
        const color =
          idx !== 0
            ? HIGHLIGHT_COLORS[(idx - 1) % HIGHLIGHT_COLORS.length]
            : undefined;

        if (idx !== -1 && color) {
          result[color] = result[color] || [];
          result[color].push([sectorIdStringFromCoords(x, y), 0]);
        }
      }
    }

    return result;
  }, [value]);
  const handleSectorClick = (sectorId: NormalizedSectorId) => {
    const coordinates = coordsFromSectorIdString(sectorId[0]);
    if (!coordinates || selectedIndex === null) return;

    const newValue = clone(value) as number[][];
    const [x, y] = coordinates;
    if (newValue[y] === undefined || newValue[y][x] === undefined) {
      return;
    }
    newValue[y][x] =
      newValue[y][x] === selectedIndex + 1 ? 0 : selectedIndex + 1;
    update(newValue);
  };

  return (
    <VisualStrategicMapFormWrapper
      file={file}
      extraFiles={EXTRA_FILES}
      strategicMap={{
        selectedSectorId: selectedSector,
        highlightedSectorIds: highlightedSectorIds,
        onSectorClick: handleSectorClick,
      }}
    >
      <SamSiteSelect
        selectedSector={selectedSector}
        onChange={setSelectedSector}
      />
    </VisualStrategicMapFormWrapper>
  );
}
