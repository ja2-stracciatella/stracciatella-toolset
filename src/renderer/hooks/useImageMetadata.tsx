import { useCallback, useState } from 'react';
import { invoke } from '../lib/invoke';
import { ImageMetadata } from '../../common/invokables/images';

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
      const m = await invoke('image/readMetadata', {
        file,
      });
      setData(m);
    } catch (e: any) {
      setError(e);
    } finally {
      setLoading(false);
    }
  }, [file]);

  return { loading, error, data, refresh };
}
