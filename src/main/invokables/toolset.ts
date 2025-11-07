import { app } from 'electron';
import z from 'zod';

let closeConfirmed = false;

export const confirmCloseSchema = z.unknown();

export type ConfirmCloseParams = z.infer<typeof confirmClose>;

export async function confirmClose(): Promise<void> {
  closeConfirmed = true;
  app.quit();
}

export function canClose() {
  return closeConfirmed;
}
