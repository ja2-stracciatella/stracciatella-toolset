import { Alert, Form, Modal, Segmented, Select, Spin } from 'antd';
import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  listTilesets,
  MapSmoothing,
  NewMapOptions,
  TilesetList,
} from '../../lib/mapViewer';

/** The tileset of the caves (`CAVES_1`) */
export const CAVES_TILESET = 1;

const TYPE_OPTIONS: { label: string; value: MapSmoothing }[] = [
  { label: 'Outdoor', value: 'normal' },
  { label: 'Basement', value: 'basement' },
  { label: 'Caves', value: 'caves' },
];

export interface NewMapDialogProps {
  /** The map file that the new map is saved to */
  file: string;
  /** Level of the sector, underground levels start as basements */
  level: number;
  /** Whether the sector has a map that saving the new one replaces */
  replacesMap: boolean;
  onCreate: (options: NewMapOptions) => unknown;
  onCancel: () => unknown;
}

/** Asks for the type and the tileset of a new map, mount it to open it */
export function NewMapDialog({
  file,
  level,
  replacesMap,
  onCreate,
  onCancel,
}: NewMapDialogProps) {
  const [smoothing, setSmoothing] = useState<MapSmoothing>(
    level === 0 ? 'normal' : 'basement',
  );
  // `null` until the user picks one, the tileset then follows the map type
  const [chosenTileset, setChosenTileset] = useState<number | null>(null);
  const [tilesets, setTilesets] = useState<TilesetList | null>(null);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;
    listTilesets()
      .then((list) => {
        if (!cancelled) {
          setTilesets(list);
        }
        return list;
      })
      .catch((e: unknown) => {
        if (!cancelled) {
          setError(e instanceof Error ? e : new Error(String(e)));
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const tileset = useMemo(() => {
    if (!tilesets) {
      return null;
    }
    if (chosenTileset !== null) {
      return chosenTileset;
    }
    const hasCaves = tilesets.tilesets.some(({ id }) => id === CAVES_TILESET);
    return smoothing === 'caves' && hasCaves ? CAVES_TILESET : tilesets.default;
  }, [chosenTileset, smoothing, tilesets]);
  const tilesetOptions = useMemo(
    () =>
      tilesets?.tilesets.map(({ id, name }) => ({
        label: `${id} – ${name}`,
        value: id,
      })) ?? [],
    [tilesets],
  );

  const onOk = useCallback(() => {
    if (tileset !== null) {
      onCreate({ tileset, smoothing });
    }
  }, [onCreate, smoothing, tileset]);
  const okButtonProps = useMemo(
    () => ({ disabled: tileset === null }),
    [tileset],
  );

  return (
    <Modal
      title={`New map ${file}`}
      open
      okText="Create"
      okButtonProps={okButtonProps}
      onOk={onOk}
      onCancel={onCancel}
    >
      <Form layout="vertical">
        <Form.Item label="Map type">
          <Segmented<MapSmoothing>
            options={TYPE_OPTIONS}
            value={smoothing}
            onChange={setSmoothing}
            aria-label="map type"
          />
        </Form.Item>
        <Form.Item label="Tileset">
          {error ? (
            <Alert
              type="error"
              message="Could not load the tilesets"
              description={error.message}
            />
          ) : tilesets ? (
            <Select<number>
              showSearch
              optionFilterProp="label"
              options={tilesetOptions}
              value={tileset}
              onChange={setChosenTileset}
              aria-label="tileset"
            />
          ) : (
            <div role="status" aria-label="loading tilesets">
              <Spin />
            </div>
          )}
        </Form.Item>
      </Form>
      {replacesMap ? (
        <Alert
          type="warning"
          showIcon
          message="Saving replaces the existing map of this sector"
        />
      ) : null}
    </Modal>
  );
}
