import z from 'zod';
import {
  InvokableDefinition,
  InvokableInput,
  InvokableName,
  InvokableOutput,
} from '../../common/invokables';
import { dialogShowOpenDialogMainJSInvokable } from './dialogs';
import { toolsetCloseWindowMainJsInvokable } from './toolset';
import { IpcMainInvokeEvent } from 'electron';
import rustInterface from '../rust';

export type MainJsInvokable<
  D extends InvokableDefinition<string, string, unknown, unknown>,
> = {
  name: InvokableName<D>;
  func: (input: InvokableInput<D>) => Promise<InvokableOutput<D>>;
  inputSchema: z.ZodSchema<InvokableInput<D>>;
};

const invokables: Array<MainJsInvokable<any>> = [
  toolsetCloseWindowMainJsInvokable,
  dialogShowOpenDialogMainJSInvokable,
];

const invokeSchema = z.object({
  name: z.string(),
  input: z.unknown(),
});

export async function handleInvoke(
  state: ReturnType<typeof rustInterface.newAppState>,
  event: IpcMainInvokeEvent,
  payload: unknown,
) {
  const { name, input } = await invokeSchema.parseAsync(payload);

  // First try JS invokables
  const invokable = invokables.find((invokable) => invokable.name === name);
  if (invokable) {
    const p = await invokable.inputSchema.parseAsync(input);
    const result = await invokable.func(p);
    return result;
  }

  // Otherwise try Rust invokables
  const result = await rustInterface.invoke(
    state,
    JSON.stringify({ func: name, params: input }),
  );
  return JSON.parse(result);
}
