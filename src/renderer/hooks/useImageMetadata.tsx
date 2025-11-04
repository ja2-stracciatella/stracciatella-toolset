import { useCallback, useState } from 'react';
import { z } from 'zod';
import { invokeWithSchema } from '../lib/invoke';

const subImageSchema = z.object({
  width: z.number(),
  height: z.number(),
  offset_x: z.number(),
  offset_y: z.number(),
});
const imageMetadataSchema = z.object({
  images: z.array(subImageSchema),
});

type ImageMetadata = z.infer<typeof imageMetadataSchema>;

export function useImageMetadata(file: string | null) {
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<Error | null>(null);
  const [data, setData] = useState<ImageMetadata | null>(null);
  const refresh = useCallback(async () => {
    if (!file) {
      return;
    }
    setLoading(true);
    try {
      const m = await invokeWithSchema(
        imageMetadataSchema,
        'read_image_metadata',
        {
          file,
        },
      );
      setData(m);
    } catch (e: any) {
      setError(e);
    } finally {
      setLoading(false);
    }
  }, [file]);

  return { loading, error, data, refresh };
}
