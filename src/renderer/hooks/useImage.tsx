import { useCallback, useState } from 'react';
import { invoke } from '../lib/invoke';

export function useImageFile(
  file: string | null,
  subimage?: number,
  transparentOutline?: boolean,
) {
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<Error | null>(null);
  const [data, setData] = useState<string | null>(null);
  const refresh = useCallback(async () => {
    if (!file) {
      return;
    }
    setLoading(true);
    try {
      const i = await invoke('image/render', {
        file,
        subimage: subimage ?? 0,
        ...(transparentOutline ? { transparentOutline } : {}),
      });
      setData(i);
    } catch (e: any) {
      setData(null);
      setError(e);
    } finally {
      setLoading(false);
    }
  }, [file, subimage, transparentOutline]);

  return { loading, error, data, refresh };
}
