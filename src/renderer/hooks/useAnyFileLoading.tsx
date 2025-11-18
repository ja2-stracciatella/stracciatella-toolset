import { useAppSelector } from './state';

export function useAnyFileLoading(files: string[]): boolean {
  return useAppSelector((state) => {
    let loading = false;
    for (const file of files) {
      loading = loading || (state.files.disk[file]?.loading ?? false);
    }
    return loading;
  });
}
