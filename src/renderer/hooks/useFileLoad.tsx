import { useCallback } from 'react';
import { useAppDispatch } from './state';
import { loadJSON } from '../state/files';

export function useFileLoad(): (file: string) => unknown {
  const dispatch = useAppDispatch();
  return useCallback(
    (file: string) => {
      dispatch(loadJSON(file));
      // Load the file and return the result
    },
    [dispatch],
  );
}
