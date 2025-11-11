import z from 'zod';
import { InvokableDefinition } from '.';

type Category = 'json';

const ANY_JSON_OBJECT_SCHEMA = z.object().catchall(z.any());

export const JSON_ROOT_SCHEMA = z.union([
  ANY_JSON_OBJECT_SCHEMA,
  z.array(ANY_JSON_OBJECT_SCHEMA),
  z.array(z.array(z.any())),
]);

export type JsonRoot = z.infer<typeof JSON_ROOT_SCHEMA>;

export const JSON_PATCH_SCHEMA = z.array(
  z.union([
    z.object({
      op: z.literal('add'),
      path: z.string(),
      value: z.any(),
    }),
    z.object({
      op: z.literal('remove'),
      path: z.string(),
    }),
    z.object({
      op: z.literal('replace'),
      path: z.string(),
      value: z.any(),
    }),
    z.object({
      op: z.literal('copy'),
      from: z.string(),
      path: z.string(),
    }),
    z.object({
      op: z.literal('move'),
      from: z.string(),
      path: z.string(),
    }),
    z.object({
      op: z.literal('test'),
      path: z.string(),
      value: z.any(),
    }),
  ]),
);

export type JsonPatch = z.infer<typeof JSON_PATCH_SCHEMA>;

const JSON_SCHEMA_SCHEMA = z
  .object({
    title: z.optional(z.string()),
    description: z.optional(z.string()),
    items: z
      .object({
        title: z.optional(z.string()),
        description: z.optional(z.string()),
      })
      .catchall(z.any()),
  })
  .catchall(z.any());

export type JsonSchema = z.infer<typeof JSON_SCHEMA_SCHEMA>;

const READ_INPUT_SCHEMA = z.object({
  file: z.string(),
});

const OUTPUT_SCHEMA = z.object({
  schema: JSON_SCHEMA_SCHEMA,
  vanilla: JSON_ROOT_SCHEMA,
  value: z.nullable(JSON_ROOT_SCHEMA),
  patch: z.nullable(JSON_PATCH_SCHEMA),
});

export type JsonReadInvokable = InvokableDefinition<
  Category,
  'read',
  z.infer<typeof READ_INPUT_SCHEMA>,
  z.infer<typeof OUTPUT_SCHEMA>
>;

export const jsonReadInvokableDefinition: JsonReadInvokable = {
  name: 'json/read',
  inputSchema: READ_INPUT_SCHEMA,
  outputSchema: OUTPUT_SCHEMA,
};

const PERSIST_INPUT_SCHEMA = z.object({
  file: z.string(),
  value: z.nullable(JSON_ROOT_SCHEMA),
  patch: z.nullable(JSON_PATCH_SCHEMA),
});

export type JsonPersistInvokable = InvokableDefinition<
  Category,
  'persist',
  z.infer<typeof PERSIST_INPUT_SCHEMA>,
  z.infer<typeof OUTPUT_SCHEMA>
>;

export const jsonPersistInvokableDefinition: JsonPersistInvokable = {
  name: 'json/persist',
  inputSchema: PERSIST_INPUT_SCHEMA,
  outputSchema: OUTPUT_SCHEMA,
};
