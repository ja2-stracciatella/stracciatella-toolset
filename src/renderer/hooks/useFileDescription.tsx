import { useAppSelector } from './state';

export function useFileDescription(file: string): string | null {
  return useAppSelector(
    (state) => state.files.disk[file]?.data?.description ?? null,
  );
}
