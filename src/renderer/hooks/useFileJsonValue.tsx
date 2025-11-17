import { JsonRoot } from '../../common/invokables/jsons';
import { useAppSelector } from './state';

export function useFileJsonValue(file: string): JsonRoot | null {
  return useAppSelector((s) => {
    const open = s.files.open[file];
    if (!open || open.editMode === 'text') {
      return null;
    }
    return open.value;
  });
}
