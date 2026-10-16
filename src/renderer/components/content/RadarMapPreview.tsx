import { Flex, Typography } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { useImageFile } from '../../hooks/useImage';
import { invoke } from '../../lib/invoke';

// Radar maps are 88×44 pixels, shown at three times the size
const WIDTH = 264;
const HEIGHT = 132;

const BOX_STYLE = {
  width: `${WIDTH}px`,
  height: `${HEIGHT}px`,
  border: '1px solid rgba(128, 128, 128, 0.3)',
} as const;

function RadarMapImage({ file }: { file: string }) {
  const { loading, error, data, refresh } = useImageFile(file);
  useEffect(() => {
    refresh();
  }, [refresh]);
  const style = useMemo(
    () => ({ ...BOX_STYLE, imageRendering: 'pixelated' }) as const,
    [],
  );

  if (error) {
    return (
      <Flex style={BOX_STYLE} justify="center" align="center">
        <Typography.Text type="danger">{error.message}</Typography.Text>
      </Flex>
    );
  }
  if (loading || !data) {
    return <div style={BOX_STYLE} />;
  }
  return (
    <img
      alt="Mini map"
      src={data}
      width={WIDTH}
      height={HEIGHT}
      style={style}
    />
  );
}

/**
 * The radar map (`radarmaps/<map>.sti`) of a map, the mini map of the strategic screen of the
 * game, from the selected mod or vanilla. Give it a new `key` to load it again.
 */
export function RadarMapPreview({ file }: { file: string }) {
  const [exists, setExists] = useState<boolean | null>(null);
  const [error, setError] = useState<Error | null>(null);

  // Checked once, the preview is mounted again (with a new `key`) to check again
  useEffect(() => {
    let current = true;
    (async () => {
      try {
        const [found] = await invoke('resources/exists', { files: [file] });
        if (current) {
          setExists(found ?? false);
        }
      } catch (e) {
        if (current) {
          setError(e instanceof Error ? e : new Error(String(e)));
        }
      }
    })();
    return () => {
      current = false;
    };
  }, [file]);

  if (error) {
    return (
      <Flex style={BOX_STYLE} justify="center" align="center">
        <Typography.Text type="danger">{error.message}</Typography.Text>
      </Flex>
    );
  }
  if (exists === null) {
    return <div style={BOX_STYLE} />;
  }
  if (!exists) {
    return (
      <Flex style={BOX_STYLE} justify="center" align="center">
        <Typography.Text type="secondary">No mini map</Typography.Text>
      </Flex>
    );
  }
  return <RadarMapImage file={file} />;
}
