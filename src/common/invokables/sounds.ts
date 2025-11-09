import z from 'zod';
import { InvokableDefinition } from '.';

type Category = 'sound';

const READ_INPUT_SCHEMA = z.object({
  file: z.string(),
});

const READ_OUTPUT_SCHEMA = z.string();

export type SoundReadInvokable = InvokableDefinition<
  Category,
  'read',
  z.infer<typeof READ_INPUT_SCHEMA>,
  z.infer<typeof READ_OUTPUT_SCHEMA>
>;

export const soundReadInvokableDefinition: SoundReadInvokable = {
  name: 'sound/read',
  inputSchema: READ_INPUT_SCHEMA,
  outputSchema: READ_OUTPUT_SCHEMA,
};
