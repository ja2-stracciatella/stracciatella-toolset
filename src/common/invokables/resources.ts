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

const READ_INPUT_SCHEMA = z.object({
  file: z.string(),
});

// Base64 encoded file contents, null if the file does not exist
const READ_OUTPUT_SCHEMA = z.string().nullable();

export type ResourcesReadInvokable = InvokableDefinition<
  CategoryPlural,
  'read',
  z.infer<typeof READ_INPUT_SCHEMA>,
  z.infer<typeof READ_OUTPUT_SCHEMA>
>;

export const resourcesReadInvokableDefinition: ResourcesReadInvokable = {
  name: 'resources/read',
  inputSchema: READ_INPUT_SCHEMA,
  outputSchema: READ_OUTPUT_SCHEMA,
};

const EXISTS_INPUT_SCHEMA = z.object({
  files: z.array(z.string()),
});

// Whether each of the files exists, in the same order as the input
const EXISTS_OUTPUT_SCHEMA = z.array(z.boolean());

export type ResourcesExistsInvokable = InvokableDefinition<
  CategoryPlural,
  'exists',
  z.infer<typeof EXISTS_INPUT_SCHEMA>,
  z.infer<typeof EXISTS_OUTPUT_SCHEMA>
>;

export const resourcesExistsInvokableDefinition: ResourcesExistsInvokable = {
  name: 'resources/exists',
  inputSchema: EXISTS_INPUT_SCHEMA,
  outputSchema: EXISTS_OUTPUT_SCHEMA,
};

const PERSIST_INPUT_SCHEMA = z.object({
  file: z.string(),
  // Base64 encoded file contents
  data: z.string(),
});

const PERSIST_OUTPUT_SCHEMA = z.null();

export type ResourcesPersistInvokable = InvokableDefinition<
  CategoryPlural,
  'persist',
  z.infer<typeof PERSIST_INPUT_SCHEMA>,
  z.infer<typeof PERSIST_OUTPUT_SCHEMA>
>;

export const resourcesPersistInvokableDefinition: ResourcesPersistInvokable = {
  name: 'resources/persist',
  inputSchema: PERSIST_INPUT_SCHEMA,
  outputSchema: PERSIST_OUTPUT_SCHEMA,
};
