import { useCallback, useEffect, useState } from 'react';
import { z } from 'zod';
import { invokeWithSchema } from '../lib/invoke';

const imageFileSchema = z.string();

type ImageFile = z.infer<typeof imageFileSchema>;

export function useImageFile(file: string | null, subimage?: number) {
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<Error | null>(null);
  const [data, setData] = useState<ImageFile | null>(null);
  const fetch = useCallback(async (file: string, subimage?: number) => {
    setLoading(true);
    invokeWithSchema(imageFileSchema, 'render_image_file', {
      file,
      subimage: subimage ?? 0,
    })
      .then((i) => setData(i))
      .catch((e) => setError(e))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    if (!file) {
      return;
    }
    fetch(file, subimage);
  }, [fetch, file, subimage]);

  return { loading, error, data };
}
