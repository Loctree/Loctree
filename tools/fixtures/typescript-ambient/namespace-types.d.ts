// Example of ambient namespace declaration
// These exports are consumed by TypeScript compiler for type checking

declare namespace App {
  export interface Locals {
    user: {
      id: string;
      name: string;
    };
  }

  export interface PageData {
    title: string;
    content: string;
  }

  export interface Platform {
    env: Record<string, string>;
  }
}

export {};
