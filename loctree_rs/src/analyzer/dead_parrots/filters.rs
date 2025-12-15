//! Filter functions for dead export detection

use globset::GlobSet;

use crate::types::{ExportSymbol, FileAnalysis};

use super::DeadFilterConfig;

pub(super) fn should_skip_dead_export_check(
    analysis: &FileAnalysis,
    config: &DeadFilterConfig,
    example_globs: Option<&GlobSet>,
) -> bool {
    let path = &analysis.path;
    let lower_path = path.to_ascii_lowercase();

    // Go exports are primarily public API; static import graph is insufficient (FP-heavy)
    // Skip dead-export detection for Go to avoid noise.
    if analysis.language == "go" {
        return true;
    }

    // JSX runtime files - exports consumed by TypeScript/Babel compiler, not by imports
    // Files matching: *jsx-runtime*, jsx-runtime.js, jsx-runtime/index.js, jsx-dev-runtime.js, etc.
    if (analysis.language == "ts" || analysis.language == "js")
        && (lower_path.contains("jsx-runtime")
            || lower_path.contains("jsx_runtime")
            || lower_path.contains("jsx-dev-runtime"))
    {
        return true;
    }

    // Test files and fixtures
    if analysis.is_test && !config.include_tests {
        return true;
    }

    // Flutter generated/plugin registrant files
    if path.ends_with("generated_plugin_registrant.dart")
        || path.contains("/generated_plugin_registrant.dart")
    {
        return true;
    }

    // Test-related directories
    const TEST_DIRS: &[&str] = &[
        "stories",
        "__tests__",
        "__mocks__",
        "__fixtures__",
        "/cypress/",
        "/e2e/",
        "/playwright/",
        "/test/",
        "/tests/",
        "/spec/",
    ];
    if TEST_DIRS.iter().any(|d| lower_path.contains(d)) && !config.include_tests {
        return true;
    }

    // Example/demo/fixture packages (library-mode noise)
    const EXAMPLE_DIRS: &[&str] = &[
        "/examples/",
        "/example/",
        "/samples/",
        "/sample/",
        "/demo/",
        "/demos/",
        "/playground/",
        "/showcase/",
    ];
    if EXAMPLE_DIRS.iter().any(|d| lower_path.contains(d)) {
        return true;
    }
    if config.library_mode {
        if let Some(globs) = example_globs
            && (globs.is_match(path) || globs.is_match(&lower_path))
        {
            return true;
        }
        const LIBRARY_NOISE_DIRS: &[&str] = &[
            "/kitchen-sink/",
            "/kitchensink/",
            "/sandbox/",
            "/sandboxes/",
            "/cookbook/",
            "/gallery/",
            "/examples-",
            "/examples_",
            "/docs/examples/",
            "/documentation/examples/",
        ];
        if LIBRARY_NOISE_DIRS.iter().any(|d| lower_path.contains(d))
            || lower_path.starts_with("examples/")
            || lower_path.starts_with("example/")
            || lower_path.starts_with("demo/")
            || lower_path.contains("/examples/")
        {
            return true;
        }
    }
    // Lowercase check for testfixtures (common in codemods)
    if lower_path.contains("testfixtures") {
        return true;
    }

    // TypeScript declaration files (.d.ts) - only contain type declarations
    if path.ends_with(".d.ts") {
        return true;
    }

    // Dart/Flutter generated artifacts
    if path.ends_with(".g.dart")
        || path.ends_with(".freezed.dart")
        || path.ends_with(".gr.dart")
        || path.ends_with(".pb.dart")
        || path.ends_with(".pbjson.dart")
        || path.ends_with(".pbenum.dart")
        || path.ends_with(".pbserver.dart")
        || path.ends_with(".config.dart")
    {
        return true;
    }

    // Config files loaded dynamically by build tools (Vite, Jest, Cypress, etc.)
    if path.contains(".config.") || path.ends_with(".config.ts") || path.ends_with(".config.js") {
        return true;
    }

    if !config.include_helpers {
        const HELPER_DIRS: &[&str] = &["/scripts/", "/script/", "/tools/", "/docs/"];
        if HELPER_DIRS.iter().any(|d| path.contains(d))
            || path.starts_with("scripts/")
            || path.starts_with("script/")
            || path.starts_with("tools/")
            || path.starts_with("docs/")
        {
            return true;
        }
    }

    // Framework routing/entry point conventions
    // SvelteKit: +page.ts, +layout.ts, +server.ts, +page.server.ts, hooks.*.ts
    // Next.js: page.tsx, layout.tsx, route.ts (in app/ directory)
    const FRAMEWORK_ENTRY_PATTERNS: &[&str] = &[
        "+page.",
        "+layout.",
        "+server.",
        "+error.",
        "/page.tsx",
        "/page.ts",
        "/layout.tsx",
        "/layout.ts",
        "/route.ts",
        "/route.tsx",
        "/error.tsx",
        "/loading.tsx",
        "/not-found.tsx",
        // SvelteKit hooks (auto-loaded by framework)
        "hooks.client.",
        "hooks.server.",
        "/hooks.",
    ];
    if FRAMEWORK_ENTRY_PATTERNS.iter().any(|p| path.contains(p)) {
        return true;
    }

    // Virtual/module-runtime entrypoints (framework-provided consumers)
    const RUNTIME_PATTERNS: &[&str] =
        &["/.svelte-kit/", "/runtime/", "/app/router/", "/app/routes/"];
    if (analysis.language == "ts" || analysis.language == "js")
        && RUNTIME_PATTERNS.iter().any(|p| path.contains(p))
    {
        return true;
    }

    // Library barrels and public API surfaces (avoid flagging public exports)
    if (path.ends_with("/index.ts")
        || path.ends_with("/index.tsx")
        || path.ends_with("/index.js")
        || path.ends_with("/index.mjs")
        || path.ends_with("/index.cjs")
        || path.ends_with("/mod.ts")
        || path.ends_with("/mod.js"))
        && (analysis.language == "ts" || analysis.language == "js")
        && (path.contains("/packages/") || path.contains("/libs/") || path.contains("/library/"))
    {
        return true;
    }

    false
}

