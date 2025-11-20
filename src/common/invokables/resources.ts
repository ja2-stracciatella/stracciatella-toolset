import z from 'zod';
import { InvokableDefinition } from '.';

type CategoryPlural = 'resources';

const RESOURCE_ENTRY_SCHEMA = z.union([
  z.object({
    type: z.literal('File'),
    path: z.string(),
  }),
  z.object({
    type: z.literal('Dir'),
    path: z.string(),
  }),
]);

export type ResourceEntry = z.infer<typeof RESOURCE_ENTRY_SCHEMA>;

const LIST_INPUT_SCHEMA = z.object({
  path: z.string(),
  modOnly: z.optional(z.boolean()),
});

const LIST_OUTPUT_SCHEMA = z.array(RESOURCE_ENTRY_SCHEMA);

export type ResourcesListInvokable = InvokableDefinition<
  CategoryPlural,
  'list',
  z.infer<typeof LIST_INPUT_SCHEMA>,
  z.infer<typeof LIST_OUTPUT_SCHEMA>
>;

export const resourcesListInvokableDefinition: ResourcesListInvokable = {
  name: 'resources/list',
  inputSchema: LIST_INPUT_SCHEMA,
  outputSchema: LIST_OUTPUT_SCHEMA,
};
