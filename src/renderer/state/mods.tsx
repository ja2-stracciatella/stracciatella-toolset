import { createAsyncThunk, createSlice } from '@reduxjs/toolkit';
import { toJSONSchema, z } from 'zod';
import { invokeWithSchema } from '../lib/invoke';
import {
  Loadable,
  makeLoadable,
  Persistable,
  makePersistable,
  buildLoadableMapping,
  buildPersistableMapping,
} from './types';

const modSchema = z.object({
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

export const MOD_JSON_SCHEMA = toJSONSchema(modSchema);

export type Mod = z.infer<typeof modSchema>;

const editableModSchema = modSchema.extend({
  path: z.string(),
});

export type EditableMod = z.infer<typeof editableModSchema>;

interface Mods {
  editable: Array<EditableMod>;
  available: Array<Mod>;
}

interface ModsState {
  mods: Loadable<Mods>;
  selected: Persistable<EditableMod>;
}

const initialState: ModsState = {
  mods: makeLoadable<Mods>(null),
  selected: makePersistable<EditableMod>(null),
};

export const loadMods = createAsyncThunk('mods/get', async () => {
  const MODS_SCHEMA = z.array(modSchema);
  const EDITABLE_MODS_SCHEMA = z.array(editableModSchema);
  const [available, editable] = await Promise.all([
    invokeWithSchema(MODS_SCHEMA, 'read_available_mods'),
    invokeWithSchema(EDITABLE_MODS_SCHEMA, 'read_editable_mods'),
  ]);
  return {
    available,
    editable,
  };
});

export const readSelectedMod = createAsyncThunk(
  'mods/read_selected',
  async () => {
    return await invokeWithSchema(
      z.nullable(editableModSchema),
      'read_selected_mod',
    );
  },
);

export const updateSelectedMod = createAsyncThunk(
  'mods/update_selected',
  async (mod: EditableMod) => {
    return await invokeWithSchema(editableModSchema, 'update_selected_mod', {
      mod_id: mod.id,
    });
  },
);

export const createNewMod = createAsyncThunk(
  'mods/create_new',
  async (newMod: Mod) => {
    return await invokeWithSchema(editableModSchema, 'create_new_mod', newMod);
  },
);

const modsSlice = createSlice({
  name: 'mods',
  initialState,
  reducers: {},
  extraReducers: (builder) => {
    buildLoadableMapping(builder, (state) => state.mods, loadMods);
    buildLoadableMapping(builder, (state) => state.selected, readSelectedMod);
    buildPersistableMapping(
      builder,
      (state) => state.selected,
      updateSelectedMod,
    );
    buildPersistableMapping(builder, (state) => state.selected, createNewMod);
  },
});

export const mods = modsSlice.reducer;
