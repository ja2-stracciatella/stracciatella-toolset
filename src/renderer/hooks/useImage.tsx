import { useCallback, useEffect, useState } from 'react';
import { z } from 'zod';
import { invokeWithSchema } from '../lib/invoke';

const imageFileSchema = z.string();

type ImageFile = z.infer<typeof imageFileSchema>;

export function useImageFile(file: string | null) {
  const [error, setError] = useState<Error | null>(null);
  const [state, setState] = useState<ImageFile | null>(null);
  const fetch = useCallback(async (file: string) => {
    const state = await invokeWithSchema(imageFileSchema, 'read_image_file', {
      file,
    });
    setState(state);
  }, []);

  useEffect(() => {
    if (!file) {
      return;
    }
    fetch(file).catch((e: any) =>
      setError(new Error(`error fetching image: ${e}`))
    );
  }, [fetch, file]);

  return { error, data: state };
}
