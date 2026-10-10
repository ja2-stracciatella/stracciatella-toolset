import { Alert, Button, Descriptions, Flex, Typography } from 'antd';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { ResourceEntry } from '../../../common/invokables/resources';
import { useDirEntries } from '../../hooks/useDirEntries';
import {
  NewMapOptions,
  radarMapFile,
  regenerateRadarMap,
} from '../../lib/mapViewer';
import { ResourceType } from '../../lib/resourceType';
import { EditorContent } from '../layout/EditorContent';
import { NewMapDialog } from './NewMapDialog';
import { RadarMapPreview } from './RadarMapPreview';
import {
  coordsFromSectorIdString,
  HIGHLIGHT_COLORS,
  NormalizedSectorId,
  StrategicMap,
} from './StrategicMap';
import { TacticalMapViewer } from './TacticalMapViewer';

const VANILLA_COLOR = HIGHLIGHT_COLORS[0];
const MOD_COLOR = HIGHLIGHT_COLORS[2];

// Surface maps are called like `a9.dat`, underground maps like `a9_b1.dat`
const MAP_FILE_REGEX = /^([a-p])(\d{1,2})(?:_b([1-3]))?\.dat$/i;

/** Path of the map file for a sector, relative to the data root */
export function mapFileForSector([sectorId, level]: NormalizedSectorId) {
  return level === 0
    ? `maps/${sectorId}.dat`
    : `maps/${sectorId}_b${level}.dat`;
}

function sectorIdFromMapFile(entry: ResourceEntry): NormalizedSectorId | null {
  if (entry.type !== 'File') {
    return null;
  }
  const match = MAP_FILE_REGEX.exec(entry.path);
  if (!match?.[1] || !match[2]) {
    return null;
  }
  const sectorId = `${match[1]}${match[2]}`.toUpperCase();
  const coords = coordsFromSectorIdString(sectorId);
  if (!coords || coords[0] < 0 || coords[0] > 15) {
    return null;
  }
  return [sectorId, match[3] ? parseInt(match[3], 10) : 0];
}

function sectorKey([sectorId, level]: NormalizedSectorId) {
  return `${sectorId}-${level}`;
}

function sectorIdsFromEntries(entries: ResourceEntry[] | null) {
  return new Map(
    (entries ?? [])
      .map(sectorIdFromMapFile)
      .filter((s): s is NormalizedSectorId => s !== null)
      .map((s) => [sectorKey(s), s]),
  );
}

function Swatch({ color }: { color: string }) {
  const style = useMemo(
    () => ({
      display: 'inline-block',
      width: '12px',
      height: '12px',
      backgroundColor: color,
      opacity: 0.6,
      marginRight: '6px',
    }),
    [color],
  );
  return <span style={style} />;
}

function Legend() {
  return (
    <Flex gap="middle">
      <span>
        <Swatch color={VANILLA_COLOR} />
        Vanilla map
      </span>
      <span>
        <Swatch color={MOD_COLOR} />
        Map from this mod
      </span>
    </Flex>
  );
}

