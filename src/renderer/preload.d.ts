declare global {
  interface Window {
    electronAPI: {
      invoke: (payload: {
        func: string;
        params: Record<string, unknown> | null;
      }) => Promise<unknown>;
    };
  }
}

export {};
