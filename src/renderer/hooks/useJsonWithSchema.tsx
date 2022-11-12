import { useCallback, useEffect, useState } from 'react';
import { z } from 'zod';
import { invokeWithSchema } from '../lib/invoke';

type JsonWithSchema<Schema, Content> = {
  content: Content;
  schema: Schema;
};

export function useJsonWithSchema<Schema, Content>(
  schemaSchema: z.ZodType<Schema>,
  contentSchema: z.ZodType<Content>,
  file: string
) {
  const [error, setError] = useState<Error | null>(null);
  const [state, setState] = useState<
    JsonWithSchema<Schema, Content> | { content: null; schema: null }
  >({ content: null, schema: null });
  const [needsRefetch, setNeedsRefetch] = useState(true);
  const refetch = useCallback(() => setNeedsRefetch(true), []);
  const fetch = useCallback(
    async (file: string) => {
      const state = await invokeWithSchema(
        z.object({
          content: contentSchema,
          schema: schemaSchema,
        }),
        'open_json_file_with_schema',
        {
          file,
        }
      );
      setState((s) => ({ ...s, ...state }));
    },
    [contentSchema, schemaSchema]
  );

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