export function TacticalMaps() {
  const [level, setLevel] = useState(0);
  const [selectedSectorId, setSelectedSectorId] =
    useState<NormalizedSectorId | null>(null);
  const [opened, setOpened] = useState<{
    file: string;
    newMap?: NewMapOptions;
    // Remounts the viewer for every new map, also of the same file
    key: string;
  } | null>(null);
  const [newMapOpen, setNewMapOpen] = useState(false);
  const [newMapCount, setNewMapCount] = useState(0);
  const allMaps = useDirEntries('maps', ResourceType.Any, false);
  const modMaps = useDirEntries('maps', ResourceType.Any, true);
  const refreshAllMaps = allMaps.refresh;
  const refreshModMaps = modMaps.refresh;

  useEffect(() => {
    refreshAllMaps();
    refreshModMaps();
  }, [refreshAllMaps, refreshModMaps]);

  const modSectorIds = useMemo(
    () => sectorIdsFromEntries(modMaps.data),
    [modMaps.data],
  );
  const vanillaSectorIds = useMemo(() => {
    const all = sectorIdsFromEntries(allMaps.data);
    for (const key of modSectorIds.keys()) {
      all.delete(key);
    }
    return all;
  }, [allMaps.data, modSectorIds]);
  const highlightedSectorIds = useMemo(
    () => ({
      [MOD_COLOR]: [...modSectorIds.values()],
      [VANILLA_COLOR]: [...vanillaSectorIds.values()],
    }),
    [modSectorIds, vanillaSectorIds],
  );

  const selectedFile = useMemo(
    () => (selectedSectorId ? mapFileForSector(selectedSectorId) : null),
    [selectedSectorId],
  );
  const selectedSource = useMemo(() => {
    if (!selectedSectorId) {
      return null;
    }
    const key = sectorKey(selectedSectorId);
    if (modSectorIds.has(key)) {
      return 'Mod';
    }
    if (vanillaSectorIds.has(key)) {
      return 'Vanilla';
    }
    return null;
  }, [modSectorIds, selectedSectorId, vanillaSectorIds]);

  const radarFile = useMemo(
    () => (selectedFile && selectedSource ? radarMapFile(selectedFile) : null),
    [selectedFile, selectedSource],
  );
  // Changes after regenerating, so that the mini map is loaded again
  const [radarVersion, setRadarVersion] = useState(0);
  const [regenerating, setRegenerating] = useState(false);
  const [regenerateError, setRegenerateError] = useState<Error | null>(null);
  useEffect(() => {
    setRegenerateError(null);
  }, [selectedFile]);
  const regenerate = useCallback(async () => {
    if (!selectedFile) {
      return;
    }
    setRegenerating(true);
    setRegenerateError(null);
    try {
      await regenerateRadarMap(selectedFile);
      setRadarVersion((version) => version + 1);
    } catch (e) {
      setRegenerateError(e instanceof Error ? e : new Error(String(e)));
    } finally {
      setRegenerating(false);
    }
  }, [selectedFile]);

  const openMap = useCallback(() => {
    setOpened(selectedFile ? { file: selectedFile, key: selectedFile } : null);
  }, [selectedFile]);
  const openNewMapDialog = useCallback(() => setNewMapOpen(true), []);
  const closeNewMapDialog = useCallback(() => setNewMapOpen(false), []);
  const createMap = useCallback(
    (newMap: NewMapOptions) => {
      setNewMapOpen(false);
      if (selectedFile) {
        setOpened({
          file: selectedFile,
          newMap,
          key: `${selectedFile}-new-${newMapCount}`,
        });
        setNewMapCount((count) => count + 1);
      }
    },
    [newMapCount, selectedFile],
  );
  const closeMap = useCallback(() => {
    setOpened(null);
    // The map may have been saved to the mod
    refreshAllMaps();
    refreshModMaps();
  }, [refreshAllMaps, refreshModMaps]);

  const error = allMaps.error ?? modMaps.error;
  const descriptionItems = useMemo(
    () => [
      {
        key: 'sector',
        label: 'Sector',
        children: selectedSectorId?.[0] ?? '-',
      },
      {
        key: 'level',
        label: 'Level',
        children: selectedSectorId?.[1] ?? '-',
      },
      { key: 'file', label: 'File', children: selectedFile ?? '-' },
      {
        key: 'source',
        label: 'Source',
        children: selectedSectorId ? (selectedSource ?? 'No map') : '-',
      },
    ],
    [selectedFile, selectedSectorId, selectedSource],
  );

  if (opened) {
    return (
      <TacticalMapViewer
        key={opened.key}
        file={opened.file}
        newMap={opened.newMap}
        onBack={closeMap}
      />
    );
  }

  return (
    <EditorContent>
      <Typography.Title level={2}>Tactical Maps</Typography.Title>
      <Typography.Paragraph>
        Select a sector and level on the strategic map to view its tactical map.
        Maps from the selected mod take precedence over vanilla maps.
      </Typography.Paragraph>
      {error ? (
        <Alert
          type="error"
          message="Could not list maps"
          description={error.message}
        />
      ) : null}
      <Flex gap="large" wrap>
        <Flex vertical gap="small">
          <StrategicMap
            level={level}
            onLevelChange={setLevel}
            selectedSectorId={selectedSectorId}
            onSectorClick={setSelectedSectorId}
            highlightedSectorIds={highlightedSectorIds}
          />
          <Legend />
        </Flex>
        <Flex vertical gap="middle" style={{ minWidth: '300px' }}>
          <Descriptions
            bordered
            size="small"
            column={1}
            items={descriptionItems}
          />
          <Button
            type="primary"
            disabled={!selectedSource || regenerating}
            onClick={openMap}
          >
            Open map
          </Button>
          <Button
            disabled={!selectedSectorId || regenerating}
            onClick={openNewMapDialog}
          >
            New map
          </Button>
          {newMapOpen && selectedFile && selectedSectorId ? (
            <NewMapDialog
              file={selectedFile}
              level={selectedSectorId[1]}
              replacesMap={selectedSource !== null}
              onCreate={createMap}
              onCancel={closeNewMapDialog}
            />
          ) : null}
          {radarFile ? (
            <Flex vertical gap="small">
              <Typography.Text strong>Mini map</Typography.Text>
              <RadarMapPreview
                key={`${radarFile}-${radarVersion}`}
                file={radarFile}
              />
              {selectedSource === 'Mod' ? (
                <Button loading={regenerating} onClick={regenerate}>
                  Regenerate mini map
                </Button>
              ) : null}
              {regenerateError ? (
                <Alert
                  type="error"
                  message="Could not regenerate the mini map"
                  description={regenerateError.message}
                />
              ) : null}
            </Flex>
          ) : null}
        </Flex>
      </Flex>
    </EditorContent>
  );
}
