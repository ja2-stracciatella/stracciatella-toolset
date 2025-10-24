import { useCallback, useEffect, useState } from 'react';
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
  const fetch = useCallback(async (file: string) => {
    setLoading(true);
    invokeWithSchema(imageMetadataSchema, 'read_image_metadata', {
      file,
    })
      .then((m) => setData(m))
      .catch((e) => setError(e))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    if (!file) {
      return;
    }
    fetch(file);
  }, [fetch, file]);

  return { loading, error, data };
}
