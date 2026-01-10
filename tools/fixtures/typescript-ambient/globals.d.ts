// Test fixture: TypeScript ambient declaration file
// These exports should NOT be flagged as dead - they're consumed by TypeScript compiler

declare global {
  interface Window {
    myApp: MyApp;
  }
}

export interface MyApp {
  version: string;
}

export type AppConfig = {
  debug: boolean;
};
