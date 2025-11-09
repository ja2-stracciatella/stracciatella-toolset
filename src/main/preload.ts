import { contextBridge, ipcRenderer } from 'electron';

const invokeChannel = 'invoke';

contextBridge.exposeInMainWorld('electronAPI', {
  invoke: ({ name, input }: { name: string; input: unknown }) =>
    ipcRenderer.invoke(invokeChannel, {
      name,
      input,
    }),
  onMainEvent: (
    channel: `event/${string}`,
    callback: (data: unknown) => void,
  ) => {
    ipcRenderer.removeAllListeners(channel);
    ipcRenderer.on(channel, (_, data) => callback(data));
  },
});
