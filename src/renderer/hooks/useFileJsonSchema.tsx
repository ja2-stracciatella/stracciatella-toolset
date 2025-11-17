import { JsonSchema } from '../../common/invokables/jsons';
import { useAppSelector } from './state';

export function useFileJsonSchema(file: string): JsonSchema | null {
  return useAppSelector((s) => {
    return s.files.disk[file]?.data?.schema ?? null;
  });
}
