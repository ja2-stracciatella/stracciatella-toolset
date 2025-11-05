import z from 'zod';
import { ResourceType } from '../lib/resourceType';
import { invokeWithSchema } from '../lib/invoke';
import { useCallback, useState } from 'react';

const dirEntrySchema = z.union([
  z.object({
    type: z.literal('File'),
    path: z.string(),
  }),
  z.object({
    type: z.literal('Dir'),
    path: z.string(),
  }),
]);

export type DirEntry = z.infer<typeof dirEntrySchema>;

const filterFunctions = {
  [ResourceType.Any]: () => true,
  [ResourceType.Sound]: (e: DirEntry) =>
    e.path.endsWith('.ogg') || e.path.endsWith('.wav'),
  [ResourceType.Graphics]: (e: DirEntry) => e.path.endsWith('.sti'),
};

export function useDirEntries(dir: string, resourceType: ResourceType) {
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<Error | null>(null);
  const [data, setData] = useState<DirEntry[] | null>(null);
  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invokeWithSchema(
        z.array(dirEntrySchema),
        'list_resources',
        {
          dir,
        },
      );
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
  }, [dir, resourceType]);

  return {
    loading,
    error,
    data,
    refresh,
  };
}
