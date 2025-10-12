import { useCallback, useEffect, useState } from 'react';
import { z } from 'zod';
import { invokeWithSchema } from '../lib/invoke';

const imageFileSchema = z.string();

type ImageFile = z.infer<typeof imageFileSchema>;

export function useImageFile(file: string | null, subimage?: number) {
  const [error, setError] = useState<Error | null>(null);
  const [state, setState] = useState<ImageFile | null>(null);
  const fetch = useCallback(async (file: string, subimage?: number) => {
    const s = await invokeWithSchema(imageFileSchema, 'read_image_file', {
      file,
      subimage: subimage ?? 0,
    });
    setState(s);
  }, []);

  useEffect(() => {
    if (!file) {
      return;
    }
    fetch(file, subimage).catch((e: any) =>
      setError(new Error(`error fetching image: ${e}`)),
    );
  }, [fetch, file, subimage]);

  return { error, data: state };
}
