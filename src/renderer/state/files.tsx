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

export type EditMode = 'visual' | 'text';

interface JsonFile {
  saveMode: SaveMode;
  schema: JsonSchema;
  vanilla: JsonRoot;
  mod: JsonRoot | null;
  patch: JsonPatch | null;
  applied: JsonRoot;
}

interface OpenBase {
  modified: boolean;
  saveMode: SaveMode;
  editMode: EditMode;
}

type Open =
  | (OpenBase & {
      editMode: 'visual';
      value: JsonRoot;
    })
  | (OpenBase & {
      editMode: 'text';
      value: string;
    });

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

    let value: JsonRoot | null = null;
    let patch: JsonPatch | null = null;
    if (open.editMode === 'visual') {
      if (open.saveMode == 'patch') {
        patch = generatePatch(disk.vanilla, open.value);
      } else {
        value = open.value;
      }
    } else {
      if (open.saveMode === 'patch') {
        patch = JSON.parse(open.value);
      } else {
        value = JSON.parse(open.value);
      }
    }

    return invoke('json/persist', {
      file: filename,
      value,
      patch,
    });
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

function generatePatch(value: JsonRoot, applied: JsonRoot): JsonPatch {
  return compare(value, applied);
}

function applyPatch(value: JsonRoot, patch: JsonPatch): JsonRoot {
  return patch.reduce(applyReducer, JSON.parse(JSON.stringify(value)));
}

function jsonToString(value: JsonRoot | JsonPatch): string {
  return JSON.stringify(value, null, 4);
}

function isModified(files: FilesState, filename: string) {
  const open = files.open[filename];
  const disk = files.disk[filename]?.data;
  if (!open || !disk) {
    return false;
  }
  if (open.saveMode !== disk.saveMode) {
    return false;
  }

  let value = open.value;
  if (open.editMode === 'text') {
    try {
      if (open.saveMode === 'patch') {
        const patch = JSON.parse(open.value);
        value = applyPatch(disk.vanilla, patch);
      } else {
        value = JSON.parse(open.value);
      }
    } catch {
      return true;
    }
  }
  return !deepEquals(disk.applied, value);
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
      const disk = state.disk[filename]?.data;
      if (!open) {
        throw new Error(`file ${filename} not open`);
      }
      if (!disk) {
        throw new Error(`file ${filename} not loaded`);
      }
      if (open.saveMode === saveMode) return;

      open.saveMode = saveMode;
      open.modified = isModified(state, filename);
      if (open.editMode === 'text') {
        if (open.saveMode === 'patch') {
          const value = JSON.parse(open.value);
          open.value = jsonToString(generatePatch(disk.applied, value));
        } else {
          const patch = JSON.parse(open.value);
          open.value = jsonToString(applyPatch(disk.applied, patch));
        }
      }
    },
    changeEditMode(
      state,
      action: PayloadAction<{ filename: string; editMode: EditMode }>,
    ) {
      const { filename, editMode } = action.payload;
      const open = state.open[filename];
      const disk = state.disk[filename]?.data;
      if (!open) {
        throw new Error(`file ${filename} not open`);
      }
      if (!disk) {
        throw new Error(`file ${filename} not loaded`);
      }
      if (open.editMode === editMode) return;

      if (open.editMode === 'text') {
        if (open.saveMode === 'patch') {
          const patch = JSON.parse(open.value);
          const applied = applyPatch(disk.applied, patch);
          state.open[filename] = {
            ...open,
            editMode: editMode as 'visual',
            value: applied,
          };
        } else {
          const value = JSON.parse(open.value);
          state.open[filename] = {
            ...open,
            editMode: editMode as 'visual',
            value: value,
          };
        }
      } else {
        if (open.saveMode === 'patch') {
          const value = open.value;
          const patch = generatePatch(disk.applied, value);
          state.open[filename] = {
            ...open,
            editMode: editMode as 'text',
            value: jsonToString(patch),
          };
        } else {
          const value = open.value;
          state.open[filename] = {
            ...open,
            editMode: editMode as 'text',
            value: jsonToString(value),
          };
        }
      }
    },
  },
  extraReducers: (builder) => {
    const transform = (data: InvokableOutput<JsonReadInvokable>) => {
      const saveMode: SaveMode = data.value ? 'replace' : 'patch';
      const applied = applyPatch(data.value ?? data.vanilla, data.patch ?? []);

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
            editMode: 'visual',
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

export const { changeJson, changeJsonItem, changeSaveMode, changeEditMode } =
  filesSlice.actions;

export const { selectModifiedFiles } = filesSlice.selectors;
