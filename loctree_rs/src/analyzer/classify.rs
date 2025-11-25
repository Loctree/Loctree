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
        let (kind, test, gen) = file_kind("src/generated/foo.gen.ts");
        assert_eq!(kind, "generated");
        assert!(!test);
        assert!(gen);

        let (kind, test, gen) = file_kind("src/components/Button.story.tsx");
        assert_eq!(kind, "story");
        assert!(!gen);
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
}
