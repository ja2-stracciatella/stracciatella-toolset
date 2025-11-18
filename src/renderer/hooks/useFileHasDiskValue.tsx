import { useAppSelector } from './state';

export function useFileHasDiskValue(file: string): boolean {
  return useAppSelector((s) => {
    return !!s.files.disk[file]?.data;
  });
}
