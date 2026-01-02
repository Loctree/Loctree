use std::path::Path;

/// Classification of a file's test status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestClassification {
    /// Production code (not a test)
    Production,
    /// Unit test file
    UnitTest,
    /// Integration test file
    IntegrationTest,
    /// Test fixture/mock data
    TestFixture,
    /// Test utility/helper
    TestHelper,
}

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

pub fn is_test_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("__tests__")
        || lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.ends_with("_test.rs")
        || lower.ends_with("_tests.rs")
        || lower.ends_with("_test.go")
        || lower.ends_with("_test.dart")
        || lower.starts_with("test_")
        || lower.contains("/tests/")
        || lower.starts_with("tests/")
        || lower.contains("/test_")
}

/// Check if a path should be excluded from production analysis reports.
/// This includes test files, test fixtures, mocks, and test-related paths.
///
/// # Examples
/// ```
/// use loctree::analyzer::classify::should_exclude_from_reports;
/// assert!(should_exclude_from_reports("tests/fixtures/foo.rs"));
/// assert!(should_exclude_from_reports("src/__tests__/bar.spec.ts"));
/// assert!(should_exclude_from_reports("src/__mocks__/api.ts"));
/// assert!(!should_exclude_from_reports("src/api/handler.rs"));
/// ```
pub fn should_exclude_from_reports(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();

    // Test directories
    lower.contains("/tests/")
        || lower.starts_with("tests/")
        || lower.contains("__tests__")
        || lower.contains("__mocks__")

    // Fixture/mock directories
        || lower.contains("/fixtures/")
        || lower.contains("/fixture/")
        || lower.contains("__fixtures__")
        || lower.contains("/mocks/")
        || lower.contains("/mock/")

    // Test file patterns
        || lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.ends_with("_test.rs")
        || lower.ends_with("_tests.rs")
        || lower.ends_with("_test.ts")
        || lower.ends_with("_test.tsx")
        || lower.ends_with("_test.js")
        || lower.ends_with("_test.jsx")
        || lower.ends_with("_test.go")
        || lower.ends_with("_test.dart")
        || lower.ends_with("_test.py")

    // Test helpers/utilities
        || lower.contains("/test_utils/")
        || lower.contains("/test_helpers/")
        || lower.contains("/testing/")
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

/// Detect if a path is a test file and classify it
pub fn classify_test_path(path: &Path) -> TestClassification {
    let path_str = path.to_str().unwrap_or("");
    let lower = path_str.to_ascii_lowercase();
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let filename_lower = filename.to_ascii_lowercase();

    // Fixture/mock detection (not including "test_" prefix files)
    if (lower.contains("fixture") || lower.contains("fixtures") || lower.contains("mock"))
        && !filename_lower.starts_with("test_")
    {
        return TestClassification::TestFixture;
    }

    // Test helper/utility detection - BEFORE integration/unit test detection
    // Only specific helper patterns (not Python test_ files)
    let is_python = filename_lower.ends_with(".py");
    let is_python_test_file = is_python && filename_lower.starts_with("test_");

    if !is_python_test_file
        && (filename_lower.contains("test_helper")
            || filename_lower.contains("test_utils")
            || filename_lower == "setup.py"
            || lower.contains("testing/"))
    {
        return TestClassification::TestHelper;
    }

    // Integration test detection (in tests/ directory but not __tests__)
    // Check if path starts with "tests/" or contains "/tests/"
    if (lower.starts_with("tests/") || lower.contains("/tests/")) && !lower.contains("__tests__") {
        return TestClassification::IntegrationTest;
    }

    // Unit test detection
    if lower.contains("__tests__")
        || lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.ends_with("_test.rs")
        || lower.ends_with("_tests.rs")
        || lower.ends_with("_test.go")
        || lower.ends_with("_test.dart")
        || filename_lower.starts_with("test_") // Python test_*.py files
        || lower.contains("/test_")
    {
        return TestClassification::UnitTest;
    }

    TestClassification::Production
}

/// Check if file content contains test code (for Rust #[cfg(test)] detection)
pub fn has_test_code(content: &str, lang: &str) -> bool {
    match lang {
        "rs" | "rust" => content.contains("#[cfg(test)]") || content.contains("#[test]"),
        "ts" | "js" | "tsx" | "jsx" => {
            content.contains("describe(") || content.contains("it(") || content.contains("test(")
        }
        "py" | "python" => {
            content.contains("def test_")
                || content.contains("import unittest")
                || content.contains("import pytest")
        }
        "go" => content.contains("func Test") || content.contains("testing.T"),
        _ => false,
    }
}

/// Get all test-related file patterns for a language
pub fn test_patterns(lang: &str) -> Vec<&'static str> {
    match lang {
        "rs" | "rust" => vec!["*_test.rs", "*_tests.rs", "tests/**/*.rs"],
        "ts" | "tsx" | "js" | "jsx" => vec![
            "*.test.ts",
            "*.test.tsx",
            "*.test.js",
            "*.test.jsx",
            "*.spec.ts",
            "*.spec.tsx",
            "*.spec.js",
            "*.spec.jsx",
            "__tests__/**/*",
        ],
        "py" | "python" => vec!["test_*.py", "*_test.py", "tests/**/*.py"],
        "go" => vec!["*_test.go"],
        "dart" => vec!["*_test.dart", "test/**/*.dart"],
        _ => vec![],
    }
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
        // New patterns
        assert!(is_test_path("test_parser.py"));
        assert!(is_test_path("src/test_utils.py"));
        assert!(is_test_path("tests/api/test.rs"));
        assert!(is_test_path("src/tests/integration.py"));

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

    #[test]
    fn classify_test_path_unit_tests() {
        // TypeScript/JavaScript unit tests
        assert_eq!(
            classify_test_path(Path::new("src/components/Button.test.tsx")),
            TestClassification::UnitTest
        );
        assert_eq!(
            classify_test_path(Path::new("utils.spec.ts")),
            TestClassification::UnitTest
        );
        assert_eq!(
            classify_test_path(Path::new("src/__tests__/helper.ts")),
            TestClassification::UnitTest
        );

        // Rust unit tests
        assert_eq!(
            classify_test_path(Path::new("src/parser_test.rs")),
            TestClassification::UnitTest
        );
        assert_eq!(
            classify_test_path(Path::new("lib/module_tests.rs")),
            TestClassification::UnitTest
        );

        // Python unit tests
        assert_eq!(
            classify_test_path(Path::new("test_utils.py")),
            TestClassification::UnitTest
        );

        // Go unit tests
        assert_eq!(
            classify_test_path(Path::new("handler_test.go")),
            TestClassification::UnitTest
        );
    }

    #[test]
    fn classify_test_path_integration_tests() {
        // Rust integration tests (in tests/ directory)
        assert_eq!(
            classify_test_path(Path::new("tests/api/endpoints.rs")),
            TestClassification::IntegrationTest
        );
        assert_eq!(
            classify_test_path(Path::new("tests/integration/database.rs")),
            TestClassification::IntegrationTest
        );

        // __tests__ should still be unit tests even with /tests/ in path
        assert_eq!(
            classify_test_path(Path::new("src/__tests__/integration.test.ts")),
            TestClassification::UnitTest
        );
    }

    #[test]
    fn classify_test_path_fixtures() {
        assert_eq!(
            classify_test_path(Path::new("tests/fixtures/data.json")),
            TestClassification::TestFixture
        );
        assert_eq!(
            classify_test_path(Path::new("__tests__/fixture/mock.ts")),
            TestClassification::TestFixture
        );
        assert_eq!(
            classify_test_path(Path::new("test/mock/server.rs")),
            TestClassification::TestFixture
        );
    }

    #[test]
    fn classify_test_path_helpers() {
        assert_eq!(
            classify_test_path(Path::new("tests/test_helper.rs")),
            TestClassification::TestHelper
        );
        assert_eq!(
            classify_test_path(Path::new("__tests__/test_utils.ts")),
            TestClassification::TestHelper
        );
        assert_eq!(
            classify_test_path(Path::new("testing/setup.py")),
            TestClassification::TestHelper
        );
    }

    #[test]
    fn classify_test_path_production() {
        assert_eq!(
            classify_test_path(Path::new("src/components/Button.tsx")),
            TestClassification::Production
        );
        assert_eq!(
            classify_test_path(Path::new("lib/parser.rs")),
            TestClassification::Production
        );
        assert_eq!(
            classify_test_path(Path::new("utils/helpers.py")),
            TestClassification::Production
        );
    }

    #[test]
    fn has_test_code_rust() {
        // Rust with #[cfg(test)]
        let code_with_test_module = r#"
            fn main() {}

            #[cfg(test)]
            mod tests {
                #[test]
                fn it_works() {
                    assert_eq!(2 + 2, 4);
                }
            }
        "#;
        assert!(has_test_code(code_with_test_module, "rs"));

        // Rust with standalone #[test]
        let code_with_test_fn = r#"
            #[test]
            fn test_something() {}
        "#;
        assert!(has_test_code(code_with_test_fn, "rust"));

        // Rust production code
        let production_code = r#"
            fn parse(input: &str) -> Result<(), Error> {
                Ok(())
            }
        "#;
        assert!(!has_test_code(production_code, "rs"));
    }

    #[test]
    fn has_test_code_typescript() {
        // Jest/Vitest style tests
        let jest_test = r#"
            describe('Button', () => {
                it('should render', () => {
                    expect(true).toBe(true);
                });
            });
        "#;
        assert!(has_test_code(jest_test, "ts"));

        let vitest_test = r#"
            test('adds 1 + 2 to equal 3', () => {
                expect(1 + 2).toBe(3);
            });
        "#;
        assert!(has_test_code(vitest_test, "tsx"));

        // Production code
        let production = r#"
            export function add(a: number, b: number): number {
                return a + b;
            }
        "#;
        assert!(!has_test_code(production, "ts"));
    }

    #[test]
    fn has_test_code_python() {
        // unittest style
        let unittest_code = r#"
            import unittest

            class TestMath(unittest.TestCase):
                def test_addition(self):
                    self.assertEqual(1 + 1, 2)
        "#;
        assert!(has_test_code(unittest_code, "py"));

        // pytest style
        let pytest_code = r#"
            import pytest

            def test_addition():
                assert 1 + 1 == 2
        "#;
        assert!(has_test_code(pytest_code, "python"));

        // Function name pattern
        let test_function = r#"
            def test_something():
                pass
        "#;
        assert!(has_test_code(test_function, "py"));

        // Production code
        let production = r#"
            def add(a, b):
                return a + b
        "#;
        assert!(!has_test_code(production, "py"));
    }

    #[test]
    fn has_test_code_go() {
        // Go test
        let go_test = r#"
            package main

            import "testing"

            func TestAdd(t *testing.T) {
                result := Add(1, 2)
                if result != 3 {
                    t.Errorf("Expected 3, got %d", result)
                }
            }
        "#;
        assert!(has_test_code(go_test, "go"));

        // Production code
        let production = r#"
            package main

            func Add(a, b int) int {
                return a + b
            }
        "#;
        assert!(!has_test_code(production, "go"));
    }

    #[test]
    fn test_patterns_all_languages() {
        // Rust patterns
        let rust_patterns = test_patterns("rs");
        assert!(rust_patterns.contains(&"*_test.rs"));
        assert!(rust_patterns.contains(&"*_tests.rs"));
        assert!(rust_patterns.contains(&"tests/**/*.rs"));

        // TypeScript patterns
        let ts_patterns = test_patterns("ts");
        assert!(ts_patterns.contains(&"*.test.ts"));
        assert!(ts_patterns.contains(&"*.spec.tsx"));
        assert!(ts_patterns.contains(&"__tests__/**/*"));

        // Python patterns
        let py_patterns = test_patterns("py");
        assert!(py_patterns.contains(&"test_*.py"));
        assert!(py_patterns.contains(&"*_test.py"));
        assert!(py_patterns.contains(&"tests/**/*.py"));

        // Go patterns
        let go_patterns = test_patterns("go");
        assert!(go_patterns.contains(&"*_test.go"));

        // Unknown language
        let unknown_patterns = test_patterns("unknown");
        assert!(unknown_patterns.is_empty());
    }
}
