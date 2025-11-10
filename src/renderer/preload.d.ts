declare global {
  interface Window {
    electronAPI: {
      invoke: (payload: { name: string; input: unknown }) => Promise<unknown>;
      onMainEvent: (
        channel: `event/${string}`,
        callback: (data: unknown) => void,
      ) => void;
    };
  }
}

export {};
