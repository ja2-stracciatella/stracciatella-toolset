import { EditMode } from '../state/files';
import { useAppSelector } from './state';

export function useFileEditMode(file: string): EditMode {
  return useAppSelector((s) => {
    return s.files.open[file]?.editMode ?? 'visual';
  });
}
