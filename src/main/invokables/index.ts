import z from 'zod';
import { showOpenDialog, showOpenDialogParamsSchema } from './dialogs';

type Invokable<I> = {
  fn: (input: I) => Promise<unknown>;
  params: z.ZodSchema<I>;
};

export const invokables: Record<string, Invokable<any>> = {
  show_open_dialog: {
    fn: showOpenDialog,
    params: showOpenDialogParamsSchema,
  },
};
