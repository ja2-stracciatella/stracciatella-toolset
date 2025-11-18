import { useAppSelector } from './state';

export function useFileSaving(filename: string): boolean {
  return useAppSelector((s) => {
    return s.files.disk[filename]?.persisting ?? false;
  });
}
