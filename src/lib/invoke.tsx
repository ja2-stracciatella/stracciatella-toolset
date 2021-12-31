import { invoke, InvokeArgs } from "@tauri-apps/api/tauri";
import { z } from "zod";

export async function invokeWithSchema<T>(schema: z.ZodType<T>, method: string, args?: InvokeArgs): Promise<T> {
    const availableModsResponse = await invoke(method, args);
    return schema.parse(availableModsResponse);
}