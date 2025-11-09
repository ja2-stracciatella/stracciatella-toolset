import { contextBridge, ipcRenderer } from 'electron';

const invokeChannel = 'invoke';
const actionsChannel = 'actions';

contextBridge.exposeInMainWorld('electronAPI', {
  invoke: ({ name, input }: { name: string; input: unknown }) =>
    ipcRenderer.invoke(invokeChannel, {
      name,
      input,
    }),
  onMainAction: (callback: (data: any) => void) => {
    ipcRenderer.removeAllListeners(actionsChannel);
    ipcRenderer.on(actionsChannel, (_, data) => callback(data));
  },
});
