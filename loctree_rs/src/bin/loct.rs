use std::any::Any;
use std::fs;
use std::io::{Write, stderr};
use std::panic;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use loctree::args::{self, parse_args};
use loctree::cli::{self, Command, DispatchResult};
use loctree::config::LoctreeConfig;
use loctree::types::{GitSubcommand, Mode};
use loctree::{OutputMode, analyzer, detect, diff, fs_utils, git, slicer, snapshot, tree};

/// Micro-animation for loct startup
/// Subtle flex - blink and you'll miss it, but those who see it know.
fn print_animated_banner() {
    // Skip in non-TTY (CI, pipes)
    if !console::Term::stderr().is_term() {
        return;
    }

    const RESET: &str = "\x1b[0m";
    const BOLD: &str = "\x1b[1m";
    const DIM: &str = "\x1b[2m";
    const CYAN: &str = "\x1b[96m";
    const WHITE: &str = "\x1b[97m";

    // Phase 1: Letters materialize one by one with glow
    let letters = ['l', 'o', 'c', 't'];
    eprint!("\r");

    for (i, ch) in letters.iter().enumerate() {
        // Brief glow (white) then settle (cyan)
        eprint!("{}{}{}", BOLD, WHITE, ch);
        let _ = stderr().flush();
        thread::sleep(Duration::from_millis(35));

        // Rewrite settled letters + current
        eprint!("\r{}{}", BOLD, CYAN);
        for c in &letters[..=i] {
            eprint!("{}", c);
        }
        let _ = stderr().flush();
        thread::sleep(Duration::from_millis(25));
    }

    // Phase 2: Dot pulse (the magic touch)
    let dots = ["", ".", "..", "...", "..", ".", ""];
    for dot in dots {
        eprint!("\r{}{}loct{}{}", BOLD, CYAN, DIM, dot);
        // Pad to clear previous dots
        eprint!("   ");
        eprint!("\r{}{}loct{}{}", BOLD, CYAN, DIM, dot);
        let _ = stderr().flush();
        thread::sleep(Duration::from_millis(40));
    }

    // Phase 3: Final form - subtle tagline
    eprint!("\r{}{}loct{} ▸{}", BOLD, CYAN, RESET, RESET);
    eprintln!();
}

fn install_broken_pipe_handler() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let is_broken = <dyn Any>::downcast_ref::<&str>(payload)
            .is_some_and(|s| s.contains("Broken pipe"))
            || <dyn Any>::downcast_ref::<String>(payload)
                .is_some_and(|s| s.contains("Broken pipe"));

        if is_broken {
            // Quietly exit when downstream closes the pipe (e.g. piping to `head`).
            std::process::exit(0);
        }

        default_hook(info);
    }));
}

fn format_usage() -> &'static str {
    "loct - Static Analysis for AI Agents (v0.8.x)\n\n\
PHILOSOPHY: Scan the WHOLE repo once with `loct auto`, then query with subcommands.\n\
            Artifacts live in .loctree/ - use them for fast, context-aware analysis.\n\n\
Quick Start:\n  \
  loct auto                      Full scan → .loctree/ artifacts\n  \
  loct slice src/foo.ts          Extract file context for AI agent\n  \
  loct report --html out.html    Generate visual HTML report\n\n\
Core Commands:\n  \
  auto              Full scan + findings (creates .loctree/)\n  \
  doctor            Interactive diagnostics and quick-wins\n  \
  slice <file>      Extract file + dependencies + consumers\n  \
  find <name>       Find symbol definitions\n  \
  trace <handler>   Debug Tauri handler pipeline\n  \
  --for-ai          AI-optimized project summary (JSON)\n\n\
Analysis:\n  \
  dead              Find unused exports (dead code)\n  \
  cycles            Find circular imports\n  \
  twins             Find duplicate symbol names\n  \
  health            Quick structural health check\n  \
  crowds            Find hub files (high import/export counts)\n\n\
Output:\n  \
  report            Generate HTML/JSON/SARIF reports\n  \
  --json            Machine-readable output\n  \
  --sarif           SARIF for GitHub Code Scanning\n\n\
Common:\n  \
  -g, --gitignore   Respect .gitignore\n  \
  --verbose         Detailed progress\n  \
  --help-full       Complete command reference\n\n\
Examples:\n  \
  loct auto                                  # Full analysis\n  \
  loct slice src/main.rs --consumers         # Context for AI\n  \
  loct dead --confidence high                # Find dead code\n  \
  loct report --html out.html --serve        # Interactive report\n  \
  loct doctor                                # Interactive fixes\n\n\
Tip: Run `loct auto` from repo root first, then query!\n\n\
More: loct --help-full\n"
}

