/// <reference types="vite/client" />

declare const __ENABLE_MOCKS__: boolean;
declare const __APP_VERSION__: string;

interface ImportMetaEnv {
  readonly VITE_API_BASE_URL?: string;
  readonly VITE_API_PROXY_TARGET?: string;
  readonly VITE_ENABLE_MOCKS?: string;
  readonly MODE: string;
  readonly DEV: boolean;
  readonly PROD: boolean;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}