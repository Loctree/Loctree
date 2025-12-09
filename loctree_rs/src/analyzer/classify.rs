use std::path::Path;

pub fn is_dev_file(path: &str) -> bool {
    path.contains("__tests__")
        || path.contains("stories")
        || path.contains(".stories.")
        || path.contains("story.")
        || path.contains("fixture")
        || path.contains("fixtures")
}

pub fn detect_language(ext: &str) -> String {
    match ext {
        "ts" | "tsx" => "ts".to_string(),
        "js" | "jsx" | "mjs" | "cjs" => "js".to_string(),
        "rs" => "rs".to_string(),
        "py" => "py".to_string(),
        "dart" => "dart".to_string(),
        "css" => "css".to_string(),
        other => other.to_string(),
    }
}

fn is_test_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("__tests__")
        || lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.ends_with("_test.rs")
        || lower.ends_with("_tests.rs")
        || lower.ends_with("_test.go")
        || lower.ends_with("_test.dart")
}

fn is_story_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("stories") || lower.contains(".story.") || lower.contains(".stories.")
}

fn is_generated_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("generated")
        || lower.contains("codegen")
        || lower.contains("/gen/")
        || lower.ends_with(".gen.ts")
        || lower.ends_with(".gen.tsx")
        || lower.ends_with(".gen.rs")
        || lower.ends_with(".g.rs")
        || lower.ends_with(".g.dart")
        || lower.ends_with(".freezed.dart")
        || lower.ends_with(".gr.dart")
        || lower.ends_with(".pb.dart")
        || lower.ends_with(".pbjson.dart")
        || lower.ends_with(".pbenum.dart")
        || lower.ends_with(".pbserver.dart")
        || lower.ends_with(".config.dart")
}

pub fn file_kind(path: &str) -> (String, bool, bool) {
    let generated = is_generated_path(path);
    let test = is_test_path(path);
    let story = is_story_path(path);
    let lower = path.to_ascii_lowercase();
    let config = lower.contains("config/")
        || lower.contains("/config/")
        || lower.ends_with("config.ts")
        || lower.ends_with("config.tsx")
        || lower.ends_with("config.js")
        || lower.ends_with("config.rs")
        || lower.ends_with(".config.ts")
        || lower.ends_with(".config.js")
        || lower.ends_with(".config.json");

    let kind = if generated {
        "generated"
    } else if test {
        "test"
    } else if story {
        "story"
    } else if config {
        "config"
    } else {
        "code"
    };

    (kind.to_string(), test, generated)
}

