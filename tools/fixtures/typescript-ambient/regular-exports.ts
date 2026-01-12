// Regular TypeScript exports (NOT ambient) - these SHOULD be flagged if unused
// This file should NOT have has_ambient_declarations = true

export interface RegularInterface {
  id: string;
  name: string;
}

export function regularFunction() {
  return "hello";
}

export const regularConst = 42;
