mod analyzer;
mod args;
mod detect;
mod fs_utils;
mod similarity;
mod slicer;
mod snapshot;
mod tree;
mod types;

use std::panic;
use std::path::PathBuf;

use args::parse_args;
use types::Mode;

fn install_broken_pipe_handler() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let is_broken = payload
            .downcast_ref::<&str>()
            .is_some_and(|s| s.contains("Broken pipe"))
            || payload
                .downcast_ref::<String>()
                .is_some_and(|s| s.contains("Broken pipe"));

        if is_broken {
            // Quietly exit when downstream closes the pipe (e.g. piping to `head`).
            std::process::exit(0);
        }

        default_hook(info);
    }));
}

fn format_usage() -> &'static str {
    "loctree (Rust) - AI-oriented Project Analyzer\n\n\
Quick start:\n  \
  loctree                   Scan current directory, write snapshot to .loctree/\n  \
  loctree slice <file>      Extract context for AI agents (deps + consumers)\n  \
  loctree -A --graph        Analyze imports + generate graph report\n\n\
Usage: loctree [root ...] [options]\n\n\
Modes:\n  \
  init (default)            Scan and save snapshot to .loctree/snapshot.json\n  \
  slice <file>              Holographic slice: extract core + deps + consumers for AI\n  \
  --analyze-imports, -A     Import/export analyzer (duplicates, dead symbols, FE↔BE coverage)\n  \
  --tree                    Directory tree view with LOC counts\n\n\
Slice options:\n  \
  --consumers               Include files that import the target\n  \
  --json                    Output as JSON (for piping to AI)\n\n\
Analyzer options (-A):\n  \
  --ext <list>              Extensions (default: auto-detected or ts,tsx,js,jsx,mjs,cjs,rs,css,py)\n  \
  --limit <N>               Top-N duplicates (default 8)\n  \
  --html-report <file>      HTML report output\n  \
  --graph                   Embed import graph in report\n  \
  --circular                Find circular imports\n  \
  --entrypoints             List entry points (main, __main__)\n  \
  --dead                    List potentially unused exports (Janitor mode)\n  \
  --sarif                   SARIF 2.1.0 output for CI integration\n  \
  --serve                   Local server for editor integration\n  \
  --json                    JSON output\n\n\
Pipeline checks (CI-friendly):\n  \
  --fail-on-missing-handlers   Exit 1 if FE invokes missing BE handlers\n  \
  --fail-on-ghost-events       Exit 1 if events have no listeners/emitters\n  \
  --fail-on-races              Exit 1 if listener/await races detected\n\n\
Common:\n  \
  -I, --ignore <path>       Ignore path (repeatable)\n  \
  --gitignore, -g           Respect .gitignore\n  \
  --full-scan               Ignore mtime, re-analyze all files\n  \
  --verbose                 Show detailed progress\n  \
  --help, -h                Show this message\n  \
  --version                 Show version\n\n\
Examples:\n  \
  loctree                                    # Quick snapshot of current dir\n  \
  loctree slice src/main.rs --consumers      # Extract context for AI\n  \
  loctree slice src/App.tsx --json | claude  # Pipe to Claude\n  \
  loctree -A --circular                      # Find circular imports\n  \
  loctree -A --dead --confidence high        # Find dead exports\n\n\
More: loctree --help-full for all options\n"
}

