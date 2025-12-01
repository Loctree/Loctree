//! End-to-End CLI Tests for loctree
//!
//! Following TDD principles - tests define expected behavior.
//! Created by M&K (c)2025 The LibraxisAI Team

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

/// Get path to test fixtures
fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Get a command pointing to the loctree binary
fn loctree() -> Command {
    cargo_bin_cmd!("loctree")
}

// ============================================
// Basic CLI Tests
// ============================================

mod cli_basics {
    use super::*;

    #[test]
    fn shows_help() {
        loctree()
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("loctree"))
            .stdout(predicate::str::contains("slice"));
    }

    #[test]
    fn shows_version() {
        loctree()
            .arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn shows_full_help() {
        loctree()
            .arg("--help-full")
            .assert()
            .success()
            .stdout(predicate::str::contains("--circular"))
            .stdout(predicate::str::contains("--dead"))
            .stdout(predicate::str::contains("--sarif"));
    }
}

// ============================================
// Scan Mode Tests
// ============================================

mod scan_mode {
    use super::*;

    #[test]
    fn scans_typescript_project() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .assert()
            .success()
            .stdout(predicate::str::contains("ts").or(predicate::str::contains("Scanned")));
    }

    #[test]
    fn creates_snapshot() {
        let temp = TempDir::new().unwrap();
        let fixture = fixtures_path().join("simple_ts");

        // Copy fixture to temp
        copy_dir_all(&fixture, temp.path()).unwrap();

        loctree().current_dir(temp.path()).assert().success();

        // Snapshot should exist
        assert!(temp.path().join(".loctree/snapshot.json").exists());
    }

    #[test]
    fn respects_gitignore_flag() {
        let fixture = fixtures_path().join("simple_ts");

        loctree().current_dir(&fixture).arg("-g").assert().success();
    }
}

// ============================================
// Slice Mode Tests
// ============================================

mod slice_mode {
    use super::*;

    /// Helper to ensure snapshot exists before slice tests
    fn ensure_snapshot(fixture: &std::path::Path) {
        loctree().current_dir(fixture).assert().success();
    }

