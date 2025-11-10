import { app } from 'electron';
import z from 'zod';
import { MainJsInvokable } from '.';
import { toolsetCloseWindowInvokableDefinition } from '../../common/invokables/toolset';

let windowCloseConfirmed = false;

export const toolsetCloseWindowMainJsInvokable: MainJsInvokable<
  typeof toolsetCloseWindowInvokableDefinition
> = {
  name: 'toolset/closeWindow',
  inputSchema: z.null(),
  func: async () => {
    windowCloseConfirmed = true;
    app.quit();
  },
};

export function toolsetWindowCloseConfirmed() {
  return windowCloseConfirmed;
}
