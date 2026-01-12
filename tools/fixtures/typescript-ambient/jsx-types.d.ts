// Example of TypeScript ambient declarations for JSX
// These exports are consumed by the TypeScript compiler when compiling JSX

declare global {
  namespace JSX {
    export interface ElementClass {
      $: any;
    }

    export interface ElementAttributesProperty {
      $props: any;
    }

    export interface IntrinsicElements {
      [name: string]: any;
    }

    export interface IntrinsicAttributes {
      key?: string | number | symbol;
    }
  }
}

export {};