    #[test]
    fn slices_single_file() {
        let fixture = fixtures_path().join("simple_ts");
        ensure_snapshot(&fixture);

        loctree()
            .current_dir(&fixture)
            .args(["slice", "src/index.ts"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Core"))
            .stdout(predicate::str::contains("index.ts"));
    }

    #[test]
    fn slices_with_deps() {
        let fixture = fixtures_path().join("simple_ts");
        ensure_snapshot(&fixture);

        loctree()
            .current_dir(&fixture)
            .args(["slice", "src/index.ts"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Deps"))
            .stdout(predicate::str::contains("greeting.ts"))
            .stdout(predicate::str::contains("date.ts"));
    }

    #[test]
    fn slices_with_consumers() {
        let fixture = fixtures_path().join("simple_ts");
        ensure_snapshot(&fixture);

        loctree()
            .current_dir(&fixture)
            .args(["slice", "src/utils/greeting.ts", "--consumers"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Consumers"))
            .stdout(predicate::str::contains("index.ts"));
    }

    #[test]
    fn slices_json_output() {
        let fixture = fixtures_path().join("simple_ts");
        ensure_snapshot(&fixture);

        loctree()
            .current_dir(&fixture)
            .args(["slice", "src/index.ts", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::contains(r#""core""#))
            .stdout(predicate::str::contains(r#""deps""#));
    }

    #[test]
    fn slice_file_not_found() {
        let fixture = fixtures_path().join("simple_ts");
        ensure_snapshot(&fixture);

        loctree()
            .current_dir(&fixture)
            .args(["slice", "nonexistent.ts"])
            .assert()
            .failure();
    }
}

// ============================================
// Analyzer Mode Tests
// ============================================

mod analyzer_mode {
    use super::*;

    #[test]
    fn detects_circular_imports() {
        let fixture = fixtures_path().join("circular_imports");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--circular"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Circular"));
    }

    #[test]
    fn detects_dead_exports() {
        let fixture = fixtures_path().join("dead_code");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--dead"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("deadFunction")
                    .or(predicate::str::contains("DEAD_CONSTANT")),
            );
    }

    #[test]
    fn lists_entrypoints() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--entrypoints"])
            .assert()
            .success()
            // Entry points might be empty for simple TS project without main()
            .stdout(
                predicate::str::is_empty()
                    .not()
                    .or(predicate::str::contains("Entry")),
            );
    }

    #[test]
    fn checks_similar_components() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--check", "greet"])
            .assert()
            .success()
            .stdout(predicate::str::contains("greet").or(predicate::str::contains("greeting")));
    }

    #[test]
    fn analyzes_impact() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--impact", "src/utils/greeting.ts"])
            .assert()
            .success()
            .stdout(predicate::str::contains("index.ts"));
    }

    #[test]
    fn finds_symbol() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--symbol", "greet"])
            .assert()
            .success()
            .stdout(predicate::str::contains("greeting.ts"));
    }

    #[test]
    fn outputs_sarif() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--sarif"])
            .assert()
            .success()
            .stdout(predicate::str::contains(r#""$schema""#))
            .stdout(predicate::str::contains("sarif"));
    }

    #[test]
    fn outputs_json() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{"));
    }
}

// ============================================
// Tauri Mode Tests
// ============================================

mod tauri_mode {
    use super::*;

    #[test]
    fn detects_tauri_project() {
        let fixture = fixtures_path().join("tauri_app");

        loctree()
            .current_dir(&fixture)
            .assert()
            .success()
            .stdout(predicate::str::contains("Tauri"));
    }

    #[test]
    fn analyzes_tauri_handlers() {
        let fixture = fixtures_path().join("tauri_app");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--preset-tauri", "src", "src-tauri/src"])
            .assert()
            .success();
    }

    #[test]
    fn detects_missing_handlers() {
        let fixture = fixtures_path().join("tauri_app");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--preset-tauri", "src", "src-tauri/src"])
            .assert()
            .success()
            .stdout(predicate::str::contains("missing_handler"));
    }

    #[test]
    fn detects_unused_handlers() {
        let fixture = fixtures_path().join("tauri_app");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--preset-tauri", "src", "src-tauri/src"])
            .assert()
            .success()
            .stdout(predicate::str::contains("unused_handler"));
    }
}

// ============================================
// CI Fail Flag Tests
// ============================================

mod ci_fail_flags {
    use super::*;

    #[test]
    fn fails_on_missing_handlers() {
        let fixture = fixtures_path().join("tauri_app");

        loctree()
            .current_dir(&fixture)
            .args([
                "-A",
                "--preset-tauri",
                "src",
                "src-tauri/src",
                "--fail-on-missing-handlers",
            ])
            .assert()
            .failure()
            .code(1);
    }

    #[test]
    fn passes_when_no_missing_handlers() {
        // Must isolate fixture to avoid scanning parent repo (loctree-dev)
        // which contains other fixtures with missing handlers!
        let temp = TempDir::new().unwrap();
        let fixture = fixtures_path().join("simple_ts");
        copy_dir_all(&fixture, temp.path()).unwrap();

        // Non-Tauri project shouldn't fail on missing handlers
        loctree()
            .current_dir(temp.path())
            .args(["-A", "--fail-on-missing-handlers"])
            .assert()
            .success();
    }
}

// ============================================
// Confidence Scoring Tests
// ============================================

mod confidence_scoring {
    use super::*;

    #[test]
    fn filters_high_confidence() {
        let fixture = fixtures_path().join("dead_code");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--dead", "--confidence", "high"])
            .assert()
            .success();
    }

    #[test]
    fn filters_low_confidence() {
        let fixture = fixtures_path().join("dead_code");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--dead", "--confidence", "low"])
            .assert()
            .success();
    }

    #[test]
    fn shows_all_confidence_levels() {
        let fixture = fixtures_path().join("dead_code");

        loctree()
            .current_dir(&fixture)
            .args(["-A", "--dead", "--confidence", "all"])
            .assert()
            .success();
    }
}

// ============================================
// Trace Command Tests
// ============================================

mod trace_command {
    use super::*;

    #[test]
    fn traces_handler() {
        let fixture = fixtures_path().join("tauri_app");

        loctree()
            .current_dir(&fixture)
            .args(["trace", "unused_handler", "src", "src-tauri/src"])
            .assert()
            .success();
    }
}

// ============================================
// Git Commands Tests (Semantic Analysis)
// ============================================

mod git_commands {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    /// Create a temporary git repository for testing
    fn create_test_git_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()
            .unwrap();

        // Create initial file and commit
        std::fs::write(path.join("main.ts"), "export function main() {}").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(path)
            .output()
            .unwrap();

        // Add second commit
        std::fs::write(
            path.join("utils.ts"),
            "export function add(a: number, b: number) { return a + b; }",
        )
        .unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Add utils"])
            .current_dir(path)
            .output()
            .unwrap();

        temp_dir
    }

    #[test]
    fn git_compare_shows_json_output() {
        let temp_dir = create_test_git_repo();

        loctree()
            .current_dir(temp_dir.path())
            .args(["git", "compare", "HEAD~1", "HEAD"])
            .assert()
            .success()
            .stdout(predicate::str::contains("from_commit"))
            .stdout(predicate::str::contains("to_commit"))
            .stdout(predicate::str::contains("files"))
            .stdout(predicate::str::contains("impact"));
    }

    #[test]
    fn git_compare_with_range_notation() {
        let temp_dir = create_test_git_repo();

        loctree()
            .current_dir(temp_dir.path())
            .args(["git", "compare", "HEAD~1..HEAD"])
            .assert()
            .success()
            .stdout(predicate::str::contains("from_commit"));
    }

    #[test]
    fn git_compare_shows_added_files() {
        let temp_dir = create_test_git_repo();

        loctree()
            .current_dir(temp_dir.path())
            .args(["git", "compare", "HEAD~1", "HEAD"])
            .assert()
            .success()
            .stdout(predicate::str::contains("utils.ts"));
    }

    #[test]
    fn git_command_fails_in_non_git_dir() {
        // Create a truly isolated temp directory (not inside any git repo)
        let temp_dir = TempDir::new().unwrap();

        // Create a simple file so it's not empty
        std::fs::write(temp_dir.path().join("test.txt"), "hello").unwrap();

        loctree()
            .current_dir(temp_dir.path())
            .args(["git", "compare", "HEAD~1"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("not a git repository"));
    }

    #[test]
    fn git_blame_returns_not_implemented() {
        let temp_dir = create_test_git_repo();

        loctree()
            .current_dir(temp_dir.path())
            .args(["git", "blame", "main.ts"])
            .assert()
            .success()
            .stdout(predicate::str::contains("not_implemented"))
            .stdout(predicate::str::contains("Phase 2"));
    }

    #[test]
    fn git_history_returns_not_implemented() {
        let temp_dir = create_test_git_repo();

        loctree()
            .current_dir(temp_dir.path())
            .args(["git", "history", "main"])
            .assert()
            .success()
            .stdout(predicate::str::contains("not_implemented"))
            .stdout(predicate::str::contains("Phase 3"));
    }

    #[test]
    fn git_when_introduced_returns_not_implemented() {
        let temp_dir = create_test_git_repo();

        loctree()
            .current_dir(temp_dir.path())
            .args(["git", "when-introduced", "--dead", "unused_fn"])
            .assert()
            .success()
            .stdout(predicate::str::contains("not_implemented"))
            .stdout(predicate::str::contains("Phase 3"));
    }

    #[test]
    fn git_compare_shows_commit_info() {
        let temp_dir = create_test_git_repo();

        loctree()
            .current_dir(temp_dir.path())
            .args(["git", "compare", "HEAD~1", "HEAD"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Initial commit"))
            .stdout(predicate::str::contains("Add utils"));
    }
}

// ============================================
// Helper Functions
// ============================================

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}
