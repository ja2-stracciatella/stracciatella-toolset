import { useCallback } from 'react';
import { useAppDispatch } from './state';
import { persistJSON } from '../state/files';

export function useFileSave(file: string) {
  const dispatch = useAppDispatch();
  return useCallback(() => {
    dispatch(persistJSON(file));
  }, [dispatch, file]);
}
