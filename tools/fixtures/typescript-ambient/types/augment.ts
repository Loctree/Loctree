// Test fixture: Type augmentation file in /types/ directory
// Should be detected as ambient by path heuristic

declare module 'vue' {
  interface ComponentCustomProperties {
    $myPlugin: MyPluginAPI;
  }
}

export interface MyPluginAPI {
  doSomething(): void;
}

export type PluginOptions = {
  enabled: boolean;
};
