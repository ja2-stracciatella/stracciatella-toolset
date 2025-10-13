import { dialog } from 'electron';
import z from 'zod';

export const showOpenDialogParamsSchema = z.object({
  title: z.optional(z.string()),
  defaultPath: z.optional(z.string()),
  type: z.union([z.literal('open-file'), z.literal('open-directory')]),
});

export type ShowOpenDialogParams = z.infer<typeof showOpenDialogParamsSchema>;

export interface ShowOpenDialogResult {
  path: string | null;
}

export async function showOpenDialog(
  params: ShowOpenDialogParams,
): Promise<ShowOpenDialogResult> {
  const result = await dialog.showOpenDialog({
    title: params.title,
    defaultPath: params.defaultPath,
    properties: [params.type === 'open-file' ? 'openFile' : 'openDirectory'],
  });
  if (!result.canceled && result.filePaths[0]) {
    return { path: result.filePaths[0] };
  }
  return { path: null };
}
