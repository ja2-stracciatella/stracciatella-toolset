import { JsonRoot } from '../../common/invokables/jsons';
import { useAppSelector } from './state';

export function useFileJsonDiskValue(file: string): JsonRoot | null {
  return useAppSelector((s) => {
    return s.files.disk[file]?.data?.applied ?? null;
  });
}
