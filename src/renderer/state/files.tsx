import {
  createAsyncThunk,
  createSelector,
  createSlice,
} from '@reduxjs/toolkit';
import type { PayloadAction } from '@reduxjs/toolkit';
import { applyReducer, Operation, compare } from 'fast-json-patch';
import { invoke } from '../lib/invoke';
import { deepEquals } from '@rjsf/utils';
import {
  buildLoadableMapping,
  buildPersistableMapping,
  makePersistable,
  Persistable,
} from './types';
import {
  JsonReadInvokable,
  JsonRoot,
  JsonSchema,
} from '../../common/invokables/jsons';
import { InvokableOutput } from 'src/common/invokables';

export type JsonPatch = Array<Operation>;

export type SaveMode = 'patch' | 'replace';

interface JsonFile {
  saveMode: SaveMode;
  schema: JsonSchema;
  vanilla: JsonRoot;
  mod: JsonRoot | null;
  patch: JsonPatch | null;
  applied: JsonRoot | null;
}

interface Open {
  modified: boolean;
  saveMode: SaveMode;
  value: JsonRoot;
}

interface FilesState {
  disk: Record<string, Persistable<JsonFile> | undefined>;
  open: Record<string, Open | undefined>;
}

const initialState: FilesState = {
  disk: {},
  open: {},
};

export const loadJSON = createAsyncThunk(
  'files/load-json',
  async (filename: string) => {
    return invoke('json/read', {
      file: filename,
    });
  },
);

export const persistJSON = createAsyncThunk(
  'files/persist-json',
  async (filename: string, { getState }) => {
    const { files } = getState() as { files: FilesState };
    const open = files.open[filename];
    if (!open) {
      throw new Error('tried to save a non-open file');
    }
    const disk = files.disk[filename]?.data ?? null;
    if (!disk) {
      throw new Error('tried to save a non-loaded file');
    }

    const editorValue = open.value;
    const mode = open.saveMode;
    const vanillaValue = disk.vanilla ?? null;

    const toSynchronize: {
      file: string;
      value: JsonRoot | null;
      patch: JsonPatch | null;
    } = {
      file: filename,
      value: null,
      patch: null,
    };
    if (mode == 'patch') {
      toSynchronize.patch = compare(vanillaValue, editorValue);
    } else {
      toSynchronize.value = editorValue;
    }

    return invoke('json/persist', toSynchronize);
  },
);

function getDiskState(
  state: FilesState,
  filename: string,
): Persistable<JsonFile> {
  if (!state.disk[filename]) {
    state.disk[filename] = makePersistable<JsonFile>(null);
  }
  return state.disk[filename];
}

function isModified(files: FilesState, filename: string) {
  const open = files.open[filename];
  const disk = files.disk[filename]?.data;
  if (!open || !disk) {
    return false;
  }

  return (
    disk.saveMode != open.saveMode || !deepEquals(disk.applied, open.value)
  );
}

const filesSlice = createSlice({
  name: 'files',
  initialState,
  reducers: {
    changeJson: (
      state,
      action: PayloadAction<{ filename: string; value: JsonRoot }>,
    ) => {
      const { filename, value } = action.payload;
      const open = state.open[filename];
      if (!open) {
        throw new Error(`file ${filename} not open`);
      }

      open.value = value;
      open.modified = isModified(state, filename);
    },
    changeJsonItem: (
      state,
      action: PayloadAction<{ filename: string; index: number; value: any }>,
    ) => {
      const { filename, index, value } = action.payload;
      const open = state.open[filename];
      if (!open) {
        throw new Error(`file ${filename} not open`);
      }
      if (!Array.isArray(open.value)) {
        throw new Error('tried to change item of a non-array file');
      }
      (open.value as Array<any>)[index] = value;
      open.modified = isModified(state, filename);
    },
    changeSaveMode(
      state,
      action: PayloadAction<{ filename: string; saveMode: SaveMode }>,
    ) {
      const { filename, saveMode } = action.payload;
      const open = state.open[filename];
      if (!open) {
        throw new Error(`file ${filename} not open`);
      }
      open.saveMode = saveMode;
      open.modified = isModified(state, filename);
    },
  },
  extraReducers: (builder) => {
    const transform = (data: InvokableOutput<JsonReadInvokable>) => {
      const saveMode: SaveMode = data.value ? 'replace' : 'patch';
      const applied = (data.patch ?? []).reduce(
        applyReducer,
        JSON.parse(JSON.stringify(data.value ?? data.vanilla)),
      );

      return {
        schema: data.schema,
        vanilla: data.vanilla,
        mod: data.value,
        patch: data.patch as Array<Operation> | null,
        applied,
        saveMode,
      };
    };
    buildLoadableMapping(
      builder,
      loadJSON,
      (state, payload) => getDiskState(state, payload.meta.arg),
      transform,
      (state, payload) => {
        const filename = payload.meta.arg;
        if (!state.open[filename]) {
          state.open[filename] = {
            saveMode: payload.payload.value ? 'replace' : 'patch',
            modified: false,
            value: getDiskState(state, filename).data?.applied ?? {},
          };
        } else {
          state.open[filename].modified = isModified(state, filename);
        }
      },
    );
    buildPersistableMapping(
      builder,
      persistJSON,
      (state, payload) => getDiskState(state, payload.meta.arg),
      transform,
      (state, payload) => {
        const filename = payload.meta.arg;
        if (state.open[filename]) {
          state.open[filename].modified = isModified(state, filename);
        }
      },
    );
  },
  selectors: {
    selectModifiedFiles: createSelector(
      (state) => state.open,
      (open: FilesState['open']) =>
        Object.entries(open).flatMap(([filename, open]) =>
          open?.modified ? [filename] : [],
        ),
    ),
  },
});

export const files = filesSlice.reducer;

export const { changeJson, changeJsonItem, changeSaveMode } =
  filesSlice.actions;

export const { selectModifiedFiles } = filesSlice.selectors;
