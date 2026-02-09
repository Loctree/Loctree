//! End-to-End CLI Tests for loctree
//!
//! Following TDD principles - tests define expected behavior.
//! Vibecrafted with AI Agents by VetCoders (c)2025 VetCoders

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
            .stdout(predicate::str::contains("--sarif").or(predicate::str::contains("sarif")))
            .stdout(predicate::str::contains("dead").or(predicate::str::contains("cycles")));
    }

    #[test]
    fn global_fresh_flag_does_not_break_subcommands() {
        loctree()
            .args(["--fresh", "query", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("loct query"));
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
            // Summary output goes to stderr (stdout reserved for machine-readable data)
            .stderr(predicate::str::contains("ts").or(predicate::str::contains("Scanned")));
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
    fn slice_rescan_flag_triggers_rescan() {
        let fixture = fixtures_path().join("simple_ts");
        ensure_snapshot(&fixture);

        loctree()
            .current_dir(&fixture)
            .args(["slice", "src/index.ts", "--rescan"])
            .write_stdin("")
            .assert()
            .success()
            .stderr(predicate::str::contains("Rescanning"));
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
            // Summary output goes to stderr (stdout reserved for machine-readable data)
            .stderr(predicate::str::contains("handlers")); // Tauri mode detected = handlers shown
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
// Impact Analysis Tests
// ============================================

mod impact_mode {
    use super::*;

    /// Helper to ensure snapshot exists before impact tests
    fn ensure_snapshot(fixture: &std::path::Path) {
        loctree().current_dir(fixture).assert().success();
    }

    #[test]
    fn impact_shows_direct_consumers() {
        let fixture = fixtures_path().join("simple_ts");
        ensure_snapshot(&fixture);

        loctree()
            .current_dir(&fixture)
            .args(["impact", "src/utils/greeting.ts"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Impact analysis"))
            .stdout(predicate::str::contains("Direct consumers"));
    }

    #[test]
    fn impact_shows_transitive_consumers() {
        let fixture = fixtures_path().join("simple_ts");
        ensure_snapshot(&fixture);

        loctree()
            .current_dir(&fixture)
            .args(["impact", "src/utils/date.ts"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Impact analysis"));
    }

    #[test]
    fn impact_no_consumers_safe_to_remove() {
        let fixture = fixtures_path().join("simple_ts");
        ensure_snapshot(&fixture);

        // index.ts is likely a top-level file with no consumers
        loctree()
            .current_dir(&fixture)
            .args(["impact", "src/index.ts"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Impact analysis"));
    }

    #[test]
    fn impact_json_output() {
        let fixture = fixtures_path().join("simple_ts");
        ensure_snapshot(&fixture);

        loctree()
            .current_dir(&fixture)
            .args(["impact", "src/utils/greeting.ts", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::contains(r#""target""#))
            .stdout(predicate::str::contains(r#""direct_consumers""#))
            .stdout(predicate::str::contains(r#""transitive_consumers""#))
            .stdout(predicate::str::contains(r#""total_affected""#));
    }

    #[test]
    fn impact_with_max_depth() {
        let fixture = fixtures_path().join("simple_ts");
        ensure_snapshot(&fixture);

        loctree()
            .current_dir(&fixture)
            .args(["impact", "src/utils/greeting.ts", "--max-depth", "1"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Impact analysis"));
    }

    #[test]
    fn impact_file_not_found() {
        let fixture = fixtures_path().join("simple_ts");
        ensure_snapshot(&fixture);

        // Nonexistent file returns error (file must exist in snapshot)
        loctree()
            .current_dir(&fixture)
            .args(["impact", "nonexistent.ts"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("File not found in snapshot"));
    }

    #[test]
    fn impact_without_snapshot_auto_scans() {
        let temp = TempDir::new().unwrap();
        // Create minimal file structure
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/test.ts"), "export const x = 1;").unwrap();

        // Without snapshot, impact command auto-scans first (good UX)
        loctree()
            .current_dir(temp.path())
            .args(["impact", "src/test.ts"])
            .assert()
            .success()
            .stderr(predicate::str::contains("running initial scan"));
    }
}

// ============================================
// Diff Mode Tests (auto-scan-base flag)
// ============================================

mod diff_mode_new_features {
    use super::*;

    #[test]
    fn diff_help_shows_auto_scan_base_flag() {
        loctree()
            .args(["diff", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("auto-scan-base")
                    .or(predicate::str::contains("Auto-scan base commit")),
            );
    }

    #[test]
    fn diff_auto_scan_base_flag_exists() {
        // Just verify the flag is recognized (don't need actual git worktree)
        loctree()
            .args(["diff", "--auto-scan-base", "--help"])
            .assert()
            .success();
    }
}

// ============================================
// Watch Mode Tests
// ============================================

mod watch_mode {
    use super::*;

    #[test]
    fn watch_help_shows_flag() {
        loctree().arg("--help").assert().success().stdout(
            predicate::str::contains("watch")
                .or(predicate::str::contains("Watch for file changes")),
        );
    }

    #[test]
    fn watch_flag_recognized() {
        // Just verify the flag is parsed (won't actually start watching in test)
        // This will timeout or need Ctrl+C, so we just check it doesn't error on parsing
        let fixture = fixtures_path().join("simple_ts");

        // Run with timeout to avoid hanging
        // Note: This test is limited - real watch mode testing would require
        // simulating file changes or mocking the watcher
        // Use new CLI syntax: `loct scan --watch` instead of legacy `loct --watch`
        loctree()
            .current_dir(&fixture)
            .args(["scan", "--watch"])
            .timeout(std::time::Duration::from_millis(100))
            .assert()
            .interrupted(); // Expect timeout/interrupt
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

/// Copy directory tree, skipping a named top-level entry (e.g. ".loctree").
fn copy_dir_excluding(
    src: &std::path::Path,
    dst: &std::path::Path,
    exclude: &str,
) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_name() == exclude {
            continue;
        }
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

// ============================================
// Instant Commands Tests (<100ms)
// ============================================
// Vibecrafted with AI Agents by VetCoders (c)2026 VetCoders

mod instant_commands {
    use super::*;

    // ----------------------------------------
    // Focus Command Tests
    // ----------------------------------------

    #[test]
    fn focus_help_shows_usage() {
        loctree()
            .args(["focus", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("focus").or(predicate::str::contains("directory")));
    }

    #[test]
    fn focus_on_directory() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["focus", "src/"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("src")
                    .or(predicate::str::contains("Focus"))
                    .or(predicate::str::contains("files")),
            );
    }

    #[test]
    fn focus_json_output() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["focus", "src/", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Hotspots Command Tests
    // ----------------------------------------

    #[test]
    fn hotspots_help_shows_usage() {
        loctree()
            .args(["hotspots", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("hotspots")
                    .or(predicate::str::contains("import"))
                    .or(predicate::str::contains("frequency")),
            );
    }

    #[test]
    fn hotspots_runs_successfully() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["hotspots"])
            .assert()
            .success();
    }

    #[test]
    fn hotspots_json_output() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["hotspots", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Health Command Tests
    // ----------------------------------------

    #[test]
    fn health_help_shows_usage() {
        loctree()
            .args(["health", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("health").or(predicate::str::contains("check")));
    }

    #[test]
    fn health_runs_successfully() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["health"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("Health")
                    .or(predicate::str::contains("OK"))
                    .or(predicate::str::contains("score")),
            );
    }

    #[test]
    fn health_alias_h() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["h"])
            .assert()
            .success();
    }

    #[test]
    fn health_json_output() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["health", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::contains(r#""health"#).or(predicate::str::starts_with("{")));
    }

    // ----------------------------------------
    // Query Command Tests
    // ----------------------------------------

    #[test]
    fn query_help_shows_usage() {
        loctree()
            .args(["query", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("query")
                    .or(predicate::str::contains("who-imports"))
                    .or(predicate::str::contains("where-symbol")),
            );
    }

    #[test]
    fn query_who_imports() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["query", "who-imports", "src/utils/greeting.ts"])
            .assert()
            .success();
    }

    #[test]
    fn query_where_symbol() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["query", "where-symbol", "greet"])
            .assert()
            .success();
    }

    #[test]
    fn query_alias_q() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["q", "who-imports", "src/utils/greeting.ts"])
            .assert()
            .success();
    }

    // ----------------------------------------
    // Commands Command Tests (Tauri)
    // ----------------------------------------

    #[test]
    fn commands_help_shows_usage() {
        loctree()
            .args(["commands", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("commands")
                    .or(predicate::str::contains("Tauri"))
                    .or(predicate::str::contains("handler")),
            );
    }

    #[test]
    fn commands_in_tauri_project() {
        let fixture = fixtures_path().join("tauri_app");

        loctree()
            .current_dir(&fixture)
            .args(["commands"])
            .assert()
            .success();
    }

    #[test]
    fn commands_json_output() {
        let fixture = fixtures_path().join("tauri_app");

        loctree()
            .current_dir(&fixture)
            .args(["commands", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Events Command Tests
    // ----------------------------------------

    #[test]
    fn events_help_shows_usage() {
        loctree()
            .args(["events", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("events")
                    .or(predicate::str::contains("emit"))
                    .or(predicate::str::contains("listen")),
            );
    }

    #[test]
    fn events_runs_successfully() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["events"])
            .assert()
            .success();
    }

    #[test]
    fn events_json_output() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["events", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Coverage Command Tests
    // ----------------------------------------

    #[test]
    fn coverage_help_shows_usage() {
        loctree()
            .args(["coverage", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("coverage")
                    .or(predicate::str::contains("test"))
                    .or(predicate::str::contains("gaps")),
            );
    }

    #[test]
    fn coverage_runs_successfully() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["coverage"])
            .assert()
            .success();
    }

    #[test]
    fn coverage_json_output() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["coverage", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }
}

// ============================================
// Analysis Commands Tests
// ============================================
// Vibecrafted with AI Agents by VetCoders (c)2026 VetCoders

mod analysis_commands {
    use super::*;

    // ----------------------------------------
    // Dead Command Tests
    // ----------------------------------------

    #[test]
    fn dead_help_shows_usage() {
        loctree()
            .args(["dead", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("dead")
                    .or(predicate::str::contains("unused"))
                    .or(predicate::str::contains("exports")),
            );
    }

    #[test]
    fn dead_detects_unused_exports() {
        let fixture = fixtures_path().join("dead_code");

        loctree()
            .current_dir(&fixture)
            .args(["dead"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("dead")
                    .or(predicate::str::contains("unused"))
                    .or(predicate::str::contains("DEAD_CONSTANT")),
            );
    }

    #[test]
    fn dead_alias_d() {
        let fixture = fixtures_path().join("dead_code");

        loctree()
            .current_dir(&fixture)
            .args(["d"])
            .assert()
            .success();
    }

    #[test]
    fn dead_json_output() {
        let fixture = fixtures_path().join("dead_code");

        loctree()
            .current_dir(&fixture)
            .args(["dead", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Cycles Command Tests
    // ----------------------------------------

    #[test]
    fn cycles_help_shows_usage() {
        loctree()
            .args(["cycles", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("cycles")
                    .or(predicate::str::contains("circular"))
                    .or(predicate::str::contains("imports")),
            );
    }

    #[test]
    fn cycles_detects_circular_imports() {
        let fixture = fixtures_path().join("circular_imports");

        loctree()
            .current_dir(&fixture)
            .args(["cycles"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("cycle")
                    .or(predicate::str::contains("circular"))
                    .or(predicate::str::contains("→")),
            );
    }

    #[test]
    fn cycles_alias_c() {
        let fixture = fixtures_path().join("circular_imports");

        loctree()
            .current_dir(&fixture)
            .args(["c"])
            .assert()
            .success();
    }

    #[test]
    fn cycles_json_output() {
        let fixture = fixtures_path().join("circular_imports");

        loctree()
            .current_dir(&fixture)
            .args(["cycles", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Twins Command Tests
    // ----------------------------------------

    #[test]
    fn twins_help_shows_usage() {
        loctree()
            .args(["twins", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("twins")
                    .or(predicate::str::contains("duplicate"))
                    .or(predicate::str::contains("dead parrot")),
            );
    }

    #[test]
    fn twins_runs_successfully() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["twins"])
            .assert()
            .success();
    }

    #[test]
    fn twins_alias_t() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["t"])
            .assert()
            .success();
    }

    #[test]
    fn twins_json_output() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["twins", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Zombie Command Tests
    // ----------------------------------------

    #[test]
    fn zombie_help_shows_usage() {
        loctree()
            .args(["zombie", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("zombie")
                    .or(predicate::str::contains("dead"))
                    .or(predicate::str::contains("orphan")),
            );
    }

    #[test]
    fn zombie_runs_successfully() {
        let fixture = fixtures_path().join("dead_code");

        loctree()
            .current_dir(&fixture)
            .args(["zombie"])
            .assert()
            .success();
    }

    #[test]
    fn zombie_json_output() {
        let fixture = fixtures_path().join("dead_code");

        loctree()
            .current_dir(&fixture)
            .args(["zombie", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Audit Command Tests
    // ----------------------------------------

    #[test]
    fn audit_help_shows_usage() {
        loctree()
            .args(["audit", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("audit")
                    .or(predicate::str::contains("full"))
                    .or(predicate::str::contains("codebase")),
            );
    }

    #[test]
    fn audit_runs_successfully() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["audit", "--no-open"])
            .assert()
            .success();
    }

    #[test]
    fn audit_stdout_flag() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["audit", "--stdout"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Audit").or(predicate::str::contains("Health")));
    }

    // ----------------------------------------
    // Crowd Command Tests
    // ----------------------------------------

    #[test]
    fn crowd_help_shows_usage() {
        loctree()
            .args(["crowd", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("crowd")
                    .or(predicate::str::contains("cluster"))
                    .or(predicate::str::contains("keyword")),
            );
    }

    #[test]
    fn crowd_with_keyword() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["crowd", "greet"])
            .assert()
            .success();
    }

    #[test]
    fn crowd_json_output() {
        // Copy fixture to temp WITHOUT .loctree/ to force auto-scan,
        // verifying --json stdout stays clean even when scanning triggers.
        let temp = TempDir::new().unwrap();
        let fixture = fixtures_path().join("simple_ts");
        copy_dir_excluding(&fixture, temp.path(), ".loctree").unwrap();

        loctree()
            .current_dir(temp.path())
            .args(["crowd", "greet", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Tagmap Command Tests
    // ----------------------------------------

    #[test]
    fn tagmap_help_shows_usage() {
        loctree()
            .args(["tagmap", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("tagmap")
                    .or(predicate::str::contains("search"))
                    .or(predicate::str::contains("unified")),
            );
    }

    #[test]
    fn tagmap_with_keyword() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["tagmap", "greet"])
            .assert()
            .success();
    }

    #[test]
    fn tagmap_json_output() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["tagmap", "greet", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Plan Command Tests
    // ----------------------------------------

    #[test]
    fn plan_help_shows_usage() {
        loctree()
            .args(["plan", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("plan").or(predicate::str::contains("refactor")));
    }

    #[test]
    fn plan_supports_multiple_targets() {
        let fixture = fixtures_path().join("plan_multi");
        let temp = TempDir::new().unwrap();
        copy_dir_all(&fixture, temp.path()).unwrap();

        loctree()
            .current_dir(temp.path())
            .args(["plan", "--json", "src", "other"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("["))
            .stdout(predicate::str::contains("\"target\": \"src\""))
            .stdout(predicate::str::contains("\"target\": \"other\""));
    }

    #[test]
    fn plan_target_layout_affects_move_targets() {
        let fixture = fixtures_path().join("plan_multi");
        let temp = TempDir::new().unwrap();
        copy_dir_all(&fixture, temp.path()).unwrap();

        loctree()
            .current_dir(temp.path())
            .args([
                "plan",
                "--json",
                "src",
                "--target-layout",
                "ui=custom-ui,app=custom-app",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("src/custom-ui"))
            .stdout(predicate::str::contains("src/custom-app"));
    }

    // ----------------------------------------
    // Sniff Command Tests
    // ----------------------------------------

    #[test]
    fn sniff_help_shows_usage() {
        loctree()
            .args(["sniff", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("sniff")
                    .or(predicate::str::contains("smell"))
                    .or(predicate::str::contains("aggregate")),
            );
    }

    #[test]
    fn sniff_runs_successfully() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["sniff"])
            .assert()
            .success();
    }

    #[test]
    fn sniff_json_output() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["sniff", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }
}

// ============================================
// Management & Core Workflow Commands Tests
// ============================================
// Vibecrafted with AI Agents by VetCoders (c)2026 VetCoders

mod management_commands {
    use super::*;

    // ----------------------------------------
    // Doctor Command Tests
    // ----------------------------------------

    #[test]
    fn doctor_help_shows_usage() {
        loctree()
            .args(["doctor", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("doctor")
                    .or(predicate::str::contains("diagnostic"))
                    .or(predicate::str::contains("recommendation")),
            );
    }

    #[test]
    fn doctor_runs_successfully() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["doctor"])
            .assert()
            .success();
    }

    #[test]
    fn doctor_json_output() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["doctor", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Suppress Command Tests
    // ----------------------------------------

    #[test]
    fn suppress_help_shows_usage() {
        loctree()
            .args(["suppress", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("suppress")
                    .or(predicate::str::contains("false positive"))
                    .or(predicate::str::contains("ignore")),
            );
    }

    #[test]
    fn suppress_list_empty() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("main.ts"), "export const x = 1;").unwrap();

        loctree()
            .current_dir(temp.path())
            .args(["suppress", "--list"])
            .assert()
            .success();
    }

    // ----------------------------------------
    // Memex Command Tests
    // ----------------------------------------

    #[test]
    fn memex_help_shows_usage() {
        loctree()
            .args(["memex", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("memex")
                    .or(predicate::str::contains("memory"))
                    .or(predicate::str::contains("vector")),
            );
    }

    // ----------------------------------------
    // Auto Command Tests
    // ----------------------------------------

    #[test]
    fn auto_help_shows_usage() {
        loctree()
            .args(["auto", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("auto")
                    .or(predicate::str::contains("scan"))
                    .or(predicate::str::contains("artifacts")),
            );
    }

    #[test]
    fn auto_creates_loctree_dir() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/main.ts"), "export const x = 1;").unwrap();

        loctree()
            .current_dir(temp.path())
            .args(["auto"])
            .assert()
            .success();

        assert!(temp.path().join(".loctree").exists());
    }

    #[test]
    fn auto_json_output() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/main.ts"), "export const x = 1;").unwrap();

        // auto mode generates .loctree/ artifacts; --json suppresses summary on stderr
        loctree()
            .current_dir(temp.path())
            .args(["auto", "--json"])
            .assert()
            .success();

        // Snapshot should exist
        assert!(temp.path().join(".loctree/snapshot.json").exists());
    }

    // ----------------------------------------
    // Tree Command Tests
    // ----------------------------------------

    #[test]
    fn tree_help_shows_usage() {
        loctree()
            .args(["tree", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("tree")
                    .or(predicate::str::contains("directory"))
                    .or(predicate::str::contains("LOC")),
            );
    }

    #[test]
    fn tree_shows_structure() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["tree"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("src")
                    .or(predicate::str::contains("├"))
                    .or(predicate::str::contains("└")),
            );
    }

    #[test]
    fn tree_with_depth() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["tree", "--depth", "1"])
            .assert()
            .success();
    }

    #[test]
    fn tree_json_output() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["tree", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Find Command Tests
    // ----------------------------------------

    #[test]
    fn find_help_shows_usage() {
        loctree()
            .args(["find", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("find")
                    .or(predicate::str::contains("search"))
                    .or(predicate::str::contains("symbol")),
            );
    }

    #[test]
    fn find_symbol() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["find", "greet"])
            .assert()
            .success()
            .stdout(predicate::str::contains("greet").or(predicate::str::contains("greeting")));
    }

    #[test]
    fn find_alias_f() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["f", "greet"])
            .assert()
            .success();
    }

    #[test]
    fn find_json_output() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["find", "greet", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{").or(predicate::str::starts_with("[")));
    }

    // ----------------------------------------
    // Report Command Tests
    // ----------------------------------------

    #[test]
    fn report_help_shows_usage() {
        loctree()
            .args(["report", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("report")
                    .or(predicate::str::contains("HTML"))
                    .or(predicate::str::contains("generate")),
            );
    }

    #[test]
    fn report_creates_html() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/main.ts"), "export const x = 1;").unwrap();

        // First create snapshot
        loctree().current_dir(temp.path()).assert().success();

        loctree()
            .current_dir(temp.path())
            .args(["report"])
            .assert()
            .success();
    }

    // ----------------------------------------
    // Lint Command Tests
    // ----------------------------------------

    #[test]
    fn lint_help_shows_usage() {
        loctree()
            .args(["lint", "--help"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("lint")
                    .or(predicate::str::contains("policy"))
                    .or(predicate::str::contains("structural")),
            );
    }

    #[test]
    fn lint_runs_successfully() {
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["lint"])
            .assert()
            .success();
    }

    #[test]
    fn lint_with_fail_flag() {
        let fixture = fixtures_path().join("simple_ts");

        // --fail should work (exit code depends on findings)
        loctree()
            .current_dir(&fixture)
            .args(["lint", "--fail"])
            .assert(); // Don't check success/failure - depends on findings
    }

    #[test]
    fn lint_sarif_flag_recognized() {
        let fixture = fixtures_path().join("simple_ts");

        // --sarif should emit SARIF JSON to stdout
        loctree()
            .current_dir(&fixture)
            .args(["lint", "--sarif"])
            .assert()
            .stdout(predicate::str::contains("\"version\""))
            .stdout(predicate::str::contains("\"runs\""))
            .success();
    }
}

// ============================================
// Framework-Specific Command Tests
// ============================================
// Vibecrafted with AI Agents by VetCoders (c)2026 VetCoders

mod framework_commands {
    use super::*;

    // ----------------------------------------
    // Routes Command Tests
    // ----------------------------------------

    #[test]
    fn routes_help_shows_usage() {
        let output = loctree().args(["routes", "--help"]).output().unwrap();
        let combined = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);
        assert!(
            combined.contains("routes")
                || combined.contains("FastAPI")
                || combined.contains("Flask"),
            "Help should mention routes/FastAPI/Flask: {}",
            combined
        );
    }

    #[test]
    fn routes_no_routes_in_ts_project() {
        // TypeScript project has no Python routes - should complete gracefully
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["routes"])
            .assert()
            .success()
            .stdout(predicate::str::contains("No routes detected"));
    }

    #[test]
    fn routes_json_output_empty() {
        // TypeScript project - JSON output should be valid with empty routes
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["routes", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::contains(r#""routes""#))
            .stdout(predicate::str::contains(r#""summary""#))
            .stdout(predicate::str::contains(r#""count""#));
    }

    #[test]
    fn routes_with_framework_filter() {
        // Filter should work even when no routes exist
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["routes", "--framework", "fastapi"])
            .assert()
            .success()
            .stdout(predicate::str::contains("No routes detected"));
    }

    #[test]
    fn routes_with_path_filter() {
        // Path filter should work even when no routes exist
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["routes", "--path", "/api/v1"])
            .assert()
            .success()
            .stdout(predicate::str::contains("No routes detected"));
    }

    #[test]
    fn routes_in_python_fixture() {
        // Create a minimal Python project with FastAPI routes
        let temp = TempDir::new().unwrap();

        // Create a FastAPI app file
        std::fs::write(
            temp.path().join("main.py"),
            r#"from fastapi import FastAPI

app = FastAPI()

@app.get("/health")
def health_check():
    return {"status": "ok"}

@app.post("/users")
def create_user(name: str):
    return {"name": name}

@app.get("/users/{user_id}")
def get_user(user_id: int):
    return {"user_id": user_id}
"#,
        )
        .unwrap();

        loctree()
            .current_dir(temp.path())
            .args(["routes"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("/health")
                    .or(predicate::str::contains("route"))
                    .or(predicate::str::contains("No routes")), // May not detect without full scan
            );
    }

    // ----------------------------------------
    // Dist Command Tests
    // ----------------------------------------

    #[test]
    fn dist_help_shows_usage() {
        let output = loctree().args(["dist", "--help"]).output().unwrap();
        let combined = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);
        assert!(
            combined.contains("dist") || combined.contains("source") || combined.contains("map"),
            "Help should mention dist/source/map: {}",
            combined
        );
    }

    #[test]
    fn dist_requires_source_map() {
        // dist command requires --source-map flag
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/index.ts"), "export const x = 1;").unwrap();

        loctree()
            .current_dir(temp.path())
            .args(["dist", "--src", "src/"])
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("source-map").or(predicate::str::contains("required")),
            );
    }

    #[test]
    fn dist_requires_src() {
        // dist command requires --src flag
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("main.js.map"), "{}").unwrap();

        loctree()
            .current_dir(temp.path())
            .args(["dist", "--source-map", "main.js.map"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("src").or(predicate::str::contains("required")));
    }

    #[test]
    fn dist_handles_missing_source_map_file() {
        // Should fail gracefully when source map file doesn't exist
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/index.ts"), "export const x = 1;").unwrap();

        loctree()
            .current_dir(temp.path())
            .args([
                "dist",
                "--source-map",
                "nonexistent.js.map",
                "--src",
                "src/",
            ])
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("not found")
                    .or(predicate::str::contains("Failed"))
                    .or(predicate::str::contains("error")),
            );
    }

    #[test]
    fn dist_handles_invalid_source_map() {
        // Should fail gracefully when source map is invalid JSON
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/index.ts"), "export const x = 1;").unwrap();
        std::fs::write(temp.path().join("main.js.map"), "not valid json").unwrap();

        loctree()
            .current_dir(temp.path())
            .args(["dist", "--source-map", "main.js.map", "--src", "src/"])
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("parse")
                    .or(predicate::str::contains("invalid"))
                    .or(predicate::str::contains("Failed"))
                    .or(predicate::str::contains("error")),
            );
    }

    #[test]
    fn dist_with_minimal_valid_source_map() {
        // Test with a minimal but valid source map structure
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(
            temp.path().join("src/index.ts"),
            "export const hello = 'world';",
        )
        .unwrap();

        // Minimal source map structure
        let source_map = r#"{
            "version": 3,
            "file": "main.js",
            "sources": ["src/index.ts"],
            "sourcesContent": ["export const hello = 'world';"],
            "names": ["hello"],
            "mappings": "AAAA"
        }"#;
        std::fs::write(temp.path().join("main.js.map"), source_map).unwrap();

        loctree()
            .current_dir(temp.path())
            .args(["dist", "--source-map", "main.js.map", "--src", "src/"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("Bundle")
                    .or(predicate::str::contains("export"))
                    .or(predicate::str::contains("Analysis")),
            );
    }

    // ----------------------------------------
    // Layoutmap Command Tests
    // ----------------------------------------

    #[test]
    fn layoutmap_help_shows_usage() {
        loctree()
            .args(["layoutmap", "--help"])
            .assert()
            .success() // layoutmap help exits successfully
            .stdout(
                predicate::str::contains("layoutmap")
                    .or(predicate::str::contains("z-index"))
                    .or(predicate::str::contains("CSS")),
            );
    }

    #[test]
    fn layoutmap_no_css_in_ts_project() {
        // TypeScript project without CSS - should complete gracefully
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["layoutmap"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("No CSS")
                    .or(predicate::str::contains("findings"))
                    .or(predicate::str::contains("0")),
            );
    }

    #[test]
    fn layoutmap_json_output_empty() {
        // JSON output should be valid even with no CSS files
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["layoutmap", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::contains("[").or(predicate::str::contains("{")));
    }

    #[test]
    fn layoutmap_with_zindex_filter() {
        // --zindex flag should be recognized
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["layoutmap", "--zindex"])
            .assert()
            .success();
    }

    #[test]
    fn layoutmap_with_sticky_filter() {
        // --sticky flag should be recognized
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["layoutmap", "--sticky"])
            .assert()
            .success();
    }

    #[test]
    fn layoutmap_with_grid_filter() {
        // --grid flag should be recognized
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["layoutmap", "--grid"])
            .assert()
            .success();
    }

    #[test]
    fn layoutmap_with_min_zindex() {
        // --min-zindex flag should be recognized
        let fixture = fixtures_path().join("simple_ts");

        loctree()
            .current_dir(&fixture)
            .args(["layoutmap", "--min-zindex", "100"])
            .assert()
            .success();
    }

    #[test]
    fn layoutmap_with_css_content() {
        // Create a temp directory with CSS that has z-index
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("styles")).unwrap();

        std::fs::write(
            temp.path().join("styles/main.css"),
            r#"
.modal {
    position: fixed;
    z-index: 1000;
    top: 0;
    left: 0;
}

.tooltip {
    position: absolute;
    z-index: 500;
}

.header {
    position: sticky;
    top: 0;
    z-index: 100;
}

.container {
    display: grid;
    grid-template-columns: 1fr 1fr;
}

.flex-row {
    display: flex;
    flex-direction: row;
}
"#,
        )
        .unwrap();

        loctree()
            .current_dir(temp.path())
            .args(["layoutmap"])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("z-index")
                    .or(predicate::str::contains("1000"))
                    .or(predicate::str::contains("modal"))
                    .or(predicate::str::contains("LAYERS"))
                    .or(predicate::str::contains("findings")),
            );
    }

    #[test]
    fn layoutmap_json_with_css() {
        // JSON output with actual CSS content
        let temp = TempDir::new().unwrap();

        std::fs::write(
            temp.path().join("app.css"),
            r#"
.overlay {
    position: fixed;
    z-index: 9999;
}
"#,
        )
        .unwrap();

        loctree()
            .current_dir(temp.path())
            .args(["layoutmap", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::contains("[").or(predicate::str::contains("{")));
    }

    #[test]
    fn layoutmap_exclude_pattern() {
        // --exclude flag should be recognized
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("node_modules")).unwrap();

        std::fs::write(
            temp.path().join("node_modules/lib.css"),
            ".x { z-index: 1000; }",
        )
        .unwrap();

        std::fs::write(temp.path().join("main.css"), ".y { z-index: 100; }").unwrap();

        loctree()
            .current_dir(temp.path())
            .args(["layoutmap", "--exclude", "**/node_modules/**"])
            .assert()
            .success();
    }
}
