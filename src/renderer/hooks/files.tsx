import { useCallback, useEffect, useMemo } from 'react';
import { useAppDispatch, useAppSelector } from './state';
import { changeJson, changeJsonItem, loadJSON } from '../state/files';

export function useJson(file: string) {
  const dispatch = useAppDispatch();
  const loading = useAppSelector((s) => s.files.json[file]?.loading) ?? false;
  const error = useAppSelector((s) => s.files.json[file]?.error) ?? null;
  const content = useAppSelector((s) => s.files.json[file]?.content);
  const update = useCallback(
    (value: any) => {
      dispatch(
        changeJson({
          file,
          value,
        }),
      );
    },
    [dispatch, file],
  );

  useEffect(() => {
    if (!loading && !content) {
      dispatch(loadJSON(file));
    }
  }, [content, dispatch, file, loading]);

  return {
    loading,
    error,
    content,
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
