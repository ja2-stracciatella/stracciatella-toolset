import z from 'zod';
import { showOpenDialog, showOpenDialogParamsSchema } from './dialogs';
import { confirmClose, confirmCloseSchema } from './toolset';

type Invokable<I> = {
  fn: (input: I) => Promise<unknown>;
  params: z.ZodSchema<I>;
};

export const invokables: Record<string, Invokable<any>> = {
  show_open_dialog: {
    fn: showOpenDialog,
    params: showOpenDialogParamsSchema,
  },
  toolset_close_confirm: {
    fn: confirmClose,
    params: confirmCloseSchema,
  },
};
