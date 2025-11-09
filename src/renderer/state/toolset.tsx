import { createAsyncThunk, createSlice, PayloadAction } from '@reduxjs/toolkit';
import { invoke } from '../lib/invoke';
import {
  buildLoadableMapping,
  buildPersistableMapping,
  makePersistable,
  Persistable,
} from './types';
import {
  PartialToolsetConfig,
  ToolsetConfig,
} from '../../common/invokables/toolset';

type ToolsetState = {
  config: Persistable<ToolsetConfig>;
  closeRequested: boolean;
};

const initialState: ToolsetState = {
  config: makePersistable<ToolsetConfig>(null),
  closeRequested: false,
};

export const readToolsetConfig = createAsyncThunk(
  'toolset-config/read',
  async () => {
    return await invoke('toolset/readConfig', null);
  },
);

export const updateToolsetConfig = createAsyncThunk(
  'toolset-config/update',
  async (config: PartialToolsetConfig) => {
    return await invoke('toolset/updateConfig', config);
  },
);

const toolsetSlice = createSlice({
  name: 'toolset-config',
  initialState,
  reducers: {
    setCloseRequested: (state, action: PayloadAction<boolean>) => {
      state.closeRequested = action.payload;
    },
  },
  extraReducers: (builder) => {
    buildLoadableMapping(
      builder,
      readToolsetConfig,
      (state) => state.config,
      (config) => config,
    );
    buildPersistableMapping(
      builder,
      updateToolsetConfig,
      (state) => state.config,
      (config) => config,
    );
  },
});

export const toolset = toolsetSlice.reducer;

export const { setCloseRequested } = toolsetSlice.actions;
