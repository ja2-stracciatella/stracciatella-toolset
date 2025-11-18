import { isArray } from 'remeda';
import { useAppSelector } from './state';

export function useFileJsonItemCount(file: string): number {
  return useAppSelector((state) => {
    const open = state.files.open[file]?.value ?? null;
    if (!open || !isArray(open)) return 0;
    return open.length;
  });
}