/// Full help is now auto-generated from Command::format_help_full()
/// This wrapper exists for backwards compatibility
fn format_usage_full() -> String {
    cli::Command::format_help_full()
}

fn main() -> std::io::Result<()> {
    install_broken_pipe_handler();

    // Get raw args for the new parser
    // nosemgrep: rust.lang.security.args.args
    // SECURITY: args() is used only for CLI flag parsing (paths, --json, etc.),
    // not for security decisions. The executable path (args[0]) is skipped.
    let raw_args: Vec<String> = std::env::args().skip(1).collect();

    // Preserve legacy full help output expected by CI/tests
    if raw_args.iter().any(|a| a == "--help-full") {
        println!("{}", format_usage_full());
        return Ok(());
    }

    // Try new subcommand parser first
    let mut parsed = match cli::parse_command(&raw_args) {
        Ok(Some(parsed_cmd)) => {
            // New syntax detected - dispatch through new system
            match cli::dispatch_command(&parsed_cmd) {
                DispatchResult::ShowHelp => {
                    println!("{}", Command::format_help());
                    return Ok(());
                }
                DispatchResult::ShowLegacyHelp => {
                    println!("{}", Command::format_legacy_help());
                    return Ok(());
                }
                DispatchResult::ShowVersion => {
                    println!("loctree {}", env!("CARGO_PKG_VERSION"));
                    return Ok(());
                }
                DispatchResult::Exit(code) => {
                    std::process::exit(code);
                }
                DispatchResult::Continue(args) => *args,
            }
        }
        Ok(None) => {
            // Legacy syntax - fall back to old parser
            match parse_args() {
                Ok(args) => args,
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            }
        }
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    // Auto-detect stack if no explicit extensions provided
    if !parsed.root_list.is_empty() {
        let mut library_mode = parsed.library_mode; // Preserve user-set library_mode flag
        detect::apply_detected_stack(
            &parsed.root_list[0],
            &mut parsed.extensions,
            &mut parsed.ignore_patterns,
            &mut parsed.tauri_preset,
            &mut library_mode,
            &mut parsed.py_roots,
            parsed.verbose,
        );
        parsed.library_mode = library_mode; // Apply auto-detected library mode

        // Load .loctreeignore from root (if exists)
        let loctreeignore_patterns = fs_utils::load_loctreeignore(&parsed.root_list[0]);
        if !loctreeignore_patterns.is_empty() {
            if parsed.verbose {
                eprintln!(
                    "[loctree] loaded {} patterns from .loctignore",
                    loctreeignore_patterns.len()
                );
            }
            parsed.ignore_patterns.extend(loctreeignore_patterns);
        }
    }

    // Handle help/version for legacy path (new path handles these above)
    if parsed.show_help {
        println!("{}", format_usage());
        return Ok(());
    }

    if parsed.show_help_full {
        println!("{}", format_usage_full());
        return Ok(());
    }

    if parsed.show_version {
        println!("loctree {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if parsed.max_depth.is_some() && parsed.max_depth.unwrap_or(0) == usize::MAX {
        eprintln!("Invalid max depth");
        std::process::exit(1);
    }

    let mut root_list: Vec<PathBuf> = Vec::new();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for root in parsed.root_list.iter() {
        if !root.is_dir() {
            let raw = if root.as_os_str().is_empty() {
                "<empty>".to_string()
            } else {
                root.display().to_string()
            };
            eprintln!(
                "Root \"{}\" (cwd: {}) is not a directory",
                raw,
                cwd.display()
            );
            std::process::exit(1);
        }
        root_list.push(root.canonicalize().unwrap_or_else(|_| root.clone()));
    }

    match parsed.mode {
        Mode::AnalyzeImports => analyzer::run_import_analyzer(&root_list, &parsed)?,
        Mode::Tree => tree::run_tree(&root_list, &parsed)?,
        Mode::Init => {
            print_animated_banner();
            snapshot::run_init(&root_list, &parsed)?
        }
        Mode::Slice => {
            let target = parsed.slice_target.as_ref().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "slice requires a target file path, e.g.: loctree slice src/foo.ts",
                )
            })?;
            let root = root_list
                .first()
                .cloned()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

            // Check if target is a directory
            let target_path = root.join(target);
            if target_path.is_dir() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "'{}' is a directory. slice works on files.\nFor directories use: loct focus {}",
                        target, target
                    ),
                ));
            }

            let json_output = matches!(parsed.output, OutputMode::Json);
            slicer::run_slice(&root, target, parsed.slice_consumers, json_output, &parsed)?;
        }
        Mode::Trace => {
            let handler_name = parsed.trace_handler.as_ref().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "trace requires a handler name, e.g.: loctree trace toggle_assistant",
                )
            })?;
            run_trace(&root_list, handler_name, &parsed)?;
        }
        Mode::ForAi => {
            run_for_ai(&root_list, &parsed)?;
        }
        Mode::Findings => {
            run_findings(&root_list, &parsed, false)?;
        }
        Mode::Summary => {
            run_findings(&root_list, &parsed, true)?;
        }
        Mode::Git(ref subcommand) => {
            run_git(subcommand, &parsed)?;
        }
        Mode::Search => {
            let query = parsed.search_query.as_ref().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "search requires a query, e.g.: loctree search my_function",
                )
            })?;
            run_search(&root_list, query, &parsed)?;
        }
    }

    Ok(())
}

