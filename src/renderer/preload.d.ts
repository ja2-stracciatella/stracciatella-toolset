declare global {
  interface Window {
    electronAPI: {
      invoke: (payload: { name: string; input: unknown }) => Promise<unknown>;
      onMainAction: (callback: (data: any) => void) => void;
    };
  }
}

export {};
