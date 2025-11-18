import { useAppSelector } from './state';

export function useFileModified(filename: string): boolean {
  return useAppSelector((s) => {
    return s.files.open[filename]?.modified ?? false;
  });
}
