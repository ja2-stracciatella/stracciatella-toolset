import { z } from "zod";
import { invokeWithSchema } from "./invoke";

const soundFileSchema = z.string();

export type SoundFile = z.infer<typeof soundFileSchema>;

export async function readSound(file: string): Promise<SoundFile> {
  let soundFile = await invokeWithSchema(soundFileSchema, "read_sound", {
    file,
  });
  return soundFile;
}
