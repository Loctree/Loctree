mod analyzer;
mod args;
mod fs_utils;
mod similarity;
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
    "loctree (Rust) - Scan once, slice many\n\n\
Quick start:\n  \
  loctree                   Scan current directory, write snapshot to .loctree/\n  \
  loctree src               Scan 'src' directory\n  \
  loctree -A --graph        Analyze imports + generate graph report\n\n\
Usage: loctree [root ...] [options]\n\n\
Modes:\n  \
  init (default)            Scan and save snapshot to .loctree/snapshot.json\n  \
  --analyze-imports, -A     Import/export analyzer (duplicates, dead symbols, FE↔BE coverage)\n  \
  --tree                    Directory tree view with LOC counts\n\n\
Presets (auto-configure extensions & ignores):\n  \
  --preset-tauri            Tauri stack: ts,tsx,rs + Tauri ignore-symbols\n  \
  --preset-styles           CSS/Tailwind: css,scss,ts,tsx\n  \
  --ai                      Compact AI-friendly JSON (top issues only)\n\n\
Analyzer options (-A):\n  \
  --ext <list>              Extensions (default: ts,tsx,js,jsx,mjs,cjs,rs,css,py)\n  \
  --limit <N>               Top-N duplicates (default 8)\n  \
  --html-report <file>      HTML report output\n  \
  --graph                   Embed import graph in report\n  \
  --serve                   Local server for editor integration\n  \
  --json                    JSON output\n  \
  --check <query>           Find similar existing components/symbols\n  \
  --dead                    List potentially unused exports (Janitor mode)\n  \
  --confidence <level>      Filter dead exports by confidence (normal|high)\n\n\
Pipeline checks (CI-friendly):\n  \
  --fail-on-missing-handlers   Exit 1 if FE invokes missing BE handlers\n  \
  --fail-on-ghost-events       Exit 1 if events have no listeners/emitters\n  \
  --fail-on-races              Exit 1 if listener/await races detected\n\n\
Common:\n  \
  -I, --ignore <path>       Ignore path (repeatable)\n  \
  --gitignore, -g           Respect .gitignore\n  \
  --scan-all                Include node_modules, target, .venv (normally skipped)\n  \
  --verbose                 Show detailed progress\n  \
  --help, -h                Show this message\n  \
  --version                 Show version\n\n\
Examples:\n  \
  loctree                                    # Quick snapshot of current dir\n  \
  loctree src -A --graph --html-report r.html  # Full analysis with graph\n  \
  loctree . --preset-tauri -A --serve        # Tauri project analysis\n\n\
More: loctree --help-full for all options\n"
}

fn format_usage_full() -> &'static str {
    "loctree (Rust) - Full options reference\n\n\
Usage: loctree [root ...] [options]\n\n\
Modes:\n  \
  init (default)            Scan and save snapshot to .loctree/snapshot.json\n  \
  --analyze-imports, -A     Import/export analyzer mode\n  \
  --tree                    Directory tree view with LOC counts\n\n\
Presets:\n  \
  --preset-tauri            Tauri stack defaults (ts,tsx,rs + ignore-symbols)\n  \
  --preset-styles           CSS/Tailwind defaults (css,scss,ts,tsx)\n  \
  --ai                      Compact AI-friendly JSON output\n\n\
Tree mode options:\n  \
  --summary[=N]             Show totals + top N large files (default 5)\n  \
  --loc <n>                 LOC threshold for large-file highlighting (default 1000)\n  \
  -L, --max-depth <n>       Limit recursion depth (0 = direct children only)\n  \
  --show-hidden, -H         Include dotfiles\n  \
  --json                    JSON output instead of tree view\n\n\
Analyzer mode options (-A):\n  \
  --ext <list>              Comma-separated extensions (default: ts,tsx,js,jsx,mjs,cjs,rs,css,py)\n  \
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
  --serve                   Start local server for editor integration\n  \
  --serve-once              Start server, exit after report generation\n  \
  --port <n>                Port for --serve (default: random)\n  \
  --editor <name>           Editor: code|cursor|windsurf|jetbrains|none (default: auto)\n  \
  --json                    JSON output\n  \
  --jsonl                   JSON Lines output (one object per line)\n\n\
Pipeline checks (CI):\n  \
  --fail-on-missing-handlers   Exit 1 if FE invokes lack BE handlers\n  \
  --fail-on-ghost-events       Exit 1 if events lack listeners/emitters\n  \
  --fail-on-races              Exit 1 if listener/await races detected\n\n\
Graph limits:\n  \
  --max-graph-nodes <N>     Skip graph if above node count\n  \
  --max-graph-edges <N>     Skip graph if above edge count\n\n\
Common:\n  \
  -I, --ignore <path>       Ignore path (repeatable)\n  \
  --gitignore, -g           Respect .gitignore rules\n  \
  --scan-all                Include node_modules, target, .venv, __pycache__ (normally skipped)\n  \
  --color[=mode]            Colorize output: auto|always|never (default auto)\n  \
  --editor-cmd <tpl>        Command template for opening files\n  \
  --verbose                 Show detailed progress and warnings\n  \
  --help, -h                Show quick help\n  \
  --help-full               Show this full reference\n  \
  --version                 Show version\n\n\
Examples:\n  \
  loctree                                        # Snapshot current dir\n  \
  loctree src --tree --summary                   # Tree view with summary\n  \
  loctree src -A --graph --html-report r.html    # Full analysis\n  \
  loctree . --preset-tauri -A --serve            # Tauri project\n  \
  loctree backend -A --ext py --py-root src      # Python project\n"
}

fn main() -> std::io::Result<()> {
    install_broken_pipe_handler();

    let parsed = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

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
    }

    Ok(())
}
