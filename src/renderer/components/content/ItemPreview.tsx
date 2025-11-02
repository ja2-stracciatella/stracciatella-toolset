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

export function ItemPreview({ inventoryGraphics: { big } }: ItemPreviewProps) {
  return <StiPreview file={big.path} />;
}
