import { clone } from 'remeda';
import {
  useFileJson,
  useFileLoading,
  useFileLoadingError,
} from '../../hooks/files';
import { EditorContent } from '../layout/EditorContent';
import { ErrorAlert } from '../common/ErrorAlert';
import { FullSizeLoader } from '../common/FullSizeLoader';
import { TextEditorOr } from '../TextEditor';
import { useMemo, useState } from 'react';
import {
  coordsFromSectorIdString,
  HIGHLIGHT_COLORS,
  NormalizedSectorId,
  sectorIdStringFromCoords,
  StrategicMap,
} from '../content/StrategicMap';
import { Badge, Flex, Select, Space, Typography } from 'antd';
import { JsonFormHeader } from './form/JsonFormHeader';

interface SamSitesAirControlProps {
  file: string;
}

function SamSitesAirControl({
  file,
  samSites,
  value,
  onChange,
}: {
  samSites: any[];
  value: any[][];
  onChange: (value: any[][]) => unknown;
} & SamSitesAirControlProps) {
  const [selectedSite, setSelectedSite] = useState(0);
  const selectedSector = useMemo(() => {
    return samSites[selectedSite]?.sector
      ? ([samSites[selectedSite]?.sector as string, 0] as NormalizedSectorId)
      : undefined;
  }, [samSites, selectedSite]);
  const samSiteOptions = useMemo(
    () =>
      samSites.map((site, index) => {
        const color = HIGHLIGHT_COLORS[index % HIGHLIGHT_COLORS.length];
        return {
          label: (
            <Space>
              <Badge color={color} />
              {site.sector}
            </Space>
          ),
          value: index,
          color,
        };
      }),
    [samSites],
  );
  const highlightedSectorIds = useMemo(() => {
    const result: { [color: string]: NormalizedSectorId[] } = {};

    for (let y = 0; y < 16; y++) {
      for (let x = 0; x < 16; x++) {
        const idx = value[y]?.[x] ?? 0;
        const color = idx !== 0 ? samSiteOptions[idx - 1]?.color : undefined;

        if (idx !== -1 && color) {
          result[color] = result[color] || [];
          result[color].push([sectorIdStringFromCoords(x, y), 0]);
        }
      }
    }

    return result;
  }, [samSiteOptions, value]);
  const handleSectorClick = (sectorId: NormalizedSectorId) => {
    const coordinates = coordsFromSectorIdString(sectorId[0]);
    if (!coordinates) return;
    const newValue = clone(value);
    const [x, y] = coordinates;
    if (newValue[y] === undefined || newValue[y][x] === undefined) {
      return;
    }
    newValue[y][x] = newValue[y][x] === selectedSite + 1 ? 0 : selectedSite + 1;
    onChange(newValue);
  };

  return (
    <Flex gap="middle">
      <StrategicMap
        selectedSectorId={selectedSector}
        highlightedSectorIds={highlightedSectorIds}
        onSectorClick={handleSectorClick}
      />
      <div>
        <JsonFormHeader file={file} />
        <Typography.Paragraph>
          Select a SAM site below and click on the map to change sectors.
        </Typography.Paragraph>
        <Select
          options={samSiteOptions}
          value={selectedSite}
          onChange={setSelectedSite}
          style={{ minWidth: '150px' }}
        />
      </div>
    </Flex>
  );
}

export function SamSitesAirControlForm({ file }: SamSitesAirControlProps) {
  const samSitesFile = 'strategic-map-sam-sites.json';
  const loadingSamSites = useFileLoading(samSitesFile);
  const errorSamSites = useFileLoadingError(samSitesFile);
  const [valueSamSites] = useFileJson(samSitesFile);
  const loading = useFileLoading(file);
  const error = useFileLoadingError(file);
  const [value, update] = useFileJson(file);

  if (
    loading == null ||
    loading ||
    loadingSamSites == null ||
    loadingSamSites
  ) {
    return <FullSizeLoader />;
  }
  if (error || errorSamSites) {
    return <ErrorAlert error={error} />;
  }

  return (
    <EditorContent path={file}>
      <TextEditorOr file={file}>
        <SamSitesAirControl
          file={file}
          samSites={valueSamSites as any[]}
          value={value as any[][]}
          onChange={update}
        />
      </TextEditorOr>
    </EditorContent>
  );
}
