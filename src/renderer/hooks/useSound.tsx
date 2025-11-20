import { useCallback, useState } from 'react';
import { invoke } from '../lib/invoke';

export function useSound(file: string) {
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<Error | null>(null);
  const [data, setData] = useState<string | null>(null);
  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const i = await invoke('sound/read', {
        file,
      });
      setData(i);
    } catch (e: any) {
      setData(null);
      setError(e);
    } finally {
      setLoading(false);
    }
  }, [file]);

  return { loading, error, data, refresh };
}
