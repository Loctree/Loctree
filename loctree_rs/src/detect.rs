//! Stack detection module for auto-configuring loctree based on project files.
//!
//! Detects project type by checking for common configuration files:
//! - Cargo.toml → Rust project
//! - tsconfig.json / package.json → TypeScript/JavaScript
//! - pyproject.toml / setup.py → Python
//! - src-tauri/ → Tauri preset
//! - vite.config.* → Vite project

use std::collections::HashSet;
use std::path::Path;

/// Result of stack detection
#[derive(Clone, Debug, Default)]
pub struct DetectedStack {
    /// File extensions to scan
    pub extensions: HashSet<String>,
    /// Patterns to ignore
    pub ignores: Vec<String>,
    /// Detected preset name (e.g., "tauri")
    pub preset_name: Option<String>,
    /// Human-readable description of detected stack
    pub description: String,
}

impl DetectedStack {
    /// Check if anything was detected
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty() && self.preset_name.is_none()
    }
}

/// Detect project stack from root directory
pub fn detect_stack(root: &Path) -> DetectedStack {
    let mut result = DetectedStack::default();
    let mut detected_parts: Vec<&str> = Vec::new();

    // Check for Cargo.toml -> Rust project
    // Also check direct subdirectories for monorepo-style layouts (e.g., codex-rs/Cargo.toml)
    let has_cargo_toml = root.join("Cargo.toml").exists() || has_cargo_in_subdir(root);
    if has_cargo_toml {
        result.extensions.insert("rs".to_string());
        result.ignores.push("target".to_string());
        detected_parts.push("Rust");
    }

    // Check for Dart/Flutter (pubspec.yaml)
    if root.join("pubspec.yaml").exists() {
        result.extensions.insert("dart".to_string());
        result.ignores.push(".dart_tool".to_string());
        result.ignores.push("build".to_string());
        result.ignores.push(".packages".to_string());
        detected_parts.push("Dart/Flutter");
    }

    // Check for Go projects (go.mod or .go files)
    if root.join("go.mod").exists()
        || root
            .read_dir()
            .ok()
            .map(|entries| {
                entries.flatten().any(|entry| {
                    entry
                        .path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("go"))
                })
            })
            .unwrap_or(false)
    {
        result.extensions.insert("go".to_string());
        result.ignores.push("vendor".to_string());
        detected_parts.push("Go");
    }

    // Check for src-tauri/ -> Tauri preset (must check before generic TS)
    if root.join("src-tauri").exists() {
        result.preset_name = Some("tauri".to_string());
        result.extensions.insert("rs".to_string());
        result.extensions.insert("ts".to_string());
        result.extensions.insert("tsx".to_string());
        result.extensions.insert("js".to_string());
        result.extensions.insert("jsx".to_string());
        result.extensions.insert("css".to_string());
        result.ignores.push("target".to_string());
        result.ignores.push("node_modules".to_string());
        result.ignores.push("dist".to_string());
        detected_parts.push("Tauri");
    }

    // Check for package.json + tsconfig.json -> TypeScript
    let has_tsconfig = root.join("tsconfig.json").exists();
    let has_package_json = root.join("package.json").exists();

    if has_tsconfig || has_package_json {
        result.extensions.insert("ts".to_string());
        result.extensions.insert("tsx".to_string());
        result.extensions.insert("js".to_string());
        result.extensions.insert("jsx".to_string());
        result.extensions.insert("mjs".to_string());
        result.extensions.insert("cjs".to_string());

        if !result.ignores.contains(&"node_modules".to_string()) {
            result.ignores.push("node_modules".to_string());
        }
        if !result.ignores.contains(&"dist".to_string()) {
            result.ignores.push("dist".to_string());
        }

        if has_tsconfig && !detected_parts.contains(&"Tauri") {
            detected_parts.push("TypeScript");
        } else if has_package_json && !detected_parts.contains(&"Tauri") {
            detected_parts.push("JavaScript");
        }
    }

    // Check for vite.config.* -> Vite project (add build to ignores)
    let vite_extensions = ["js", "ts", "mjs"];
    for ext in vite_extensions {
        if root.join(format!("vite.config.{}", ext)).exists() {
            if !result.ignores.contains(&"dist".to_string()) {
                result.ignores.push("dist".to_string());
            }
            result.ignores.push("build".to_string());
            if !detected_parts.contains(&"Vite") {
                detected_parts.push("Vite");
            }
            break;
        }
    }

    // Check for svelte.config.* -> SvelteKit project
    let svelte_exists =
        root.join("svelte.config.js").exists() || root.join("svelte.config.ts").exists();
    // Also check apps/* and packages/* for monorepos
    let mut svelte_in_subdir = false;
    for subdir in ["apps", "packages"] {
        let dir = root.join(subdir);
        if dir.is_dir()
            && let Ok(entries) = std::fs::read_dir(&dir)
        {
            for e in entries.flatten() {
                let path = e.path();
                if path.is_dir()
                    && (path.join("svelte.config.js").exists()
                        || path.join("svelte.config.ts").exists())
                {
                    svelte_in_subdir = true;
                    break;
                }
            }
        }
        if svelte_in_subdir {
            break;
        }
    }
    if svelte_exists || svelte_in_subdir {
        result.extensions.insert("svelte".to_string());
        result.ignores.push(".svelte-kit".to_string());
        if !detected_parts.contains(&"SvelteKit") {
            detected_parts.push("SvelteKit");
        }
    }

    // Check for Vue projects (vue.config.*, vite.config.* with Vue, or .vue files in src/)
    let vue_config_exists =
        root.join("vue.config.js").exists() || root.join("vue.config.ts").exists();
    let has_vue_files = root.join("src").exists()
        && std::fs::read_dir(root.join("src"))
            .map(|entries| {
                entries.flatten().any(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("vue"))
                })
            })
            .unwrap_or(false);
    // Also check packages/* for monorepos (common in Vue ecosystem)
    let mut vue_in_subdir = false;
    for subdir in ["packages", "packages-private", "apps"] {
        let dir = root.join(subdir);
        if dir.is_dir()
            && let Ok(entries) = std::fs::read_dir(&dir)
        {
            for e in entries.flatten() {
                let path = e.path();
                if path.is_dir() {
                    // Check for .vue files in this package's src/
                    let pkg_src = path.join("src");
                    if pkg_src.is_dir()
                        && std::fs::read_dir(&pkg_src)
                            .map(|entries| {
                                entries.flatten().any(|e| {
                                    e.path()
                                        .extension()
                                        .is_some_and(|ext| ext.eq_ignore_ascii_case("vue"))
                                })
                            })
                            .unwrap_or(false)
                    {
                        vue_in_subdir = true;
                        break;
                    }
                }
            }
        }
        if vue_in_subdir {
            break;
        }
    }
    if vue_config_exists || has_vue_files || vue_in_subdir {
        result.extensions.insert("vue".to_string());
        if !detected_parts.contains(&"Vue") {
            detected_parts.push("Vue");
        }
    }

    // Check for pyproject.toml / setup.py -> Python
    if root.join("pyproject.toml").exists() || root.join("setup.py").exists() {
        result.extensions.insert("py".to_string());
        result.ignores.push(".venv".to_string());
        result.ignores.push("venv".to_string());
        result.ignores.push("__pycache__".to_string());
        result.ignores.push(".pytest_cache".to_string());
        result.ignores.push(".mypy_cache".to_string());
        result.ignores.push(".ruff_cache".to_string());
        result.ignores.push("*.egg-info".to_string());
        result.ignores.push(".eggs".to_string());
        result.ignores.push("dist".to_string());
        result.ignores.push("build".to_string());
        result.ignores.push(".tox".to_string());
        // Common ML/data caches that often contain symlinks
        result.ignores.push(".fastembed_cache".to_string());
        result.ignores.push(".cache".to_string());
        result.ignores.push("logs".to_string());
        result.ignores.push("packaging".to_string());
        // uv specific
        result.ignores.push(".uv".to_string());
        detected_parts.push("Python");
    }

    // Check for CSS files in common locations
    if root.join("src").exists() || root.join("styles").exists() {
        // Only add CSS if we have a JS/TS project
        if result.extensions.contains("ts") || result.extensions.contains("js") {
            result.extensions.insert("css".to_string());
        }
    }

    // Common dev/test directories to ignore (reduces noise in dead export reports)
    if !result.ignores.is_empty() {
        // These often contain test fixtures, mocks, or platform-specific code
        for dir in &[
            "e2e",
            "scripts",
            "mobile",
            "__mocks__",
            "__fixtures__",
            "fixtures",
        ] {
            if !result.ignores.contains(&dir.to_string()) {
                result.ignores.push(dir.to_string());
            }
        }
    }

    // Build description
    if !detected_parts.is_empty() {
        result.description = format!("Detected: {}", detected_parts.join(" + "));
    }

    result
}

