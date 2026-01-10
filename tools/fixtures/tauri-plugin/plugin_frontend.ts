import { invoke } from '@tauri-apps/api/core';

// Plugin invoke patterns - these should match the backend handlers
async function useWindowPlugin() {
  // Should match: #[command(root = "crate")] fn set_title
  await invoke('plugin:window|set_title', { title: 'New Title' });

  // Should match: #[command(root = "crate", rename = "customIcon")] fn set_icon
  await invoke('plugin:window|customIcon', { iconPath: '/path/to/icon.png' });

  // Should match: #[command(root = "crate", rename_all = "camelCase")] fn maximize_window
  await invoke('plugin:window|maximizeWindow');

  // Should match: #[command] fn internal_command (NOT plugin-namespaced)
  await invoke('internal_command');
}

async function useDialogPlugin() {
  // Should match: #[command(root = "crate")] fn show_message in dialog plugin
  await invoke('plugin:dialog|show_message', { message: 'Hello!' });
}

// Missing handler - should be reported as coverage gap
async function callMissing() {
  await invoke('plugin:window|missing_handler');
}

void useWindowPlugin();
void useDialogPlugin();
void callMissing();
