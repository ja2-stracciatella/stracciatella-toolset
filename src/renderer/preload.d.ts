declare global {
  interface Window {
    electronAPI: {
      invoke: (payload: {
        func: string;
        params: Record<string, unknown> | null;
      }) => Promise<unknown>;
      onMainAction: (callback: (data: any) => void) => void;
    };
  }
}

export {};
