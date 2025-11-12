import { StiPreview } from './StiPreview';

interface SubImage {
  path: string;
  index: number;
}

interface ItemPreviewProps {
  inventoryGraphics?: {
    small?: SubImage;
    big?: SubImage;
  };
}

export function ItemPreview({ inventoryGraphics }: ItemPreviewProps) {
  return <StiPreview file={inventoryGraphics?.big?.path ?? ''} />;
}
