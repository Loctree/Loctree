// Example of module augmentation using declare module
// These exports extend existing modules consumed by TypeScript compiler

declare module 'vue' {
  export interface ComponentCustomProperties {
    $router: Router;
    $route: Route;
  }
}

declare module '@vue/runtime-core' {
  export interface GlobalComponents {
    RouterLink: typeof import('vue-router')['RouterLink'];
    RouterView: typeof import('vue-router')['RouterView'];
  }
}

export {};
