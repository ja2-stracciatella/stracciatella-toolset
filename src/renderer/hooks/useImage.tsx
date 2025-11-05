import { useCallback, useState } from 'react';
import { z } from 'zod';
import { invokeWithSchema } from '../lib/invoke';

const imageFileSchema = z.string();

type ImageFile = z.infer<typeof imageFileSchema>;

export function useImageFile(file: string | null, subimage?: number) {
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<Error | null>(null);
  const [data, setData] = useState<ImageFile | null>(null);
  const refresh = useCallback(async () => {
    if (!file) {
      return;
    }
    setLoading(true);
    try {
      const i = await invokeWithSchema(imageFileSchema, 'render_image_file', {
        file,
        subimage: subimage ?? 0,
      });
      setData(i);
    } catch (e: any) {
      setError(e);
    } finally {
      setLoading(false);
    }
  }, [file, subimage]);

  return { loading, error, data, refresh };
}
