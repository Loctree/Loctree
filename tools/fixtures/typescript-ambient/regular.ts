// Test fixture: Regular TypeScript file with actual dead exports
// These SHOULD be flagged as dead code

export function unusedFunction(): void {
  console.log("I am never imported");
}

export const unusedConstant = 42;

export interface UnusedInterface {
  name: string;
}

export type UnusedType = string | number;
