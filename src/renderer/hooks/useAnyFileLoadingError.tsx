import { SerializedError } from '@reduxjs/toolkit';
import { useAppSelector } from './state';

export function useAnyFileLoadingError(
  files: string[],
): SerializedError | null {
  return useAppSelector((state) => {
    for (const file of files) {
      const error = state.files.disk[file]?.loadingError ?? null;
      if (error) {
        return error;
      }
    }
    return null;
  });
}
