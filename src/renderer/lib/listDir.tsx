import { z } from 'zod';
import { invokeWithSchema } from './invoke';

export enum ResourceType {
  Any,
  Sound,
}

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

export async function listDir(
  resourceType: ResourceType,
  dir: string,
): Promise<Array<DirEntry>> {
  let entries = await invokeWithSchema(
    z.array(dirEntrySchema),
    'list_resources',
    {
      dir,
    },
  );

  // Disallow referencing slfs
  entries = entries.filter((e) => e.type === 'Dir' || !e.path.endsWith('.slf'));
  if (resourceType === ResourceType.Sound) {
    entries = entries.filter(
      (e) =>
        e.type === 'Dir' || e.path.endsWith('.ogg') || e.path.endsWith('.wav'),
    );
  }

  entries.sort((a, b) => {
    if (a.type === 'File' && b.type === 'Dir') {
      return 1;
    }
    if (a.type === 'Dir' && b.type === 'File') {
      return -1;
    }
    return a.path.localeCompare(b.path, 'en', { ignorePunctuation: true });
  });

  return entries;
}
