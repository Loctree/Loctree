#[tauri::command]
pub fn used_cmd() {}

#[tauri::command(rename = "explicit_rename")]
pub async fn backend_name() {}

#[tauri::command(rename_all = "camelCase")]
pub fn snake_case_func() {}

// rename_all to PascalCase should expose renameAllPascal
#[tauri::command(rename_all = "PascalCase")]
pub fn rename_all_pascal() {}

#[tauri::command]
pub fn backend_only() {}
