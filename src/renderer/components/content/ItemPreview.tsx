import { ExclamationOutlined } from '@ant-design/icons';
import { Avatar } from 'antd';
import { useMemo } from 'react';
import { useImageFile } from '../../hooks/useImage';

interface SubImage {
  path: string;
  index: number;
}

interface ItemPreviewProps {
  inventoryGraphics: {
    small: SubImage;
    big: SubImage;
  };
}

const crispEdgesStyle = { imageRendering: 'crisp-edges' as const };

export function ItemPreview({ inventoryGraphics: { big } }: ItemPreviewProps) {
  const { data: image, error } = useImageFile(big.path);
  const additionalAvatarProps = useMemo(() => {
    if (error) {
      return { icon: <ExclamationOutlined />, title: error.message };
    }
    if (!image) {
      return {};
    }
    return { src: image };
  }, [error, image]);

  if (error) {
    return <span>Error loading image: {error.message}</span>;
  }

  return (
    <Avatar shape="square" {...additionalAvatarProps} style={crispEdgesStyle} />
  );
}
