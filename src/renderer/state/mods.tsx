import { createAsyncThunk, createSlice } from '@reduxjs/toolkit';
import { invoke } from '../lib/invoke';
import {
  Loadable,
  makeLoadable,
  Persistable,
  makePersistable,
  buildLoadableMapping,
  buildPersistableMapping,
} from './types';
import { EditableMod, Mod } from '../../common/invokables/mods';

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

export const loadMods = createAsyncThunk(
  'mods/get',
  async (): Promise<Mods> => {
    const [available, editable] = await Promise.all([
      invoke('mods/listAvailable', null),
      invoke('mods/listEditable', null),
    ]);
    return {
      available,
      editable,
    };
  },
);

export const readSelectedMod = createAsyncThunk(
  'mods/read_selected',
  async () => {
    return invoke('mod/readSelected', null);
  },
);

export const updateSelectedMod = createAsyncThunk(
  'mods/update_selected',
  async (mod: EditableMod) => {
    return invoke('mod/updateSelected', mod);
  },
);

export const createNewMod = createAsyncThunk(
  'mods/create_new',
  async (newMod: Mod) => {
    return invoke('mod/create', newMod);
  },
);

const modsSlice = createSlice({
  name: 'mods',
  initialState,
  reducers: {},
  extraReducers: (builder) => {
    buildLoadableMapping(
      builder,
      loadMods,
      (state) => state.mods,
      (mods) => mods,
    );
    buildLoadableMapping(
      builder,
      readSelectedMod,
      (state) => state.selected,
      (selected) => selected,
    );
    buildPersistableMapping(
      builder,
      updateSelectedMod,
      (state) => state.selected,
      (selected) => selected,
    );
    buildPersistableMapping(
      builder,
      createNewMod,
      (state) => state.selected,
      (selected) => selected,
    );
  },
});

export const mods = modsSlice.reducer;
