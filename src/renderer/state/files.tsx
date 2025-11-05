import { createAsyncThunk, createSlice } from '@reduxjs/toolkit';
import type { PayloadAction } from '@reduxjs/toolkit';
import { z } from 'zod';
import { applyReducer, Operation, compare } from 'fast-json-patch';
import { invokeWithSchema } from '../lib/invoke';
import { deepEquals } from '@rjsf/utils';
import {
  buildLoadableMapping,
  buildPersistableMapping,
  makePersistable,
  Persistable,
} from './types';

const anyJsonObjectSchema = z.object().catchall(z.any());
const jsonRootSchema = z.union([
  anyJsonObjectSchema,
  z.array(anyJsonObjectSchema),
  z.array(z.array(z.any())),
]);
export type JsonRoot = z.infer<typeof jsonRootSchema>;
const jsonPatchSchema = z.array(
  z.object({ op: z.any(), path: z.any(), value: z.optional(z.any()) }),
);
export type JsonPatch = Array<Operation>;
const jsonSchemaSchema = z
  .object({
    title: z.optional(z.string()),
    description: z.optional(z.string()),
    items: z.optional(anyJsonObjectSchema),
  })
  .catchall(z.any());
export type JsonSchema = z.infer<typeof jsonSchemaSchema>;

export type SaveMode = 'patch' | 'replace';

interface JsonFile {
  saveMode: SaveMode;
  schema: JsonSchema;
  vanilla: JsonRoot;
  mod: JsonRoot | null;
  patch: JsonPatch | null;
  applied: JsonRoot | null;
}

interface FilesState {
  disk: Record<string, Persistable<JsonFile>>;
  open: Record<string, JsonRoot>;
  saveMode: Record<string, SaveMode>;
  modified: Record<string, boolean>;
}

const initialState: FilesState = {
  disk: {},
  saveMode: {},
  open: {},
  modified: {},
};

const invokeResultSchema = z.object({
  schema: jsonSchemaSchema,
  vanilla: jsonRootSchema,
  value: z.union([jsonRootSchema, z.null()]),
  patch: z.union([jsonPatchSchema, z.null()]),
});

type InvokeResult = z.infer<typeof invokeResultSchema>;

export const loadJSON = createAsyncThunk(
  'files/load-json',
  async (filename: string) => {
    return invokeWithSchema(invokeResultSchema, 'open_json_with_schema', {
      filename,
    });
  },
);

export const persistJSON = createAsyncThunk(
  'files/persist-json',
  async (filename: string, { getState }) => {
    const { files } = getState() as { files: FilesState };

    const editorValue = files.open[filename] ?? null;
    const mode = files.saveMode[filename] ?? null;
    if (editorValue === null || mode === null) {
      throw new Error('tried to save a non-open file');
    }

    const vanillaValue = files.disk[filename]?.data?.vanilla ?? null;
    if (vanillaValue === null) {
      throw new Error('tried to save a non-loaded file');
    }

    const toSynchronize: {
      filename: string;
      value: JsonRoot | null;
      patch: JsonPatch | null;
    } = {
      filename,
      value: editorValue,
      patch: null,
    };
    if (mode == 'patch') {
      toSynchronize.value = null;
      toSynchronize.patch = compare(vanillaValue, editorValue);
    }

    return await invokeWithSchema(
      invokeResultSchema,
      'persist_json',
      toSynchronize,
    );
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
  const diskSaveMode = files.disk[filename]?.data?.saveMode ?? null;
  const diskValue = files.disk[filename]?.data?.applied ?? null;
  const value = files.open[filename] ?? null;
  const saveMode = files.saveMode[filename] ?? null;

  return diskSaveMode != saveMode || !deepEquals(diskValue, value);
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

      state.open[filename] = value;
      state.modified[filename] = isModified(state, filename);
    },
    changeJsonItem: (
      state,
      action: PayloadAction<{ filename: string; index: number; value: any }>,
    ) => {
      const { filename, index, value } = action.payload;
      const o = state.open[filename];
      if (!Array.isArray(o)) {
        throw new Error('tried to change item of a non-array file');
      }
      (o as Array<any>)[index] = value;
      state.modified[filename] = isModified(state, filename);
    },
    changeSaveMode(
      state,
      action: PayloadAction<{ filename: string; saveMode: SaveMode }>,
    ) {
      const { filename, saveMode } = action.payload;
      state.saveMode[filename] = saveMode;
      state.modified[filename] = isModified(state, filename);
    },
  },
  extraReducers: (builder) => {
    const transform = (data: InvokeResult) => {
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
        if (!state.saveMode[filename]) {
          state.saveMode[filename] = payload.payload.value
            ? 'replace'
            : 'patch';
        }
        if (!state.open[filename]) {
          state.open[filename] =
            getDiskState(state, filename).data?.applied ?? {};
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
        state.modified[filename] = isModified(state, filename);
      },
    );
  },
});

export const files = filesSlice.reducer;

export const { changeJson, changeJsonItem, changeSaveMode } =
  filesSlice.actions;
