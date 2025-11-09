import z from 'zod';
import { InvokableDefinition } from '.';

type Category = 'dialog';

const SHOW_OPEN_DIALOG_INPUT_SCHEMA = z.object({
  title: z.optional(z.string()),
  defaultPath: z.optional(z.string()),
  type: z.union([z.literal('open-file'), z.literal('open-directory')]),
});

const SHOW_OPEN_DIALOG_OUTPUT_SCHEMA = z.object({
  path: z.nullable(z.string()),
});

export type DialogShowOpenDialogInvokable = InvokableDefinition<
  Category,
  'showOpenDialog',
  z.infer<typeof SHOW_OPEN_DIALOG_INPUT_SCHEMA>,
  z.infer<typeof SHOW_OPEN_DIALOG_OUTPUT_SCHEMA>
>;

export const dialogShowOpenDialogInvokableDefinition: DialogShowOpenDialogInvokable =
  {
    name: 'dialog/showOpenDialog',
    inputSchema: SHOW_OPEN_DIALOG_INPUT_SCHEMA,
    outputSchema: SHOW_OPEN_DIALOG_OUTPUT_SCHEMA,
  };
