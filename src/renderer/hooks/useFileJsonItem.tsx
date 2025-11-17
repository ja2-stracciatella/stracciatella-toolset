import { isArray } from 'remeda';
import { useAppSelector } from './state';
import { isPlainObject } from '@reduxjs/toolkit';
import { AnyJsonObject } from '../../common/invokables/jsons';

export function useFileJsonItem(
  file: string,
  index: number,
): AnyJsonObject | null {
  return useAppSelector((s) => {
    const open = s.files.open[file]?.value;
    if (!isArray(open)) return null;
    const item = open[index];
    if (!isPlainObject(item)) return null;
    return item;
  });
}
