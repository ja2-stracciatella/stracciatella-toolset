import { useAppSelector } from './state';

export function useFileTitle(file: string): string {
  return useAppSelector((state) => state.files.disk[file]?.data?.title ?? '');
}
