import { useCallback } from 'react';
import { changeText } from '../state/files';
import { useAppDispatch } from './state';

export function useFileTextUpdate(file: string): (value: string) => unknown {
  const dispatch = useAppDispatch();
  return useCallback(
    (value: string) => {
      dispatch(
        changeText({
          filename: file,
          value,
        }),
      );
    },
    [dispatch, file],
  );
}
