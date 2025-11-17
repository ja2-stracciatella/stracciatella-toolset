import { useCallback } from 'react';
import { JsonRoot } from '../../common/invokables/jsons';
import { changeJson } from '../state/files';
import { useAppDispatch } from './state';

export function useFileJsonUpdate(file: string): (value: JsonRoot) => unknown {
  const dispatch = useAppDispatch();
  return useCallback(
    (value: JsonRoot) => {
      dispatch(
        changeJson({
          filename: file,
          value,
        }),
      );
    },
    [dispatch, file],
  );
}
