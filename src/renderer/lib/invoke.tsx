import {
  AnyInvokableName,
  getInvokableDefinitionByName,
  InvokableFromName,
  InvokableInput,
  InvokableOutput,
} from '../../common/invokables';
import { z } from 'zod';

export async function invoke<
  Name extends AnyInvokableName,
  D extends InvokableFromName<Name>,
>(name: Name, input: InvokableInput<D>): Promise<InvokableOutput<D>> {
  try {
    const res = await window.electronAPI.invoke({ name, input });
    const definition = getInvokableDefinitionByName(name);
    const schema = definition.outputSchema;
    const output = await schema.parseAsync(res);
    return output as InvokableOutput<D>;
  } catch (error) {
    if (error instanceof z.ZodError) {
      throw new Error(
        `failed to parse response from invoke ${name} with input ${JSON.stringify(input)}: ${error.message}`,
      );
    }
    throw error;
  }
}
