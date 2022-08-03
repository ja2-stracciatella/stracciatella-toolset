import { useCallback, useEffect, useState } from "react";
import { z } from "zod";
import { invokeWithSchema } from "../lib/invoke";

const jsonWithSchemaSchema = z.object({
  content: z.any(),
  schema: z.any(),
});

type JsonWithSchema = z.infer<typeof jsonWithSchemaSchema>;

export function useJsonWithSchema(file: string) {
  const [error, setError] = useState<Error | null>(null);
  const [state, setState] = useState<
    JsonWithSchema | { content: null; schema: null }
  >({ content: null, schema: null });
  const [needsRefetch, setNeedsRefetch] = useState(true);
  const refetch = useCallback(() => setNeedsRefetch(true), []);
  const fetch = useCallback(async (file: string) => {
    const state = await invokeWithSchema(
      jsonWithSchemaSchema,
      "open_json_file_with_schema",
      {
        file,
      }
    );
    setState(state);
  }, []);

  useEffect(() => {
    if (needsRefetch) {
      fetch(file)
        .catch((e: any) =>
          setError(new Error(`error fetching json with schema: ${e}`))
        )
        .finally(() => setNeedsRefetch(false));
    }
  }, [fetch, file, needsRefetch]);

  return { error, refetch, ...state };
}