pub fn language_from_path(path: &str) -> String {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();
    detect_language(&ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_dev_and_non_dev_files() {
        assert!(is_dev_file("features/__tests__/thing.ts"));
        assert!(is_dev_file("components/Button.stories.tsx"));
        assert!(is_dev_file("fixtures/foo.rs"));
        assert!(!is_dev_file("src/app.tsx"));
    }

    #[test]
    fn classifies_file_kinds_and_flags() {
        let (kind, test, is_generated) = file_kind("src/generated/foo.gen.ts");
        assert_eq!(kind, "generated");
        assert!(!test);
        assert!(is_generated);

        let (kind, test, is_generated) = file_kind("src/components/Button.story.tsx");
        assert_eq!(kind, "story");
        assert!(!is_generated);
        assert!(!test);

        let (kind, test, _) = file_kind("src/__tests__/foo.test.ts");
        assert_eq!(kind, "test");
        assert!(test);

        let (kind, _, _) = file_kind("config/vite.config.ts");
        assert_eq!(kind, "config");

        let (kind, _, _) = file_kind("src/features/app.tsx");
        assert_eq!(kind, "code");
    }

    #[test]
    fn detects_language_from_path() {
        assert_eq!(language_from_path("foo/bar.tsx"), "ts");
        assert_eq!(language_from_path("foo/bar.rs"), "rs");
        assert_eq!(language_from_path("foo/bar.py"), "py");
        assert_eq!(language_from_path("foo/bar.css"), "css");
        assert_eq!(language_from_path("foo/bar.unknown"), "unknown");
    }

    #[test]
    fn detect_language_all_extensions() {
        assert_eq!(detect_language("ts"), "ts");
        assert_eq!(detect_language("tsx"), "ts");
        assert_eq!(detect_language("js"), "js");
        assert_eq!(detect_language("jsx"), "js");
        assert_eq!(detect_language("mjs"), "js");
        assert_eq!(detect_language("cjs"), "js");
        assert_eq!(detect_language("rs"), "rs");
        assert_eq!(detect_language("py"), "py");
        assert_eq!(detect_language("go"), "go");
        assert_eq!(detect_language("css"), "css");
        assert_eq!(detect_language("html"), "html");
    }

    #[test]
    fn is_dev_file_variations() {
        // __tests__ variations
        assert!(is_dev_file("src/__tests__/Button.test.ts"));
        assert!(is_dev_file("__tests__/unit/helper.ts"));

        // stories variations
        assert!(is_dev_file("components/stories/Button.tsx"));
        assert!(is_dev_file("Button.stories.tsx"));
        assert!(is_dev_file("Button.story.tsx"));

        // fixtures
        assert!(is_dev_file("test/fixtures/data.json"));
        assert!(is_dev_file("fixture/mock.ts"));

        // regular files should not match
        assert!(!is_dev_file("src/components/Button.tsx"));
        assert!(!is_dev_file("lib/utils.ts"));
        assert!(!is_dev_file("src/store/index.ts"));
    }

    #[test]
    fn is_test_path_variations() {
        assert!(is_test_path("src/__tests__/foo.ts"));
        assert!(is_test_path("src/Button.test.tsx"));
        assert!(is_test_path("utils.spec.ts"));
        assert!(is_test_path("lib_test.rs"));
        assert!(is_test_path("module_tests.rs"));
        assert!(is_test_path("SRC/__TESTS__/FOO.TS")); // case insensitive

        assert!(!is_test_path("src/Button.tsx"));
        assert!(!is_test_path("testing.ts")); // 'testing' not a test marker
    }

    #[test]
    fn is_story_path_variations() {
        assert!(is_story_path("Button.stories.tsx"));
        assert!(is_story_path("Button.story.tsx"));
        assert!(is_story_path("components/stories/Button.tsx"));
        assert!(is_story_path("BUTTON.STORIES.TSX")); // case insensitive

        assert!(!is_story_path("src/Button.tsx"));
        assert!(!is_story_path("history.ts")); // 'history' doesn't match
    }

    #[test]
    fn is_generated_path_variations() {
        assert!(is_generated_path("src/generated/types.ts"));
        assert!(is_generated_path("lib/codegen/schema.ts"));
        assert!(is_generated_path("out/gen/api.ts"));
        assert!(is_generated_path("types.gen.ts"));
        assert!(is_generated_path("api.gen.tsx"));
        assert!(is_generated_path("schema.gen.rs"));
        assert!(is_generated_path("proto.g.rs"));
        assert!(is_generated_path("SRC/GENERATED/FOO.TS")); // case insensitive

        assert!(!is_generated_path("src/utils.ts"));
        assert!(!is_generated_path("generic.ts")); // 'generic' != 'generated'
    }

    #[test]
    fn file_kind_config_variations() {
        // Directory-based config (must have /config/ in middle)
        let (kind, _, _) = file_kind("src/config/app.ts");
        assert_eq!(kind, "config");

        // File-suffix based config (must end with "config.ts" or ".config.ts" etc)
        let (kind, _, _) = file_kind("vite.config.ts");
        assert_eq!(kind, "config");

        let (kind, _, _) = file_kind("tailwind.config.js");
        assert_eq!(kind, "config");

        // Note: tsconfig.json doesn't match pattern - would need config.json or .config.json
        let (kind, _, _) = file_kind("app.config.json");
        assert_eq!(kind, "config");
    }

    #[test]
    fn file_kind_priority_generated_over_test() {
        // Generated takes priority over test for kind, but test flag is set independently
        let (kind, test, generated) = file_kind("__tests__/generated/mock.gen.ts");
        assert_eq!(kind, "generated");
        assert!(test); // test flag is true because path contains __tests__
        assert!(generated);
    }

    #[test]
    fn file_kind_priority_test_over_story() {
        // Test takes priority over story
        let (kind, test, _) = file_kind("Button.stories.test.ts");
        assert_eq!(kind, "test");
        assert!(test);
    }

    #[test]
    fn language_from_path_edge_cases() {
        // No extension - returns empty string
        assert_eq!(language_from_path("Makefile"), "");
        assert_eq!(language_from_path("src/noext"), "");

        // Hidden files without extension - returns empty (extension() returns None)
        assert_eq!(language_from_path(".gitignore"), "");
        assert_eq!(language_from_path(".env"), "");

        // Double extensions (only last matters) - note tsx -> ts mapping
        assert_eq!(language_from_path("file.test.ts"), "ts");
        assert_eq!(language_from_path("app.module.tsx"), "ts"); // tsx mapped to ts
    }
}
