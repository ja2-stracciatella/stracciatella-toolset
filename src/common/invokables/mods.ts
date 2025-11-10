import z from 'zod';
import { InvokableDefinition } from '.';

type Category = 'mod';

type CategoryPlural = `${Category}s`;

export const MOD_SCHEMA = z.object({
  id: z
    .string()
    .min(1)
    .regex(/^[a-z0-9-]+$/)
    .meta({
      title: 'Mod ID',
      description:
        'This is used as the identifier and directory name for your mod. Must contain only lowercase letters, numbers and dashes.',
    }),
  name: z.string().min(1).meta({
    title: 'Mod Name',
    description: 'The name that is displayed to the user for your mod.',
  }),
  description: z.optional(z.string()).meta({
    title: 'Description',
    description: 'A brief description of your mod.',
  }),
  version: z.string().min(1).meta({
    title: 'Version',
    description: 'A version for your mod. E.g. `0.1.0`',
  }),
});

export type Mod = z.infer<typeof MOD_SCHEMA>;

const EDITABLE_MOD_SCHEMA = MOD_SCHEMA.extend({
  path: z.string(),
});

export type EditableMod = z.infer<typeof EDITABLE_MOD_SCHEMA>;

const LIST_AVAILABLE_OUTPUT_SCHEMA = z.array(MOD_SCHEMA);

export type ModListAvailableInvokable = InvokableDefinition<
  CategoryPlural,
  'listAvailable',
  null,
  z.infer<typeof LIST_AVAILABLE_OUTPUT_SCHEMA>
>;

export const modListAvailableInvokableDefinition: ModListAvailableInvokable = {
  name: 'mods/listAvailable',
  inputSchema: z.null(),
  outputSchema: LIST_AVAILABLE_OUTPUT_SCHEMA,
};

const LIST_EDITABLE_OUTPUT_SCHEMA = z.array(EDITABLE_MOD_SCHEMA);

export type ModListEditableInvokable = InvokableDefinition<
  CategoryPlural,
  'listEditable',
  null,
  z.infer<typeof LIST_EDITABLE_OUTPUT_SCHEMA>
>;

export const modListEditableInvokableDefinition: ModListEditableInvokable = {
  name: 'mods/listEditable',
  inputSchema: z.null(),
  outputSchema: LIST_EDITABLE_OUTPUT_SCHEMA,
};

const READ_SELECTED_OUTPUT_SCHEMA = z.nullable(EDITABLE_MOD_SCHEMA);

export type ModReadSelectedInvokable = InvokableDefinition<
  Category,
  'readSelected',
  null,
  z.infer<typeof READ_SELECTED_OUTPUT_SCHEMA>
>;

export const modReadSelectedInvokableDefinition: ModReadSelectedInvokable = {
  name: 'mod/readSelected',
  inputSchema: z.null(),
  outputSchema: READ_SELECTED_OUTPUT_SCHEMA,
};

export type ModUpdateSelectedInvokable = InvokableDefinition<
  Category,
  'updateSelected',
  z.infer<typeof EDITABLE_MOD_SCHEMA>,
  z.infer<typeof EDITABLE_MOD_SCHEMA>
>;

export const modUpdateSelectedInvokableDefinition: ModUpdateSelectedInvokable =
  {
    name: 'mod/updateSelected',
    inputSchema: EDITABLE_MOD_SCHEMA,
    outputSchema: EDITABLE_MOD_SCHEMA,
  };

export type ModCreateInvokable = InvokableDefinition<
  Category,
  'create',
  z.infer<typeof MOD_SCHEMA>,
  z.infer<typeof EDITABLE_MOD_SCHEMA>
>;

export const modCreateInvokableDefinition: ModCreateInvokable = {
  name: 'mod/create',
  inputSchema: MOD_SCHEMA,
  outputSchema: EDITABLE_MOD_SCHEMA,
};
