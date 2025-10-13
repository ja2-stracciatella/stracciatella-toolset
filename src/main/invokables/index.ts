import z from 'zod';
import { showOpenDialog, showOpenDialogParamsSchema } from './dialogs';

export const invokables: Record<string, { fn: Function; params: z.ZodObject }> =
  {
    show_open_dialog: {
      fn: showOpenDialog,
      params: showOpenDialogParamsSchema,
    },
  };
