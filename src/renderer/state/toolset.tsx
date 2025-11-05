import { createAsyncThunk, createSlice } from '@reduxjs/toolkit';
import { toJSONSchema, z } from 'zod';
import { invokeWithSchema } from '../lib/invoke';
import {
  buildLoadableMapping,
  buildPersistableMapping,
  makePersistable,
  Persistable,
} from './types';

const partialToolsetConfigSchema = z.object({
  stracciatellaHome: z.nullable(z.string()),
  vanillaGameDir: z.nullable(z.string()),
  stracciatellaInstallDir: z.nullable(z.string()),
  lastSelectedMod: z.nullable(z.string()),
});

export type PartialToolsetConfig = z.infer<typeof partialToolsetConfigSchema>;

const fullToolsetConfigSchema = z.object({
  stracciatellaHome: z.string().min(1).meta({
    title: 'JA2 Stracciatella Home Directory',
    description:
      'This should be auto-populated if you have JA2 Stracciatella installed and started it successfully at least once. It is the location of your JA2 Stracciatella configuration and mods.',
  }),
  vanillaGameDir: z.string().min(1).meta({
    title: 'Vanilla Game Directory',
    description:
      'This should be auto-populated if you have JA2 Stracciatella installed and started it successfully at least once. It is the location of your vanilla (original) Jagged Alliance 2 installation.',
  }),
  stracciatellaInstallDir: z.string().min(1).meta({
    title: 'JA2 Stracciatella Directory',
    description:
      'This should point at your JA2 Straccciatella install directory. It is used to read the JSON files included with the game.',
  }),
  lastSelectedMod: z.nullable(z.string().min(1)).meta({
    title: 'Last Selected Mod',
    description: 'The last mod that was selected in the mod selection dialog.',
  }),
});

export const FULL_TOOLSET_CONFIG_JSON_SCHEMA = toJSONSchema(
  fullToolsetConfigSchema,
);

export type FullToolsetConfig = z.infer<typeof fullToolsetConfigSchema>;

const toolsetConfigSchema = z.union([
  z.object({
    partial: z.literal(true),
    config: partialToolsetConfigSchema,
  }),
  z.object({
    partial: z.literal(false),
    config: fullToolsetConfigSchema,
  }),
]);

export type ToolsetConfig = z.infer<typeof toolsetConfigSchema>;

type ToolsetState = Persistable<ToolsetConfig>;

const initialState: ToolsetState = makePersistable<ToolsetConfig>(null);

export const readToolsetConfig = createAsyncThunk(
  'toolset-config/read',
  async () => {
    return await invokeWithSchema(toolsetConfigSchema, 'read_toolset_config');
  },
);

export const updateToolsetConfig = createAsyncThunk(
  'toolset-config/update',
  async (config: PartialToolsetConfig) => {
    return await invokeWithSchema(
      toolsetConfigSchema,
      'update_toolset_config',
      {
        config,
      },
    );
  },
);

const toolsetSlice = createSlice({
  name: 'toolset-config',
  initialState,
  reducers: {},
  extraReducers: (builder) => {
    buildLoadableMapping(
      builder,
      readToolsetConfig,
      (state) => state,
      (config) => config,
    );
    buildPersistableMapping(
      builder,
      updateToolsetConfig,
      (state) => state,
      (config) => config,
    );
  },
});

export const toolset = toolsetSlice.reducer;
