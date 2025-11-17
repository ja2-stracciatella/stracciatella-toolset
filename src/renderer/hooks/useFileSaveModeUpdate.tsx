import { useCallback } from 'react';
import { useAppDispatch } from './state';
import { changeSaveMode, SaveMode } from '../state/files';

export function useFileSaveModeUpdate(file: string) {
  const dispatch = useAppDispatch();
  return useCallback(
    (saveMode: SaveMode) => {
      dispatch(
        changeSaveMode({
          filename: file,
          saveMode,
        }),
      );
    },
    [dispatch, file],
  );
}