fn format_usage_full() -> &'static str {
    "loctree (Rust) - AI-oriented Project Analyzer - Full reference\n\n\
Usage: loctree [root ...] [options]\n\n\
Modes:\n  \
  init (default)            Scan and save snapshot to .loctree/snapshot.json\n  \
  slice <file>              Holographic slice: extract context for AI agents\n  \
  --analyze-imports, -A     Import/export analyzer mode\n  \
  --tree                    Directory tree view with LOC counts\n\n\
Slice mode options:\n  \
  slice <file>              Target file to extract context for\n  \
  --consumers               Include files that import the target\n  \
  --json                    Output as JSON (for piping to AI agents)\n\n\
Presets:\n  \
  --preset-tauri            Tauri stack defaults (ts,tsx,rs + ignore-symbols)\n  \
  --preset-styles           CSS/Tailwind defaults (css,scss,ts,tsx)\n  \
  --ai                      Compact AI-friendly JSON output\n\n\
Auto-detection:\n  \
  Stack is auto-detected from: Cargo.toml, tsconfig.json, pyproject.toml, vite.config.*, src-tauri/\n\n\
Tree mode options:\n  \
  --summary[=N]             Show totals + top N large files (default 5)\n  \
  --loc <n>                 LOC threshold for large-file highlighting (default 1000)\n  \
  -L, --max-depth <n>       Limit recursion depth (0 = direct children only)\n  \
  --show-hidden, -H         Include dotfiles\n  \
  --find-artifacts          Find build artifact dirs (node_modules, target, .venv, etc.)\n  \
  --json                    JSON output instead of tree view\n\n\
Analyzer mode options (-A):\n  \
  --ext <list>              Comma-separated extensions (default: auto-detected)\n  \
  --limit <N>               Top-N duplicate exports / dynamic imports (default 8)\n  \
  --top-dead-symbols <N>    Cap dead-symbol list (default 20)\n  \
  --skip-dead-symbols       Omit dead-symbol analysis entirely\n  \
  --ignore-symbols <list>   Symbols to skip in duplicate counting\n  \
  --ignore-symbols-preset <name>  Presets: common | tauri\n  \
  --focus <glob[,..]>       Filter results to matching globs\n  \
  --exclude-report <glob[,..]>  Exclude files from report (e.g. **/__tests__/**)\n  \
  --py-root <path>          Extra Python import roots (repeatable)\n  \
  --html-report <file>      Write HTML report to file\n  \
  --graph                   Embed import graph in HTML report\n  \
  --circular                Find circular imports (SCC analysis)\n  \
  --entrypoints             List entry points (main, __main__, index)\n  \
  --dead                    List potentially unused exports (Janitor mode)\n  \
  --confidence <level>      Dead exports confidence filter: normal | high\n  \
  --sarif                   SARIF 2.1.0 output for CI integration\n  \
  --symbol <name>           Search for symbol across all files\n  \
  --impact <file>           Show what files import the target\n  \
  --check <query>           Find similar existing components/symbols\n  \
  --serve                   Start local server for editor integration\n  \
  --serve-once              Start server, exit after report generation\n  \
  --port <n>                Port for --serve (default: random)\n  \
  --editor <name>           Editor: code|cursor|windsurf|jetbrains|none (default: auto)\n  \
  --json                    JSON output\n  \
  --jsonl                   JSON Lines output (one object per line)\n\n\
Pipeline checks (CI):\n  \
  --fail-on-missing-handlers   Exit 1 if FE invokes missing BE handlers\n  \
  --fail-on-ghost-events       Exit 1 if events lack listeners/emitters\n  \
  --fail-on-races              Exit 1 if listener/await races detected\n\n\
Graph limits:\n  \
  --max-graph-nodes <N>     Skip graph if above node count\n  \
  --max-graph-edges <N>     Skip graph if above edge count\n\n\
Common:\n  \
  -I, --ignore <path>       Ignore path (repeatable)\n  \
  --gitignore, -g           Respect .gitignore rules\n  \
  --scan-all                Include node_modules, target, .venv, __pycache__ (normally skipped)\n  \
  --full-scan               Ignore mtime cache, re-analyze all files\n  \
  --color[=mode]            Colorize output: auto|always|never (default auto)\n  \
  --editor-cmd <tpl>        Command template for opening files\n  \
  --verbose                 Show detailed progress and warnings\n  \
  --help, -h                Show quick help\n  \
  --help-full               Show this full reference\n  \
  --version                 Show version\n\n\
Examples:\n  \
  loctree                                        # Snapshot current dir (incremental)\n  \
  loctree slice src/main.rs --consumers --json   # Extract AI context\n  \
  loctree -A --circular                          # Find circular imports\n  \
  loctree -A --entrypoints                       # List entry points\n  \
  loctree -A --sarif > results.sarif             # CI-friendly output\n  \
  loctree src -A --graph --html-report r.html    # Full analysis\n  \
  loctree . --preset-tauri -A --serve            # Tauri project\n  \
  loctree backend -A --ext py --py-root src      # Python project\n"
}

fn main() -> std::io::Result<()> {
    install_broken_pipe_handler();

    let mut parsed = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    // Auto-detect stack if no explicit extensions provided
    if !parsed.root_list.is_empty() {
        detect::apply_detected_stack(
            &parsed.root_list[0],
            &mut parsed.extensions,
            &mut parsed.ignore_patterns,
            &mut parsed.tauri_preset,
            parsed.verbose,
        );
    }

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
        Mode::Init => snapshot::run_init(&root_list, &parsed)?,
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
            let json_output = matches!(parsed.output, types::OutputMode::Json);
            slicer::run_slice(&root, target, parsed.slice_consumers, json_output, &parsed)?;
        }
    }

    Ok(())
}