fn run_trace(
    root_list: &[PathBuf],
    handler_name: &str,
    parsed: &args::ParsedArgs,
) -> std::io::Result<()> {
    use analyzer::root_scan::{ScanConfig, ScanResults, scan_roots};
    use analyzer::trace::{print_trace_human, print_trace_json, trace_handler};
    use std::collections::HashSet;

    // Prepare scan config - reuse logic from runner
    let extensions = parsed.extensions.clone().or_else(|| {
        Some(
            ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "css", "py"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        )
    });

    let py_stdlib = analyzer::scan::python_stdlib();

    // Load custom Tauri command macros from .loctree/config.toml
    let loctree_config = root_list
        .first()
        .map(|root| LoctreeConfig::load(root))
        .unwrap_or_default();
    let custom_command_macros = loctree_config.tauri.command_macros;
    let command_detection = analyzer::ast_js::CommandDetectionConfig::new(
        &loctree_config.tauri.dom_exclusions,
        &loctree_config.tauri.non_invoke_exclusions,
        &loctree_config.tauri.invalid_command_names,
    );

    let scan_results = scan_roots(ScanConfig {
        roots: root_list,
        parsed,
        extensions,
        focus_set: &None,
        exclude_set: &None,
        ignore_exact: HashSet::new(),
        ignore_prefixes: Vec::new(),
        py_stdlib: &py_stdlib,
        cached_analyses: None,
        collect_edges: false,
        custom_command_macros: &custom_command_macros,
        command_detection,
    })?;

    let ScanResults {
        global_fe_commands,
        global_be_commands,
        global_analyses,
        ..
    } = scan_results;

    // Get registered handlers
    let registered_impls: HashSet<String> = global_analyses
        .iter()
        .flat_map(|a| a.tauri_registered_handlers.iter().cloned())
        .collect();

    let result = trace_handler(
        handler_name,
        &global_analyses,
        &global_fe_commands,
        &global_be_commands,
        &registered_impls,
    );

    if matches!(parsed.output, OutputMode::Json) {
        print_trace_json(&result);
    } else {
        print_trace_human(&result);
    }

    Ok(())
}

