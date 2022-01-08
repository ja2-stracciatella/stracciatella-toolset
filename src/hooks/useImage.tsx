import { useCallback, useEffect, useState } from "react";
import { z } from "zod";
import { invokeWithSchema } from "../lib/invoke";

const ImageFile = z.string();

type ImageFile = z.infer<typeof ImageFile>;

export function useImageFile(file: string) {
  const [error, setError] = useState<Error | null>(null);
  const [state, setState] = useState<ImageFile | null>(null);
  const fetch = useCallback(async (file: string) => {
    const state = await invokeWithSchema(ImageFile, "read_image_file", {
      file,
    });
    setState(state);
  }, []);

  useEffect(() => {
    fetch(file).catch((e: any) =>
      setError(new Error(`error fetching image: ${e}`))
    );
  }, [fetch, file]);

  return { error, data: state };
}
