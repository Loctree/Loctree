mod analyzer;
mod args;
mod fs_utils;
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
    "loctree (Rust)\n\nUsage: loctree [root ...] [options]\n\nModes:\n  --analyze-imports, -A   Switch to import/export analyzer (duplicate exports, re-exports, dynamic imports).\n  --preset-tauri          Apply Tauri-friendly defaults (extensions + ignore-symbols preset).\n  --preset-styles         Apply CSS/Tailwind-friendly defaults (styles analyzer preset).\n  --ai                    Emit a compact, AI-friendly JSON summary (top issues only, no per-file payloads).\n\nTree mode (default):\n  --summary[=N]           Totals + top large files (N entries, default 5).\n  --loc <n>               Threshold (LOC) for large-file highlighting. Default 1000.\n  -L, --max-depth <n>     Limit recursion depth (0 = only direct children).\n  --show-hidden, -H       Include dotfiles.\n  --json                  Emit JSON instead of a tree view (single root => object, multi-root => array).\n\nAnalyzer mode (-A):\n  --ext <list>            Comma-separated extensions (default: ts,tsx,js,jsx,mjs,cjs,rs,css,py).\n  --limit <N>             Top-N duplicate exports / dynamic imports (default 8).\n  --top-dead-symbols <N>  Cap dead-symbol list (default 20). Use --skip-dead-symbols to omit entirely.\n  --skip-dead-symbols     Omit dead-symbol analysis (useful for huge repos / AI mode).\n  --ignore-symbols <l>    Comma-separated symbols to skip when counting duplicate exports.\n  --ignore-symbols-preset Presets: common | tauri (main,run,setup,__all__,test_* etc.).\n  --focus <glob[,..]>     Filter duplicate-export rows to matching globs (report/output filter only).\n  --exclude-report <glob[,..]> Drop duplicate rows whose files match these globs (e.g. **/__tests__/**).\n  --py-root <path>        Extra Python import roots (repeatable); pyproject roots are still inferred.\n  --html-report <file>    Write analyzer results to an HTML report file.\n  --graph                 Embed an import graph; report UI uses tabs + a bottom drawer for graph controls.\n  --serve                 Start a lightweight local server so report links can open files in your editor/OS handler (keeps running).\n  --serve-once            Start the server but exit after generation (no keepalive).\n  --port <n>              Optional port for --serve (default: random open port).\n  --editor <name>         Editor integration: code|cursor|windsurf|jetbrains|none (default: auto).\n  --fail-on-missing-handlers  Exit non-zero if frontend invoke calls lack backend handlers (pipeline check).\n  --fail-on-ghost-events     Exit non-zero if events are emitted with no listeners or listeners have no emitters.\n  --fail-on-races            Exit non-zero if basic listener/await race heuristics are detected.\n  --max-graph-nodes <N>   Skip graph if above this node count (safety guard).\n  --max-graph-edges <N>   Skip graph if above this edge count (safety guard).\n\nCommon:\n  -I, --ignore <path>     Ignore a folder/file (relative or absolute). Repeatable.\n  --gitignore, -g         Respect current Git ignore rules (requires git).\n  --color[=mode]          Colorize large files. mode: auto|always|never (default auto).\n  --editor-cmd <tpl>      Command template to open files (default: code -g {file}:{line}, fallback open/xdg-open).\n  --jsonl                 Emit one JSON object per line (per root) in analyzer mode.\n  --help, -h              Show this message.\n\nExamples:\n  loctree src --ext rs,ts --summary\n  loctree src packages/app src-tauri/src -I node_modules -L 2\n  loctree . --json > tree.json\n  loctree src apps/web -A --json --ext ts,tsx,css --limit 10 --serve --html-report /tmp/loctree.html\n  loctree backend -A --ext py --gitignore --limit 5 --py-root backend/src\n  loctree . -A --ext py --ignore-symbols main,run --graph --html-report /tmp/loctree.html\n  loctree src src-tauri/src -A --preset-tauri --graph --serve --html-report /tmp/loctree.html\n"
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
    }

    Ok(())
}
