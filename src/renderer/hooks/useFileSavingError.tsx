import { SerializedError } from '@reduxjs/toolkit';
import { useAppSelector } from './state';

export function useFileSavingError(filename: string): SerializedError | null {
  return useAppSelector((s) => {
    return s.files.disk[filename]?.persistingError ?? null;
  });
}
