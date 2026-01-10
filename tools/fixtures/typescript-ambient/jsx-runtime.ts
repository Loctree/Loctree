// Test fixture: JSX runtime file with ambient declarations
// Vue-style jsx-runtime pattern - exports consumed by compiler, not imports

declare global {
  namespace JSX {
    interface Element {}
    interface IntrinsicElements {
      div: any;
      span: any;
    }
  }
}

export interface VNode {
  type: string;
  props: Record<string, any>;
}

export type ComponentType = string | Function;
