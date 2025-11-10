import { dialog } from 'electron';
import { dialogShowOpenDialogInvokableDefinition } from '../../common/invokables/dialogs';
import { MainJsInvokable } from '.';

export const dialogShowOpenDialogMainJSInvokable: MainJsInvokable<
  typeof dialogShowOpenDialogInvokableDefinition
> = {
  name: 'dialog/showOpenDialog',
  inputSchema: dialogShowOpenDialogInvokableDefinition.inputSchema,
  func: async (input) => {
    const result = await dialog.showOpenDialog({
      title: input.title,
      defaultPath: input.defaultPath,
      properties: [input.type === 'open-file' ? 'openFile' : 'openDirectory'],
    });
    if (!result.canceled && result.filePaths[0]) {
      return { path: result.filePaths[0] };
    }
    return { path: null };
  },
};
