import { createAsyncThunk, createSlice } from '@reduxjs/toolkit';
import type { SerializedError } from '@reduxjs/toolkit';
import { z } from 'zod';
import { invokeWithSchema } from '../lib/invoke';

const modSchema = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string(),
  version: z.string(),
});

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
  loading: boolean;
  error: SerializedError | null;
  mods: Mods | null;
  selected: EditableMod | null;
}

const initialState: ModsState = {
  loading: true,
  error: null,
  mods: null,
  selected: null,
};

export const getMods = createAsyncThunk('mods/get', async () => {
  const MODS_SCHEMA = z.array(modSchema);
  const EDITABLE_MODS_SCHEMA = z.array(editableModSchema);
  const [available, editable] = await Promise.all([
    invokeWithSchema(MODS_SCHEMA, 'get_available_mods'),
    invokeWithSchema(EDITABLE_MODS_SCHEMA, 'get_editable_mods'),
  ]);
  return {
    available,
    editable,
  };
});

export const setSelectedMod = createAsyncThunk(
  'mods/set_selected',
  async (mod: EditableMod, { getState }) => {
    const { mods } = getState() as { mods: ModsState };
    if (mods.mods == null) {
      throw new Error('cannot select mod while mods are loading');
    }
    if (!mods.mods.editable.find((m) => m.id === mod.id)) {
      throw new Error(`cannot select unknown mod "${mod.id}"`);
    }

    await invokeWithSchema(z.any(), 'set_selected_mod', {
      mod_id: mod.id,
    });

    return mod;
  },
);

const modsSlice = createSlice({
  name: 'mods',
  initialState,
  reducers: {},
  extraReducers: (builder) => {
    const pending = (state: ModsState) => {
      state.loading = true;
      state.error = null;
    };

    builder.addCase(getMods.pending, pending);
    // TODO: Find a way to extract (same as setSelectedMod)
    builder.addCase(getMods.rejected, (state, action) => {
      state.loading = true;
      state.error = action.error;
    });
    // TODO: Find a way to extract (same as setSelectedMod)
    builder.addCase(getMods.fulfilled, (state, action) => {
      state.loading = false;
      state.error = null;
      state.mods = action.payload;
    });

    builder.addCase(setSelectedMod.pending, pending);
    builder.addCase(setSelectedMod.rejected, (state, action) => {
      state.loading = true;
      state.error = action.error;
    });
    builder.addCase(setSelectedMod.fulfilled, (state, action) => {
      state.loading = false;
      state.error = null;
      state.selected = action.payload;
    });
  },
});

export const mods = modsSlice.reducer;
