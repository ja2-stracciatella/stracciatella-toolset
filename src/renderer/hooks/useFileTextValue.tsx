import { useAppSelector } from './state';

export function useFileTextValue(file: string): string | null {
  return useAppSelector((s) => {
    const open = s.files.open[file];
    if (!open || open.editMode === 'visual') {
      return null;
    }
    return open.value;
  });
}
