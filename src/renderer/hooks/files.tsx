import { useCallback, useEffect, useMemo } from 'react';
import { useAppDispatch, useAppSelector } from './state';
import { changeJson, changeJsonItem, loadJSON } from '../state/files';
import { SerializedError } from '@reduxjs/toolkit';
import { shallowEqual } from 'react-redux';

type UseJsonResult = {
  loading: boolean;
  error: SerializedError | null;
  content: {
    modified: boolean;
    value: any;
    schema: any;
  } | null;
};

type UseJsonsResult<T extends { [key: PropertyKey]: string }> = {
  loading: boolean;
  error: SerializedError | null;
  results: { [key in keyof T]: UseJsonResult };
  update: (key: keyof T, value: any) => void;
};

export function useJsons<T extends { [key: string]: string }>(
  files: T,
): UseJsonsResult<T> {
  const dispatch = useAppDispatch();
  const results = useAppSelector(
    (s) => {
      const results: { [key in keyof T]: UseJsonResult } = {} as any;
      for (const key in files) {
        results[key] = {
          loading: s.files.json[files[key]]?.loading ?? false,
          error: s.files.json[files[key]]?.error ?? null,
          content: s.files.json[files[key]]?.content ?? null,
        };
      }
      return results;
    },
    (a, b) => {
      if (shallowEqual(a, b)) return true;
      if (!shallowEqual(Object.keys(a), Object.keys(b))) return false;
      for (const key in a) {
        if (!shallowEqual(a[key], b[key])) return false;
      }
      return true;
    },
  );
  const loading = useMemo(() => {
    for (const r in results) {
      if (results[r].loading) return true;
    }
    return false;
  }, [results]);
  const error = useMemo(() => {
    for (const r in results) {
      if (results[r].error) return results[r].error;
    }
    return null;
  }, [results]);
  const update = useCallback(
    (file: keyof T, value: any) => {
      dispatch(
        changeJson({
          file: files[file],
          value,
        }),
      );
    },
    [files, dispatch],
  );

  useEffect(() => {
    for (const f in files) {
      if (!results[f].loading && !results[f].error && !results[f].content) {
        dispatch(loadJSON(files[f]));
      }
    }
  }, [dispatch, files, loading, results]);

  return {
    loading,
    error,
    results,
    update,
  };
}

export function useJson(
  file: string,
): UseJsonResult & { update: (value: any) => void } {
  const { results, update: updateMany } = useJsons({ f: file });
  const update = useCallback(
    (value: any) => {
      updateMany('f', value);
    },
    [updateMany],
  );

  return {
    ...results.f,
    update,
  };
}

export function useJsonItem(file: string, index: number) {
  const dispatch = useAppDispatch();
  const loading = useAppSelector((s) => s.files.json[file]?.loading) ?? false;
  const error = useAppSelector((s) => s.files.json[file]?.error) ?? null;
  const schema = useAppSelector((s) => s.files.json[file]?.content?.schema);
  const value = useAppSelector(
    (s) => s.files.json[file]?.content?.value[index],
  );
  const itemSchema = useMemo(() => {
    if (!schema) {
      return null;
    }
    const { title: _t, description: _d, ...rest } = schema.items;
    return rest;
  }, [schema]);
  const update = useCallback(
    (v: any) => {
      dispatch(
        changeJsonItem({
          file,
          index,
          value: v,
        }),
      );
    },
    [dispatch, file, index],
  );

  useEffect(() => {
    if (!loading && !schema) {
      dispatch(loadJSON(file));
    }
  }, [schema, dispatch, file, loading]);

  return {
    loading,
    error,
    schema: itemSchema,
    value,
    update,
  };
}
