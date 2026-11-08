
declare global {
  interface Window {
    __TAURI_INTERNALS__?: {
      metadata?: {
        currentWindow?: { label?: string };
        currentWebview?: { label?: string };
      };
      plugins?: Record<string, unknown>;
      invoke?: (cmd: string, args?: unknown) => Promise<unknown>;
    };
  }
}

export {};
