import { ExclamationOutlined } from '@ant-design/icons';
import { Avatar } from 'antd';
import { useMemo } from 'react';
import { useImageFile } from '../../hooks/useImage';
import { StiPreview } from './StiPreview';

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
  return <StiPreview file={big.path} />;
}
