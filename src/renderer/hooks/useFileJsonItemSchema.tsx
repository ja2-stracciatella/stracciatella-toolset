import { JsonSchema } from 'src/common/invokables/jsons';
import { useAppSelector } from './state';

export function useFileJsonItemSchema(filename: string): JsonSchema | null {
  return useAppSelector((s) => {
    return s.files.disk[filename]?.data?.itemSchema ?? null;
  });
}
