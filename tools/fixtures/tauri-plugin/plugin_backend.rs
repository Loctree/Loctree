// Tauri plugin example - window plugin
#![plugin(identifier = "window")]

use tauri::{command, Runtime};

// Plugin command with root = "crate" - should be exposed as "plugin:window|set_title"
#[command(root = "crate")]
pub async fn set_title<R: Runtime>(title: String) -> Result<(), String> {
    println!("Setting title: {}", title);
    Ok(())
}

// Plugin command with rename - should be exposed as "plugin:window|customIcon"
#[command(root = "crate", rename = "customIcon")]
pub fn set_icon<R: Runtime>(icon_path: String) -> Result<(), String> {
    println!("Setting icon: {}", icon_path);
    Ok(())
}

// Plugin command with rename_all - should be exposed as "plugin:window|maximizeWindow"
#[command(root = "crate", rename_all = "camelCase")]
pub fn maximize_window<R: Runtime>() -> Result<(), String> {
    println!("Maximizing window");
    Ok(())
}

// Regular command without root = "crate" - should NOT be plugin-namespaced
#[command]
pub fn internal_command() -> String {
    "Internal".to_string()
}

// Another plugin with different identifier
#![plugin(identifier = "dialog")]

// This should be exposed as "plugin:dialog|show_message"
#[command(root = "crate")]
pub async fn show_message(message: String) -> Result<(), String> {
    println!("Showing message: {}", message);
    Ok(())
}