fn run_for_ai(root_list: &[PathBuf], parsed: &args::ParsedArgs) -> std::io::Result<()> {
    use analyzer::coverage::{compute_command_gaps_with_confidence, compute_unregistered_handlers};
    use analyzer::for_ai::{generate_for_ai_report, print_for_ai_json};
    use analyzer::output::process_root_context;
    use analyzer::root_scan::{ScanConfig, ScanResults, scan_roots};
    use analyzer::scan::{opt_globset, python_stdlib};
    use std::collections::HashSet;

    let extensions = parsed.extensions.clone().or_else(|| {
        Some(
            ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "css", "py"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        )
    });

    let py_stdlib = python_stdlib();
    let focus_set = opt_globset(&parsed.focus_patterns);
    let exclude_set = opt_globset(&parsed.exclude_report_patterns);

    // Load custom Tauri command macros from .loctree/config.toml
    let loctree_config = root_list
        .first()
        .map(|root| LoctreeConfig::load(root))
        .unwrap_or_default();
    let custom_command_macros = loctree_config.tauri.command_macros;
    let command_detection = analyzer::ast_js::CommandDetectionConfig::new(
        &loctree_config.tauri.dom_exclusions,
        &loctree_config.tauri.non_invoke_exclusions,
        &loctree_config.tauri.invalid_command_names,
    );

    let scan_results = scan_roots(ScanConfig {
        roots: root_list,
        parsed,
        extensions,
        focus_set: &focus_set,
        exclude_set: &exclude_set,
        ignore_exact: HashSet::new(),
        ignore_prefixes: Vec::new(),
        py_stdlib: &py_stdlib,
        cached_analyses: None,
        collect_edges: true, // Need edges for hub files
        custom_command_macros: &custom_command_macros,
        command_detection,
    })?;

    let ScanResults {
        contexts,
        global_fe_commands,
        global_be_commands,
        global_analyses,
        ..
    } = scan_results;

    // Get registered handlers
    let registered_impls: HashSet<String> = global_analyses
        .iter()
        .flat_map(|a| a.tauri_registered_handlers.iter().cloned())
        .collect();

    // Filter BE commands to registered only
    let mut global_be_registered: analyzer::coverage::CommandUsage =
        std::collections::HashMap::new();
    for (name, locs) in &global_be_commands {
        for (path, line, impl_name) in locs {
            if registered_impls.is_empty() || registered_impls.contains(impl_name) {
                global_be_registered.entry(name.clone()).or_default().push((
                    path.clone(),
                    *line,
                    impl_name.clone(),
                ));
            }
        }
    }

    // Compute gaps
    let (global_missing, global_unused) = compute_command_gaps_with_confidence(
        &global_fe_commands,
        &global_be_registered,
        &focus_set,
        &exclude_set,
        &global_analyses,
    );

    let global_unregistered = compute_unregistered_handlers(
        &global_be_commands,
        &registered_impls,
        &focus_set,
        &exclude_set,
    );

    // Build report sections
    let pipeline_summary = analyzer::pipelines::build_pipeline_summary(
        &global_analyses,
        &focus_set,
        &exclude_set,
        &global_fe_commands,
        &global_be_commands,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
    );
    let git_ctx = snapshot::Snapshot::current_git_context();

    let mut report_sections = Vec::new();
    for (idx, ctx) in contexts.into_iter().enumerate() {
        let artifacts = process_root_context(
            idx,
            ctx,
            parsed,
            &global_fe_commands,
            &global_be_commands,
            &global_missing,
            &global_unregistered,
            &global_unused,
            &pipeline_summary,
            Some(&git_ctx),
            "loctree-json",
            "1.2.0",
            &global_analyses,
        );
        if let Some(section) = artifacts.report_section {
            report_sections.push(section);
        }
    }

    // Generate AI report
    let project_root = root_list
        .first()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| ".".to_string());

    let report = generate_for_ai_report(&project_root, &report_sections, &global_analyses);

    // Persist agent bundle to disk for single-file consumption
    if parsed.output == OutputMode::Json
        && let Some(root) = root_list.first()
    {
        let agent_path = root.join(".loctree").join("agent.json");
        if let Some(dir) = agent_path.parent() {
            if let Err(e) = fs::create_dir_all(dir) {
                eprintln!("[loct][agent] Failed to create {}: {}", dir.display(), e);
            } else {
                match serde_json::to_vec_pretty(&report) {
                    Ok(data) => {
                        if let Err(e) = fs::write(&agent_path, data) {
                            eprintln!(
                                "[loct][agent] Failed to write {}: {}",
                                agent_path.display(),
                                e
                            );
                        } else {
                            eprintln!("[loct][agent] Bundle saved to {}", agent_path.display());
                        }
                    }
                    Err(e) => eprintln!("[loct][agent] Failed to serialize agent bundle: {e}"),
                }
            }
        }
    }

    // JSONL mode outputs one QuickWin per line for streaming agent consumption
    if parsed.output == OutputMode::Jsonl {
        analyzer::for_ai::print_agent_feed_jsonl(&report);
    } else {
        print_for_ai_json(&report);
    }

    Ok(())
}

