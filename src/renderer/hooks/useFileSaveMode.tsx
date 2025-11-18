import { SaveMode } from '../state/files';
import { useAppSelector } from './state';

export function useFileSaveMode(file: string): SaveMode {
  return useAppSelector((s) => {
    return s.files.open[file]?.saveMode ?? 'patch';
  });
}
