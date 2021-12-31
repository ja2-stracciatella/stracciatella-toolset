import { useCallback, useEffect, useState } from "react";
import { z } from "zod";
import { invokeWithSchema } from "../lib/invoke";

const JsonWithSchema = z.object({
  content: z.any(),
  schema: z.any(),
});

type JsonWithSchema = z.infer<typeof JsonWithSchema>;

export function useJsonWithSchema(file: string) {
  const [error, setError] = useState<Error | null>(null);
  const [state, setState] = useState<JsonWithSchema | null>(null);
  const fetch = useCallback(async (file: string) => {
    try {
      const state = await invokeWithSchema(
        JsonWithSchema,
        "open_json_file_with_schema",
        {
          file,
        }
      );
      setState(state);
    } catch (e: any) {
        setError(new Error(`error fetching json with schema: ${e}`));
    }
  }, []);

  useEffect(() => {
    fetch(file);
  }, [fetch, file]);

  return { error, data: state };
}
