import { useCallback, useEffect } from 'react';
import { useAppDispatch, useAppSelector } from './state';
import { changeJson, changeJsonItem, loadJSON, SaveMode } from '../state/files';
import { SerializedError } from '@reduxjs/toolkit';
import { AppState } from '../state/store';
import { memoize } from 'proxy-memoize';
import { JsonRoot, JsonSchema } from '../../common/invokables/jsons';

type UseFilesRequest = { [key: PropertyKey]: string };

type UseFilesResult<R extends UseFilesRequest, V> = {
  [key in keyof R]: V;
};

const useAppFilesProxySelector = function useAppFilesProxySelector<R>(
  fn: (state: AppState) => R,
  deps: any[],
): R {
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return useAppSelector(useCallback(memoize(fn), deps));
};

export function useFilesLoading<R extends UseFilesRequest>(
  files: R,
): UseFilesResult<R, boolean | null> {
  const loading = useAppFilesProxySelector(
    (s: AppState) => {
      const loading: { [key in keyof R]: boolean | null } = {} as any;
      for (const key in files) {
        loading[key] = s.files.disk[files[key]]?.loading ?? null;
      }
      return loading;
    },
    [files],
  );

  return loading;
}

export function useFilesError<R extends UseFilesRequest>(
  files: R,
): UseFilesResult<R, SerializedError | null> {
  const errors = useAppFilesProxySelector(
    function selectFilesError(s) {
      const errors: { [key in keyof R]: SerializedError | null } = {} as any;
      for (const key in files) {
        errors[key] = s.files.disk[files[key]]?.loadingError ?? null;
      }
      return errors;
    },
    [files],
  );

  return errors;
}

export function useFilesSchema<R extends UseFilesRequest>(
  files: R,
): UseFilesResult<R, Record<string, any> | null> {
  const schemas = useAppFilesProxySelector(
    function selectFilesSchema(s) {
      const schemas: { [key in keyof R]: Record<string, any> | null } =
        {} as any;
      for (const key in files) {
        schemas[key] = s.files.disk[files[key]]?.data?.schema ?? null;
      }
      return schemas;
    },
    [files],
  );

  return schemas;
}

export function useFilesJson<R extends UseFilesRequest>(
  files: R,
): {
  values: UseFilesResult<R, JsonRoot | null>;
  update: (file: keyof R, value: JsonRoot) => void;
} {
  const dispatch = useAppDispatch();
  const loading = useFilesLoading(files);
  const values = useAppFilesProxySelector(
    function selectFilesJson(s) {
      const values: { [key in keyof R]: JsonRoot | null } = {} as any;
      for (const key in files) {
        values[key] = s.files.open[files[key]]?.value ?? null;
      }
      return values;
    },
    [files],
  );
  const update = useCallback(
    (file: keyof R, value: JsonRoot) => {
      dispatch(
        changeJson({
          filename: files[file],
          value,
        }),
      );
    },
    [files, dispatch],
  );

  useEffect(() => {
    for (const key in files) {
      if (loading[key] === null) {
        dispatch(loadJSON(files[key]));
      }
    }
  }, [dispatch, files, loading]);

  return {
    values,
    update,
  };
}

export function useFileLoading(filename: string): boolean | null {
  return useAppSelector(function selectFileLoading(s) {
    return s.files.disk[filename]?.loading ?? null;
  });
}

export function useFileSaving(filename: string): boolean | null {
  return useAppSelector(function selectFileSaving(s) {
    return s.files.disk[filename]?.persisting ?? null;
  });
}

export function useFileSaveMode(filename: string): SaveMode | null {
  return useAppSelector(function selectFileSaveMode(s) {
    return s.files.open[filename]?.saveMode ?? null;
  });
}

export function useFileError(filename: string): SerializedError | null {
  return useAppSelector(function selectFileError(s) {
    return s.files.disk[filename]?.loadingError ?? null;
  });
}

export function useFileSchema(filename: string): JsonSchema | null {
  return useAppSelector(function selectFileSchema(s) {
    return s.files.disk[filename]?.data?.schema ?? null;
  });
}

export function useFileModified(filename: string): boolean | null {
  return useAppSelector(function selectFileModified(s) {
    return s.files.open[filename]?.modified ?? null;
  });
}

export function useFileJson(
  filename: string,
): [JsonRoot | null, (value: JsonRoot) => void] {
  const dispatch = useAppDispatch();
  const loading = useFileLoading(filename);
  const update = useCallback(
    (value: JsonRoot) => {
      dispatch(
        changeJson({
          filename,
          value,
        }),
      );
    },
    [filename, dispatch],
  );

  useEffect(() => {
    if (loading === null) {
      dispatch(loadJSON(filename));
    }
  }, [dispatch, filename, loading]);

  return [
    useAppSelector((s) => {
      return s.files.open[filename]?.value ?? null;
    }),
    update,
  ];
}

export function useFileJsonItemSchema(
  filename: string,
): Record<string, any> | null {
  const schema = useFileSchema(filename);
  if (!schema) return null;
  return schema.items ?? null;
}

export function useFileJsonNumberOfItems(filename: string): number | null {
  const [arr] = useFileJson(filename);
  if (!arr) return null;
  if (!Array.isArray(arr)) return null;

  return arr.length;
}

export function useFileJsonItem(
  filename: string,
  index: number,
): [Record<string, any> | null, (value: Record<string, any>) => void] {
  const dispatch = useAppDispatch();
  const [arr] = useFileJson(filename);
  const update = useCallback(
    (value: Record<string, any>) => {
      dispatch(
        changeJsonItem({
          filename,
          index,
          value,
        }),
      );
    },
    [dispatch, filename, index],
  );
  if (!arr) return [null, update];
  if (!Array.isArray(arr)) return [null, update];

  return [arr[index] ?? null, update];
}