/// Output findings.json or summary to stdout
fn run_findings(
    root_list: &[PathBuf],
    parsed: &args::ParsedArgs,
    summary_only: bool,
) -> std::io::Result<()> {
    use analyzer::findings::{Findings, FindingsConfig};
    use analyzer::root_scan::{ScanConfig, scan_roots};
    use analyzer::scan::{opt_globset, python_stdlib};
    use std::collections::HashSet;

    let extensions = parsed.extensions.clone().or_else(|| {
        Some(
            ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "css", "py"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        )
    });

    let py_stdlib = python_stdlib();
    let focus_set = opt_globset(&parsed.focus_patterns);
    let exclude_set = opt_globset(&parsed.exclude_report_patterns);

    // Load custom config
    let loctree_config = root_list
        .first()
        .map(|root| LoctreeConfig::load(root))
        .unwrap_or_default();
    let custom_command_macros = loctree_config.tauri.command_macros;
    let command_detection = analyzer::ast_js::CommandDetectionConfig::new(
        &loctree_config.tauri.dom_exclusions,
        &loctree_config.tauri.non_invoke_exclusions,
        &loctree_config.tauri.invalid_command_names,
    );

    // Scan the project
    let scan_results = scan_roots(ScanConfig {
        roots: root_list,
        parsed,
        extensions,
        focus_set: &focus_set,
        exclude_set: &exclude_set,
        ignore_exact: HashSet::new(),
        ignore_prefixes: Vec::new(),
        py_stdlib: &py_stdlib,
        cached_analyses: None,
        collect_edges: true,
        custom_command_macros: &custom_command_macros,
        command_detection,
    })?;

    // Create a minimal snapshot for barrel chaos analysis
    let snap = snapshot::Snapshot::new(root_list.iter().map(|p| p.display().to_string()).collect());

    // Produce findings
    let config = FindingsConfig {
        high_confidence: parsed.dead_confidence.as_deref() == Some("high"),
        library_mode: parsed.library_mode,
        python_library: parsed.python_library,
        example_globs: parsed.library_example_globs.clone(),
    };

    let findings = Findings::produce(&scan_results, &snap, config);

    // Output to stdout
    if summary_only {
        let summary = findings.summary_only();
        let json = serde_json::to_string_pretty(&summary)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        println!("{}", json);
    } else {
        let json = findings
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        println!("{}", json);
    }

    Ok(())
}

/// Unified search - aggregates symbol, semantic, and dead code results
fn run_search(
    root_list: &[PathBuf],
    query: &str,
    parsed: &args::ParsedArgs,
) -> std::io::Result<()> {
    use analyzer::search::{print_search_results, run_search as do_search};

    // Try to load snapshot first for zero-latency search
    let default_root = PathBuf::from(".");
    let root = root_list.first().unwrap_or(&default_root);
    let analyses = match snapshot::Snapshot::load(root) {
        Ok(snap) => {
            // Use snapshot - instant search!
            snap.files
        }
        Err(_) => {
            // No snapshot - fall back to full scan
            use analyzer::root_scan::{ScanConfig, ScanResults, scan_roots};
            use analyzer::scan::python_stdlib;
            use std::collections::HashSet;

            eprintln!(
                "[loct][find] No snapshot found, scanning (run `loct scan` for faster searches)"
            );

            let extensions = parsed.extensions.clone().or_else(|| {
                Some(
                    ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "css", "py"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                )
            });

            let py_stdlib = python_stdlib();
            let loctree_config = root_list
                .first()
                .map(|r| LoctreeConfig::load(r))
                .unwrap_or_default();
            let custom_command_macros = loctree_config.tauri.command_macros;
            let command_detection = analyzer::ast_js::CommandDetectionConfig::new(
                &loctree_config.tauri.dom_exclusions,
                &loctree_config.tauri.non_invoke_exclusions,
                &loctree_config.tauri.invalid_command_names,
            );

            let scan_results = scan_roots(ScanConfig {
                roots: root_list,
                parsed,
                extensions,
                focus_set: &None,
                exclude_set: &None,
                ignore_exact: HashSet::new(),
                ignore_prefixes: Vec::new(),
                py_stdlib: &py_stdlib,
                cached_analyses: None,
                collect_edges: false,
                custom_command_macros: &custom_command_macros,
                command_detection,
            })?;

            let ScanResults {
                global_analyses, ..
            } = scan_results;
            global_analyses
        }
    };

    let results = do_search(query, &analyses);
    print_search_results(
        &results,
        parsed.output,
        parsed.search_symbol_only,
        parsed.search_dead_only,
        parsed.search_semantic_only,
    );

    Ok(())
}