/// Check if an export from a Svelte file is likely a component API method.
/// Svelte components expose methods via `export function` that are called via `bind:this`:
///   let modal: MyModal;
///   <MyModal bind:this={modal} />
///   modal.show();  // calling the exported function
///
/// These are NOT imported via ES imports, so they appear as "dead" in static analysis.
pub(super) fn is_jsx_runtime_export(export_name: &str, file_path: &str) -> bool {
    // JSX runtime export names defined by React JSX transform spec
    const JSX_RUNTIME_EXPORTS: &[&str] = &["jsx", "jsxs", "jsxDEV", "jsxsDEV", "Fragment"];

    if !JSX_RUNTIME_EXPORTS.contains(&export_name) {
        return false;
    }

    // Check if file is likely a JSX runtime
    // Patterns: jsx-runtime, jsx_runtime, jsx-dev-runtime (React dev mode)
    let lower_path = file_path.to_ascii_lowercase();
    lower_path.contains("jsx-runtime")
        || lower_path.contains("jsx_runtime")
        || lower_path.contains("jsx-dev-runtime")
}

/// Check if an export is likely a Flow type-only export.
/// Flow files (annotated with @flow) export types that are used via Flow's type system,
/// not via regular import statements. These exports don't appear in static import analysis
pub(super) fn is_flow_type_export(export_symbol: &ExportSymbol, analysis: &FileAnalysis) -> bool {
    if !analysis.is_flow_file {
        return false;
    }

    // Flow type exports: type, interface, opaque type
    // These are type-only and won't appear in runtime imports
    matches!(export_symbol.kind.as_str(), "type" | "interface" | "opaque")
}

/// Check if an export is used in a WeakMap/WeakSet registry pattern.
/// These are common in React and other libraries for storing metadata about objects
/// without causing memory leaks. Exports stored in WeakMap/WeakSet are used dynamically.
pub(super) fn is_weakmap_registry_export(
    _export_symbol: &ExportSymbol,
    analysis: &FileAnalysis,
) -> bool {
    // If a file contains WeakMap/WeakSet usage (detected by AST visitor),
    // conservatively assume all exports might be stored dynamically in the registry.
    // This reduces false positives in React DevTools and similar code where exports
    // are stored in WeakMaps for dynamic lookup.
    analysis.has_weak_collections
}
pub(super) fn is_python_test_export(analysis: &FileAnalysis, exp: &ExportSymbol) -> bool {
    if !analysis.path.ends_with(".py") {
        return false;
    }
    if exp.kind == "class" && exp.name.starts_with("Test") {
        return true;
    }
    if exp.kind == "def" && exp.name.starts_with("test_") {
        return true;
    }
    false
}

pub(super) fn is_python_test_path(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_lowercase();
    lower.contains("/tests/")
        || lower.contains("/test/")
        || lower.ends_with("_test.py")
        || lower.ends_with("_tests.py")
        || lower
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("test_"))
}
