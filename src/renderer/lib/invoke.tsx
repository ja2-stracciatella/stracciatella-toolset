// import { invoke, InvokeArgs } from '@tauri-apps/api/tauri';
import { z } from 'zod';

function invoke(func: string, params: Record<string, unknown>): unknown {
  return window.electronAPI.invoke({ func, params });
}

export async function invokeWithSchema<T>(
  schema: z.ZodType<T>,
  func: string,
  params: Record<string, unknown> = {}
): Promise<T> {
  const res = await invoke(func, params);
  return schema.parse(res);
}
