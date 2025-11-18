import { JsonRoot } from '../../common/invokables/jsons';
import { createAppSelector, useAppSelector } from './state';
import { AppState } from '../state/store';
import { useMemo } from 'react';

function createSelector(files: string[]) {
  return createAppSelector([(state: AppState) => state.files.disk], (disk) =>
    files.map((file) => disk[file]?.data?.applied ?? null),
  );
}

export function useFilesJsonDiskValue(
  files: Array<string>,
): Array<JsonRoot | null> {
  const selector = useMemo(() => createSelector(files), [files]);
  return useAppSelector(selector);
}