/// Handle git subcommands for temporal awareness
fn run_git(subcommand: &GitSubcommand, parsed: &args::ParsedArgs) -> std::io::Result<()> {
    // Discover git repository from current directory
    let cwd = std::env::current_dir()?;
    let repo = git::GitRepo::discover(&cwd)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()))?;

    match subcommand {
        GitSubcommand::Compare { from, to } => run_git_compare(&repo, from, to.as_deref(), parsed),
        GitSubcommand::Blame { file } => run_git_blame(&repo, file, parsed),
        GitSubcommand::History {
            symbol,
            file,
            limit,
        } => run_git_history(&repo, symbol.as_deref(), file.as_deref(), *limit, parsed),
        GitSubcommand::WhenIntroduced {
            circular,
            dead,
            import,
        } => run_git_when_introduced(
            &repo,
            circular.as_deref(),
            dead.as_deref(),
            import.as_deref(),
            parsed,
        ),
    }
}

/// Compare snapshots between two commits
fn run_git_compare(
    repo: &git::GitRepo,
    from: &str,
    to: Option<&str>,
    _parsed: &args::ParsedArgs,
) -> std::io::Result<()> {
    // Get commit info
    let from_commit = repo
        .get_commit_info(from)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()))?;

    let to_commit = if let Some(to_ref) = to {
        Some(
            repo.get_commit_info(to_ref)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()))?,
        )
    } else {
        None // Working tree
    };

    // Get changed files between commits
    let to_ref = to.unwrap_or("HEAD");
    let changed_files = repo
        .changed_files(from, to_ref)
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    // Try to load existing snapshot from .loctree/snapshot.json
    let repo_path = repo.path().to_path_buf();
    let current_snapshot = match snapshot::Snapshot::load(&repo_path) {
        Ok(snap) => snap,
        Err(_) => {
            // No snapshot exists, create an empty one
            // Suggest running `loctree init` first
            eprintln!("Warning: No snapshot found. Run 'loctree init' first for full analysis.");
            eprintln!("Showing file-level changes only.");
            snapshot::Snapshot::new(vec![repo_path.display().to_string()])
        }
    };

    // For MVP: Use the same snapshot for both from and to
    // This means graph/export diffs will be empty, but file changes will be shown
    // TODO: In future, checkout commits to temp worktrees and scan them
    let from_snapshot = current_snapshot.clone();
    let to_snapshot = current_snapshot;

    // Compare snapshots
    let snapshot_diff = diff::SnapshotDiff::compare(
        &from_snapshot,
        &to_snapshot,
        Some(from_commit),
        to_commit,
        &changed_files,
    );

    // Output as JSON (agent-first design)
    let json = serde_json::to_string_pretty(&snapshot_diff)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    println!("{}", json);

    Ok(())
}

/// Symbol-level blame for a file
fn run_git_blame(
    _repo: &git::GitRepo,
    file: &str,
    _parsed: &args::ParsedArgs,
) -> std::io::Result<()> {
    // TODO: Implement symbol-level blame
    // For now, return a placeholder response
    let response = serde_json::json!({
        "status": "not_implemented",
        "message": "git blame is planned for Phase 2",
        "file": file,
        "hint": "Use 'loctree git compare' for snapshot comparison"
    });
    println!("{}", serde_json::to_string_pretty(&response).unwrap());
    Ok(())
}

/// Track symbol or file history
fn run_git_history(
    _repo: &git::GitRepo,
    symbol: Option<&str>,
    file: Option<&str>,
    limit: usize,
    _parsed: &args::ParsedArgs,
) -> std::io::Result<()> {
    // TODO: Implement symbol/file history tracking
    let response = serde_json::json!({
        "status": "not_implemented",
        "message": "git history is planned for Phase 3",
        "symbol": symbol,
        "file": file,
        "limit": limit,
        "hint": "Use 'loctree git compare' for snapshot comparison"
    });
    println!("{}", serde_json::to_string_pretty(&response).unwrap());
    Ok(())
}

/// Find when a pattern was introduced
fn run_git_when_introduced(
    _repo: &git::GitRepo,
    circular: Option<&str>,
    dead: Option<&str>,
    import: Option<&str>,
    _parsed: &args::ParsedArgs,
) -> std::io::Result<()> {
    // TODO: Implement pattern origin finder
    let response = serde_json::json!({
        "status": "not_implemented",
        "message": "git when-introduced is planned for Phase 3",
        "circular": circular,
        "dead": dead,
        "import": import,
        "hint": "Use 'loctree git compare' for snapshot comparison"
    });
    println!("{}", serde_json::to_string_pretty(&response).unwrap());
    Ok(())
}
