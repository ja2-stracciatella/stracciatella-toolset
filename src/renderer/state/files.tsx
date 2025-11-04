import {
  createAsyncThunk,
  createSlice,
  miniSerializeError,
} from '@reduxjs/toolkit';
import type { PayloadAction, SerializedError } from '@reduxjs/toolkit';
import { z } from 'zod';
import { applyReducer, Operation, compare } from 'fast-json-patch';
import { invokeWithSchema } from '../lib/invoke';
import { deepEquals } from '@rjsf/utils';

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
  loading: boolean | null;
  saving: boolean | null;
  error: SerializedError | null;
  content: {
    schema: JsonSchema;
    vanilla: JsonRoot;
    mod: JsonRoot | null;
    patch: JsonPatch | null;
    applied: JsonRoot | null;
    saveMode: SaveMode;
  } | null;
}

interface FilesState {
  disk: Record<string, JsonFile>;
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

export const loadJSON = createAsyncThunk(
  'files/load-json',
  async (filename: string) => {
    return invokeWithSchema(
      z.object({
        schema: jsonSchemaSchema,
        vanilla: jsonRootSchema,
        value: z.union([jsonRootSchema, z.null()]),
        patch: z.union([jsonPatchSchema, z.null()]),
      }),
      'open_json_with_schema',
      {
        filename,
      },
    );
  },
);

export const persistJSON = createAsyncThunk(
  'files/persist-json',
  async ({ filename }: { filename: string }, { getState }) => {
    const { files } = getState() as { files: FilesState };

    const editorValue = files.open[filename] ?? null;
    const mode = files.saveMode[filename] ?? null;
    if (editorValue === null || mode === null) {
      throw new Error('tried to save a non-open file');
    }

    const vanillaValue = files.disk[filename]?.content?.vanilla ?? null;
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

    return invokeWithSchema(
      z.object({
        value: z.union([jsonRootSchema, z.null()]),
        patch: z.union([jsonPatchSchema, z.null()]),
      }),
      'persist_json',
      toSynchronize,
    );
  },
);

function getDiskState(state: FilesState, filename: string): JsonFile {
  if (!state.disk[filename]) {
    state.disk[filename] = {
      saving: null,
      loading: null,
      error: null,
      content: null,
    };
  }
  return state.disk[filename];
}

function isModified(files: FilesState, filename: string) {
  const diskSaveMode = files.disk[filename]?.content?.saveMode ?? null;
  const diskValue = files.disk[filename]?.content?.applied ?? null;
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
      const f = getDiskState(state, filename);
      const o = state.open[filename];
      if (!Array.isArray(o)) {
        f.error = miniSerializeError(
          new Error('tried to change item of a non-array file'),
        );
        return;
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
    builder.addCase(loadJSON.pending, (state, action) => {
      const file = action.meta.arg;
      const c = getDiskState(state, file);
      c.loading = true;
      c.error = null;
    });
    // TODO: Find a way to extract (same as setSelectedMod)
    builder.addCase(loadJSON.rejected, (state, action) => {
      const file = action.meta.arg;
      const c = getDiskState(state, file);
      c.loading = false;
      c.error = action.error;
    });
    // TODO: Find a way to extract (same as setSelectedMod)
    builder.addCase(loadJSON.fulfilled, (state, action) => {
      const file = action.meta.arg;
      const c = getDiskState(state, file);
      const saveMode = action.payload.value ? 'replace' : 'patch';
      const applied = (action.payload.patch ?? []).reduce(
        applyReducer,
        JSON.parse(
          JSON.stringify(action.payload.value ?? action.payload.vanilla),
        ),
      );

      c.loading = false;
      c.error = null;
      c.content = {
        schema: action.payload.schema,
        vanilla: action.payload.vanilla,
        mod: action.payload.value,
        patch: action.payload.patch as Array<Operation> | null,
        applied,
        saveMode,
      };

      if (!state.saveMode[file]) {
        state.saveMode[file] = action.payload.value ? 'replace' : 'patch';
      }
      if (!state.open[file]) {
        state.open[file] = applied;
      }
    });

    builder.addCase(persistJSON.pending, (state, action) => {
      const { filename } = action.meta.arg;
      const c = getDiskState(state, filename);
      c.saving = true;
      c.error = null;
    });
    builder.addCase(persistJSON.rejected, (state, action) => {
      const { filename } = action.meta.arg;
      const c = getDiskState(state, filename);
      c.saving = false;
      c.error = action.error;
    });
    builder.addCase(persistJSON.fulfilled, (state, action) => {
      const { filename } = action.meta.arg;

      const c = getDiskState(state, filename);
      c.saving = false;

      const { content } = c;
      if (content === null) {
        c.error = miniSerializeError(new Error('saved non-loaded file'));
        return;
      }

      const o = state.open[filename];
      if (o === null) {
        c.error = miniSerializeError(new Error('saved non-open file'));
        return;
      }

      content.mod = action.payload.value;
      content.patch = action.payload.patch;
      content.saveMode = state.saveMode[filename];

      const applied = (content.patch ?? []).reduce(
        applyReducer,
        JSON.parse(JSON.stringify(content.mod ?? content.vanilla)),
      );
      content.applied = applied;

      state.modified[filename] = isModified(state, filename);
    });
  },
});

export const files = filesSlice.reducer;

export const { changeJson, changeJsonItem, changeSaveMode } =
  filesSlice.actions;
