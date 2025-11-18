import { useCallback } from 'react';
import { useAppDispatch } from './state';
import { changeEditMode, EditMode } from '../state/files';

export function useFileEditModeUpdate(file: string) {
  const dispatch = useAppDispatch();
  return useCallback(
    (editMode: EditMode) => {
      dispatch(
        changeEditMode({
          filename: file,
          editMode,
        }),
      );
    },
    [dispatch, file],
  );
}
