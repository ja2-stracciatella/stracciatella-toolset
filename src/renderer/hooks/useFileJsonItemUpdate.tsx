import { useCallback } from 'react';
import { useAppDispatch } from './state';
import { changeJsonItem } from '../state/files';

export function useFileJsonItemUpdate(
  file: string,
  index: number,
): (value: object) => unknown {
  const dispatch = useAppDispatch();
  return useCallback(
    (value: object) => {
      dispatch(
        changeJsonItem({
          filename: file,
          index,
          value,
        }),
      );
    },
    [dispatch, file, index],
  );
}
