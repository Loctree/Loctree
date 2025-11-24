import { safeInvoke, invokeSnake, invoke } from '@tauri-apps/api/core';

// Commands with matching backend handlers
async function runHappyPath() {
  await safeInvoke('used_cmd');
  await invokeSnake('explicit_rename');
  await safeInvoke('snakeCaseFunc');
  await invoke('RenameAllPascal');
}

// Intentional missing handler to verify coverage
async function callMissing() {
  await safeInvoke('frontend_missing');
}

void runHappyPath();
void callMissing();
