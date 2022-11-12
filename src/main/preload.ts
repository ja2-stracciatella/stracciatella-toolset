import { contextBridge, ipcRenderer } from 'electron';

const invokeChannel = 'invoke';

contextBridge.exposeInMainWorld('electronAPI', {
  invoke: ({ func, params }: { func: string; params: unknown }) =>
    ipcRenderer.invoke(invokeChannel, {
      func,
      params,
    }),
});
