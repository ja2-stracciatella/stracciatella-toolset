import { invoke } from "@tauri-apps/api";
import { z } from "zod";
import { useCallback, useEffect, useState } from "react";
import { createContainer } from "unstated-next";
import { useJsonWithSchema } from "../hooks/useJsonWithSchema";

interface FilesState {
  inMemory: {
    [path: string]: any;
  };
  saveErrors: {
    [path: string]: Error | undefined;
  };
}

function useFilesState(
  initialState: FilesState = { inMemory: {}, saveErrors: {} }
) {
  const [filesState, setFilesState] = useState<FilesState>(initialState);
  const fileChanged = useCallback(
    (path: string, value: any) =>
      setFilesState((filesState) => ({
        ...filesState,
        inMemory: {
          ...filesState.inMemory,
          [path]: value,
        },
      })),
    []
  );
  const fileSaved = useCallback(
    (path: string) =>
      setFilesState((filesState) => ({
        saveErrors: Object.fromEntries(
          Object.entries(filesState.saveErrors).filter(([k]) => k !== path)
        ),
        inMemory: Object.fromEntries(
          Object.entries(filesState.inMemory).filter(([k]) => k !== path)
        ),
      })),
    []
  );
  const fileError = useCallback(
    (path: string, error: Error) =>
      setFilesState((filesState) => ({
        ...filesState,
        saveErrors: {
          ...filesState.saveErrors,
          [path]: error,
        },
      })),
    []
  );
  const saveFile = useCallback(
    async (path: string) => {
      if (!filesState.inMemory[path]) {
        fileError(path, new Error(`json file "${path}": was not changed`));
        return;
      }

      try {
        await invoke("persist_json_file", {
          path,
          content: filesState.inMemory[path],
        });

        fileSaved(path);
      } catch (e: any) {
        fileError(path, new Error(`cannot persist json file "${path}": ${e}`));
      }
    },
    [fileError, fileSaved, filesState.inMemory]
  );

  return {
    ...filesState,
    fileChanged,
    saveFile,
  };
}

const filesState = createContainer(useFilesState);

export const FilesProvider = filesState.Provider;
export const useFiles = filesState.useContainer;

export function useModifyableJsonWithSchema<Schema, Content>(
  schemaSchema: z.ZodType<Schema>,
  contentSchema: z.ZodType<Content>,
  path: string
) {
  const { inMemory, fileChanged: fileChangedInternal } = useFiles();
  const {
    content = null,
    schema = null,
    error,
    refetch,
  } = useJsonWithSchema(schemaSchema, contentSchema, path);
  const fileChanged = useCallback(
    (value: Content) => fileChangedInternal(path, value),
    [fileChangedInternal, path]
  );

  useEffect(() => {
    if (!inMemory[path]) {
      // refetch when inMemory is cleared (e.g. when saved)
      refetch();
    }
  }, [inMemory, path, refetch]);

  if (!content) {
    return { error, fileChanged };
  }
  return {
    fileChanged,
    modified: !!inMemory[path],
    content: (inMemory[path] as Content) ?? content,
    schema,
    error,
  };
}