/// Check if any direct subdirectory contains a Cargo.toml (monorepo detection)
fn has_cargo_in_subdir(root: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip common non-Rust directories
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.')
                || name_str == "node_modules"
                || name_str == "dist"
                || name_str == "build"
            {
                continue;
            }

            if path.join("Cargo.toml").exists() {
                return true;
            }
        }
    }
    false
}

/// Apply detected stack to parsed args if no explicit config provided
pub fn apply_detected_stack(
    root: &Path,
    extensions: &mut Option<HashSet<String>>,
    ignore_patterns: &mut Vec<String>,
    tauri_preset: &mut bool,
    verbose: bool,
) {
    // Skip if user already specified extensions
    if extensions.is_some() {
        return;
    }

    // Skip if tauri preset is already set
    if *tauri_preset {
        return;
    }

    let detected = detect_stack(root);

    if detected.is_empty() {
        return;
    }

    if verbose && !detected.description.is_empty() {
        eprintln!("[loctree][detect] {}", detected.description);
    }

    // Apply detected extensions
    if !detected.extensions.is_empty() {
        *extensions = Some(detected.extensions);
    }

    // Apply ignores only if user didn't specify any
    if ignore_patterns.is_empty() {
        *ignore_patterns = detected.ignores;
    }

    // Apply preset
    if let Some(preset) = detected.preset_name
        && preset == "tauri"
    {
        *tauri_preset = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_rust_project() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .expect("write Cargo.toml");

        let detected = detect_stack(tmp.path());

        assert!(detected.extensions.contains("rs"));
        assert!(detected.ignores.contains(&"target".to_string()));
    }

    #[test]
    fn test_detect_typescript_project() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::write(tmp.path().join("tsconfig.json"), "{}").expect("write tsconfig.json");

        let detected = detect_stack(tmp.path());

        assert!(detected.extensions.contains("ts"));
        assert!(detected.extensions.contains("tsx"));
        assert!(detected.ignores.contains(&"node_modules".to_string()));
    }

    #[test]
    fn test_detect_tauri_project() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::create_dir(tmp.path().join("src-tauri")).expect("create src-tauri dir");
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .expect("write Cargo.toml");
        std::fs::write(tmp.path().join("package.json"), "{}").expect("write package.json");

        let detected = detect_stack(tmp.path());

        assert_eq!(detected.preset_name, Some("tauri".to_string()));
        assert!(detected.extensions.contains("rs"));
        assert!(detected.extensions.contains("ts"));
        assert!(detected.extensions.contains("tsx"));
    }

    #[test]
    fn test_detect_python_project() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::write(
            tmp.path().join("pyproject.toml"),
            "[project]\nname = \"test\"",
        )
        .expect("write pyproject.toml");

        let detected = detect_stack(tmp.path());

        assert!(detected.extensions.contains("py"));
        assert!(detected.ignores.contains(&".venv".to_string()));
        assert!(detected.ignores.contains(&"__pycache__".to_string()));
    }

    #[test]
    fn test_detect_empty_project() {
        let tmp = TempDir::new().expect("create temp dir");

        let detected = detect_stack(tmp.path());

        assert!(detected.is_empty());
    }

    #[test]
    fn test_detect_mixed_project() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::write(tmp.path().join("Cargo.toml"), "").expect("write Cargo.toml");
        std::fs::write(tmp.path().join("pyproject.toml"), "").expect("write pyproject.toml");

        let detected = detect_stack(tmp.path());

        assert!(detected.extensions.contains("rs"));
        assert!(detected.extensions.contains("py"));
    }

    #[test]
    fn test_detect_vite_project() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::write(tmp.path().join("vite.config.ts"), "export default {}").expect("write");
        std::fs::write(tmp.path().join("package.json"), "{}").expect("write");

        let detected = detect_stack(tmp.path());

        assert!(detected.ignores.contains(&"build".to_string()));
        assert!(detected.description.contains("Vite"));
    }

    #[test]
    fn test_detect_javascript_only() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::write(tmp.path().join("package.json"), "{}").expect("write package.json");

        let detected = detect_stack(tmp.path());

        assert!(detected.extensions.contains("js"));
        assert!(detected.description.contains("JavaScript"));
    }

    #[test]
    fn test_detect_with_src_adds_css() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::write(tmp.path().join("package.json"), "{}").expect("write package.json");
        std::fs::create_dir(tmp.path().join("src")).expect("create src");

        let detected = detect_stack(tmp.path());

        assert!(detected.extensions.contains("css"));
    }

    #[test]
    fn test_apply_detected_stack_skips_if_extensions_set() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::write(tmp.path().join("Cargo.toml"), "").expect("write");

        let mut extensions = Some(HashSet::from(["py".to_string()]));
        let mut ignores = Vec::new();
        let mut tauri = false;

        apply_detected_stack(tmp.path(), &mut extensions, &mut ignores, &mut tauri, false);

        // Should not have changed - user specified extensions
        assert!(extensions.as_ref().unwrap().contains("py"));
        assert!(!extensions.as_ref().unwrap().contains("rs"));
    }

    #[test]
    fn test_apply_detected_stack_skips_if_tauri_preset() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::write(tmp.path().join("Cargo.toml"), "").expect("write");

        let mut extensions: Option<HashSet<String>> = None;
        let mut ignores = Vec::new();
        let mut tauri = true; // Already set

        apply_detected_stack(tmp.path(), &mut extensions, &mut ignores, &mut tauri, false);

        // Should not have changed - tauri already set
        assert!(extensions.is_none());
    }

    #[test]
    fn test_apply_detected_stack_applies_tauri() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::create_dir(tmp.path().join("src-tauri")).expect("mkdir");
        std::fs::write(tmp.path().join("package.json"), "{}").expect("write");

        let mut extensions: Option<HashSet<String>> = None;
        let mut ignores = Vec::new();
        let mut tauri = false;

        apply_detected_stack(tmp.path(), &mut extensions, &mut ignores, &mut tauri, false);

        assert!(tauri);
        assert!(extensions.is_some());
        assert!(extensions.as_ref().unwrap().contains("ts"));
    }

    #[test]
    fn test_apply_detected_stack_preserves_user_ignores() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::write(tmp.path().join("Cargo.toml"), "").expect("write");

        let mut extensions: Option<HashSet<String>> = None;
        let mut ignores = vec!["custom_ignore".to_string()];
        let mut tauri = false;

        apply_detected_stack(tmp.path(), &mut extensions, &mut ignores, &mut tauri, false);

        // Should NOT have applied detected ignores since user specified their own
        assert_eq!(ignores, vec!["custom_ignore".to_string()]);
    }

    #[test]
    fn test_apply_detected_stack_verbose() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::write(tmp.path().join("Cargo.toml"), "").expect("write");

        let mut extensions: Option<HashSet<String>> = None;
        let mut ignores = Vec::new();
        let mut tauri = false;

        // Should not panic with verbose=true
        apply_detected_stack(tmp.path(), &mut extensions, &mut ignores, &mut tauri, true);
    }

    #[test]
    fn test_detected_stack_is_empty() {
        let empty = DetectedStack::default();
        assert!(empty.is_empty());

        let with_ext = DetectedStack {
            extensions: HashSet::from(["rs".to_string()]),
            ..Default::default()
        };
        assert!(!with_ext.is_empty());

        let with_preset = DetectedStack {
            preset_name: Some("tauri".to_string()),
            ..Default::default()
        };
        assert!(!with_preset.is_empty());
    }

    #[test]
    fn test_detect_rust_in_subdirectory() {
        // Monorepo layout: package.json at root, Cargo.toml in subdirectory
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::write(tmp.path().join("package.json"), "{}").expect("write package.json");
        std::fs::create_dir(tmp.path().join("codex-rs")).expect("mkdir codex-rs");
        std::fs::write(
            tmp.path().join("codex-rs").join("Cargo.toml"),
            "[package]\nname = \"test\"",
        )
        .expect("write Cargo.toml");

        let detected = detect_stack(tmp.path());

        // Should detect both JavaScript and Rust
        assert!(detected.extensions.contains("rs"));
        assert!(detected.extensions.contains("js"));
        assert!(detected.description.contains("JavaScript"));
        assert!(detected.description.contains("Rust"));
    }

    #[test]
    fn test_has_cargo_in_subdir() {
        let tmp = TempDir::new().expect("create temp dir");

        // No subdirs yet
        assert!(!has_cargo_in_subdir(tmp.path()));

        // Add a subdir without Cargo.toml
        std::fs::create_dir(tmp.path().join("src")).expect("mkdir");
        assert!(!has_cargo_in_subdir(tmp.path()));

        // Add a subdir with Cargo.toml
        std::fs::create_dir(tmp.path().join("backend")).expect("mkdir");
        std::fs::write(tmp.path().join("backend").join("Cargo.toml"), "").expect("write");
        assert!(has_cargo_in_subdir(tmp.path()));
    }

    #[test]
    fn test_has_cargo_in_subdir_skips_hidden() {
        let tmp = TempDir::new().expect("create temp dir");
        std::fs::create_dir(tmp.path().join(".hidden")).expect("mkdir");
        std::fs::write(tmp.path().join(".hidden").join("Cargo.toml"), "").expect("write");

        // Should not find Cargo.toml in hidden directories
        assert!(!has_cargo_in_subdir(tmp.path()));
    }
}
