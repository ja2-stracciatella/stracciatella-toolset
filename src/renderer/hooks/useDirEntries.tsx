import { ResourceType } from '../lib/resourceType';
import { invoke } from '../lib/invoke';
import { useCallback, useState } from 'react';
import { ResourceEntry } from '../../common/invokables/resources';

const filterFunctions = {
  [ResourceType.Any]: () => true,
  [ResourceType.Sound]: (e: ResourceEntry) =>
    e.path.endsWith('.ogg') || e.path.endsWith('.wav'),
  [ResourceType.Graphics]: (e: ResourceEntry) => e.path.endsWith('.sti'),
};

export function useDirEntries(
  dir: string,
  resourceType: ResourceType,
  modOnly: boolean,
) {
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<Error | null>(null);
  const [data, setData] = useState<ResourceEntry[] | null>(null);
  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invoke('resources/list', {
        path: dir,
        modOnly,
      });
      const filterFn = filterFunctions[resourceType];
      const entries = result.filter((e) => {
        // Always show directories
        if (e.type === 'Dir') {
          return true;
        }
        // Disallow referencing slfs
        if (e.path.endsWith('.slf')) {
          return false;
        }
        return filterFn(e);
      });

      entries.sort((a, b) => {
        if (a.type === 'File' && b.type === 'Dir') {
          return 1;
        }
        if (a.type === 'Dir' && b.type === 'File') {
          return -1;
        }
        return a.path.localeCompare(b.path, 'en', {
          ignorePunctuation: true,
        });
      });

      setData(entries);
    } catch (e: any) {
      setError(e);
    } finally {
      setLoading(false);
    }
  }, [dir, modOnly, resourceType]);

  return {
    loading,
    error,
    data,
    refresh,
  };
}
