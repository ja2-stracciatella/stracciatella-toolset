import { contextBridge, ipcRenderer } from 'electron';

const invokeChannel = 'invoke';
const actionsChannel = 'actions';

contextBridge.exposeInMainWorld('electronAPI', {
  invoke: ({ func, params }: { func: string; params: unknown }) =>
    ipcRenderer.invoke(invokeChannel, {
      func,
      params,
    }),
  onMainAction: (callback: (data: any) => void) => {
    ipcRenderer.removeAllListeners(actionsChannel);
    ipcRenderer.on(actionsChannel, (_, data) => callback(data));
  },
});
