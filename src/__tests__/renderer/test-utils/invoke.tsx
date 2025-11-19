import { isDeepEqual } from 'remeda';
import {
  AnyInvokableName,
  InvokableFromName,
  InvokableInput,
  InvokableOutput,
} from 'src/common/invokables';
import { MockedFunction } from 'vitest';

type MockedInvoke = MockedFunction<typeof window.electronAPI.invoke>;

type InvokeResult =
  | {
      type: 'reject';
      error: Error;
    }
  | {
      type: 'resolve';
      output: unknown;
    };

export class InvokeMock {
  private invoke: MockedInvoke;
  private expectedCalls: Array<{
    name: AnyInvokableName;
    input: unknown;
    result: InvokeResult;
  }>;

  constructor(invoke: typeof window.electronAPI.invoke) {
    this.invoke = invoke as MockedInvoke;
    this.invoke.mockImplementation(async (payload) =>
      this.mockImplementation(payload),
    );
    this.expectedCalls = [];
  }

  private mockImplementation(
    payload: Parameters<typeof window.electronAPI.invoke>[0],
  ) {
    for (const call of this.expectedCalls) {
      if (
        isDeepEqual(payload, {
          name: call.name,
          input: call.input,
        })
      ) {
        if (call.result.type === 'resolve') {
          return call.result.output;
        }
        throw call.result.error;
      }
    }
    throw new Error(
      `Unexpected invoke call to ${payload.name} with input ${JSON.stringify(payload.input)}`,
    );
  }

  resolve<T extends AnyInvokableName>(
    name: T,
    input: InvokableInput<InvokableFromName<T>>,
    output: InvokableOutput<InvokableFromName<T>>,
  ) {
    this.expectedCalls.push({
      name,
      input,
      result: {
        type: 'resolve',
        output,
      },
    });
  }

  reject<T extends AnyInvokableName>(
    name: T,
    input: InvokableInput<InvokableFromName<T>>,
    error: Error,
  ) {
    this.expectedCalls.push({
      name,
      input,
      result: {
        type: 'reject',
        error,
      },
    });
  }
}

export function getInvokeMock() {
  return new InvokeMock(window.electronAPI.invoke);
}
