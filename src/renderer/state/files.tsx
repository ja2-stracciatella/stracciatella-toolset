import {
  createAsyncThunk,
  createSlice,
  miniSerializeError,
} from '@reduxjs/toolkit';
import type { PayloadAction, SerializedError } from '@reduxjs/toolkit';
import { z } from 'zod';
import { invokeWithSchema } from '../lib/invoke';

interface JsonFile {
  loading: boolean;
  error: SerializedError | null;
  content: {
    modified: boolean;
    schema: any;
    value: any;
  } | null;
}

interface FilesState {
  json: {
    [path: string]: JsonFile | undefined;
  };
}

const initialState: FilesState = {
  json: {},
};

export const loadJSON = createAsyncThunk(
  'files/load-json',
  async (file: string) => {
    return invokeWithSchema(
      z.object({
        content: z.any(),
        schema: z.any(),
      }),
      'open_json_file_with_schema',
      {
        file,
      },
    );
  },
);

export const saveJSON = createAsyncThunk(
  'files/save-json',
  async (file: string, { getState }) => {
    const { files } = getState() as { files: FilesState };
    const content = files.json[file]?.content?.value;

    if (!content) {
      throw new Error('no content for file to save');
    }

    return invokeWithSchema(z.any(), 'persist_json_file', {
      file,
      content,
    });
  },
);

function getJsonState(state: FilesState, file: string): JsonFile {
  let f = state.json[file];
  if (!f) {
    f = {
      loading: true,
      error: null,
      content: null,
    };
    state.json[file] = f;
  }
  return f;
}

const filesSlice = createSlice({
  name: 'files',
  initialState,
  reducers: {
    changeJson: (
      state,
      action: PayloadAction<{ file: string; value: any }>,
    ) => {
      const c = getJsonState(state, action.payload.file);
      c.loading = false;
      if (c.content) {
        c.error = null;
        c.content.modified = true;
        c.content.value = action.payload.value;
      } else {
        c.error = miniSerializeError(
          new Error('changed json content of unloaded file'),
        );
      }
    },
    changeJsonItem: (
      state,
      action: PayloadAction<{ file: string; index: number; value: any }>,
    ) => {
      const c = getJsonState(state, action.payload.file);
      c.loading = false;
      if (c.content) {
        if (c.content.value.length) {
          c.error = null;
          c.content.modified = true;
          c.content.value[action.payload.index] = action.payload.value;
        } else {
          c.error = miniSerializeError(
            new Error('changed json item of non array file'),
          );
        }
      } else {
        c.error = miniSerializeError(
          new Error('changed json item of unloaded file'),
        );
      }
    },
  },
  extraReducers: (builder) => {
    builder.addCase(loadJSON.pending, (state, action) => {
      const file = action.meta.arg;
      const c = getJsonState(state, file);
      c.loading = true;
      c.error = null;
    });
    // TODO: Find a way to extract (same as setSelectedMod)
    builder.addCase(loadJSON.rejected, (state, action) => {
      const file = action.meta.arg;
      const c = getJsonState(state, file);
      c.loading = false;
      c.error = action.error;
    });
    // TODO: Find a way to extract (same as setSelectedMod)
    builder.addCase(loadJSON.fulfilled, (state, action) => {
      const file = action.meta.arg;
      const c = getJsonState(state, file);
      c.loading = false;
      c.error = null;
      c.content = {
        modified: false,
        schema: action.payload.schema,
        value: action.payload.content,
      };
    });

    builder.addCase(saveJSON.pending, (state, action) => {
      const file = action.meta.arg;
      const c = getJsonState(state, file);
      c.error = null;
    });
    builder.addCase(saveJSON.rejected, (state, action) => {
      const file = action.meta.arg;
      const c = getJsonState(state, file);
      c.error = action.error;
    });
    builder.addCase(saveJSON.fulfilled, (state, action) => {
      const file = action.meta.arg;
      const c = getJsonState(state, file);
      const { content } = c;
      if (content) {
        content.modified = false;
      }
    });
  },
});

export const files = filesSlice.reducer;

export const { changeJson } = filesSlice.actions;
export const { changeJsonItem } = filesSlice.actions;
