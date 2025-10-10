import { createAsyncThunk, createSlice } from '@reduxjs/toolkit';
import type { SerializedError } from '@reduxjs/toolkit';
import { z } from 'zod';
import { invokeWithSchema } from '../lib/invoke';

const partialToolsetConfigSchema = z.object({
  stracciatellaHome: z.union([z.string(), z.null()]),
  vanillaGameDir: z.union([z.string(), z.null()]),
  stracciatellaInstallDir: z.union([z.string(), z.null()]),
  lastSelectedMod: z.union([z.string(), z.null()]),
});

export type PartialToolsetConfig = z.infer<typeof partialToolsetConfigSchema>;

const fullToolsetConfigSchema = z.object({
  stracciatellaHome: z.string(),
  vanillaGameDir: z.string(),
  stracciatellaInstallDir: z.string(),
  lastSelectedMod: z.union([z.string(), z.null()]),
});

export type FullToolsetConfig = z.infer<typeof fullToolsetConfigSchema>;

const SerializedSchema = z.object({
  partial: z.boolean(),
  config: partialToolsetConfigSchema,
});

interface PartialToolsetConfigState {
  partial: true;
  value: PartialToolsetConfig;
}

interface FullToolsetConfigState {
  partial: false;
  value: FullToolsetConfig;
}

type ToolsetConfigState = PartialToolsetConfigState | FullToolsetConfigState;

interface ToolsetState {
  loading: boolean;
  error: SerializedError | null;
  config: ToolsetConfigState;
}

const initialState: ToolsetState = {
  loading: true,
  error: null,
  config: {
    partial: true,
    value: {
      stracciatellaHome: null,
      vanillaGameDir: null,
      stracciatellaInstallDir: null,
      lastSelectedMod: null,
    },
  },
};

export const getToolsetConfig = createAsyncThunk(
  'toolset-config/get',
  async () => {
    return invokeWithSchema(SerializedSchema, 'get_toolset_config');
  },
);

export const setToolsetConfig = createAsyncThunk(
  'toolset-config/set',
  async (config: PartialToolsetConfig) => {
    const partial = [
      config.stracciatellaHome,
      config.stracciatellaInstallDir,
      config.vanillaGameDir,
    ].some((v) => v === null);

    return invokeWithSchema(SerializedSchema, 'set_toolset_config', {
      partial,
      config,
    });
  },
);

const toolsetSlice = createSlice({
  name: 'toolset-config',
  initialState,
  reducers: {},
  extraReducers: (builder) => {
    const pending = (state: ToolsetState) => {
      state.loading = true;
      state.error = null;
    };

    builder.addCase(getToolsetConfig.pending, pending);
    // TODO: Find a way to extract (same as setToolsetConfig)
    builder.addCase(getToolsetConfig.rejected, (state, action) => {
      state.loading = true;
      state.error = action.error;
    });
    // TODO: Find a way to extract (same as setToolsetConfig)
    builder.addCase(getToolsetConfig.fulfilled, (state, action) => {
      state.loading = false;
      state.error = null;
      if (action.payload.partial) {
        state.config = {
          partial: true,
          value: action.payload.config,
        };
      } else {
        state.config = {
          partial: false,
          value: fullToolsetConfigSchema.parse(action.payload.config),
        };
      }
    });

    builder.addCase(setToolsetConfig.pending, pending);
    builder.addCase(setToolsetConfig.rejected, (state, action) => {
      state.loading = false;
      state.error = action.error;
    });
    builder.addCase(setToolsetConfig.fulfilled, (state, action) => {
      state.loading = false;
      state.error = null;
      if (action.payload.partial) {
        state.config = {
          partial: true,
          value: action.payload.config,
        };
      } else {
        state.config = {
          partial: false,
          value: fullToolsetConfigSchema.parse(action.payload.config),
        };
      }
    });
  },
});

export const toolset = toolsetSlice.reducer;
