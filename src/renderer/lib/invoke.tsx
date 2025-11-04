// import { invoke, InvokeArgs } from '@tauri-apps/api/tauri';
import { z } from 'zod';

function invoke(func: string, params: Record<string, unknown> | null): unknown {
  return window.electronAPI.invoke({ func, params });
}

export async function invokeWithSchema<T>(
  schema: z.ZodType<T>,
  func: string,
  params: Record<string, unknown> | null = null,
): Promise<T> {
  try {
    const res = await invoke(func, params);
    return schema.parse(res);
  } catch (error) {
    if (error instanceof z.ZodError) {
      throw new Error(
        `failed to parse response from invoke ${func} with params ${JSON.stringify(params)}: ${error.message}`,
      );
    }
    throw error;
  }
}
