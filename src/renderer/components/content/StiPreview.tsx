import { ExclamationOutlined } from '@ant-design/icons';
import { Avatar } from 'antd';
import { useMemo } from 'react';
import { useImageFile } from '../../hooks/useImage';

const crispEdgesStyle = { imageRendering: 'crisp-edges' as const };

export function StiPreview({
  file,
  subimage,
}: {
  file: string;
  subimage?: number;
}) {
  const { data: image, error } = useImageFile(file, subimage);
  const additionalAvatarProps = useMemo(() => {
    if (error) {
      return { icon: <ExclamationOutlined />, title: error.message };
    }
    if (!image) {
      return {};
    }
    return { src: image };
  }, [error, image]);

  return (
    <Avatar shape="square" {...additionalAvatarProps} style={crispEdgesStyle} />
  );
}
