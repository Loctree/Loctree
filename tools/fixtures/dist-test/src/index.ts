export function usedFunction() {
  return "I am used!";
}

export function deadFunction() {
  return "I am never imported";
}

export const USED_CONST = 42;
export const DEAD_CONST = "never used";
