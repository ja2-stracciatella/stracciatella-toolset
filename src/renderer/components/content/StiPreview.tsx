import { ExclamationOutlined, QuestionOutlined } from '@ant-design/icons';
import { Flex, Image } from 'antd';
import { useEffect, useMemo } from 'react';
import { useImageFile } from '../../hooks/useImage';

export function StiPreview({
  file,
  subimage,
  size,
  upscale,
  transparentOutline,
}: {
  file: string;
  subimage?: number;
  /** Width and height of the preview, the image is scaled down to fit. 2em by default. */
  size?: string;
  /** Scales small images up to fill the preview, keeping the pixels sharp */
  upscale?: boolean;
  /** Hides the outline pixels of item graphics, like the game does without an outline */
  transparentOutline?: boolean;
}) {
  const { loading, data, error, refresh } = useImageFile(
    file,
    subimage,
    transparentOutline,
  );
  const imageStyle = useMemo(() => {
    if (upscale) {
      return {
        width: '100%',
        height: '100%',
        objectFit: 'contain',
        imageRendering: 'pixelated',
      } as const;
    }
    return size
      ? ({
          maxWidth: size,
          maxHeight: size,
          objectFit: 'contain',
        } as const)
      : undefined;
  }, [size, upscale]);
  const wrapperStyle = useMemo(
    () => (upscale ? { width: '100%', height: '100%' } : undefined),
    [upscale],
  );
  const image = useMemo(() => {
    if (loading) {
      return <QuestionOutlined />;
    }
    if (error) {
      return <ExclamationOutlined title={error.message} />;
    }
    if (!data) {
      return <QuestionOutlined />;
    }
    return (
      <Image
        preview={false}
        src={data}
        style={imageStyle}
        wrapperStyle={wrapperStyle}
      />
    );
  }, [loading, error, data, imageStyle, wrapperStyle]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const style = useMemo(
    () => ({ width: size ?? '2em', height: size ?? '2em' }),
    [size],
  );
  return (
    <Flex style={style} justify="center" align="center">
      {image}
    </Flex>
  );
}
