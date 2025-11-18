import { useAppSelector } from './state';

export function useFileLoading(filename: string): boolean {
  return useAppSelector((s) => {
    return s.files.disk[filename]?.loading ?? false;
  });
}
