//! Help text generation for CLI commands.
//!
//! Created by M&K (c)2025 The LibraxisAI Team
//! Co-Authored-By: Maciej <void@div0.space> & Klaudiusz <the1st@whoai.am>

use super::help_texts::*;
use super::types::Command;

impl Command {
    /// Get the command name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            Command::Auto(_) => "auto",
            Command::Scan(_) => "scan",
            Command::Tree(_) => "tree",
            Command::Slice(_) => "slice",
            Command::Find(_) => "find",
            Command::Dead(_) => "dead",
            Command::Cycles(_) => "cycles",
            Command::Trace(_) => "trace",
            Command::Commands(_) => "commands",
            Command::Routes(_) => "routes",
            Command::Events(_) => "events",
            Command::Info(_) => "info",
            Command::Lint(_) => "lint",
            Command::Report(_) => "report",
            Command::Help(_) => "help",
            Command::Version => "version",
            Command::Query(_) => "query",
            Command::Diff(_) => "diff",
            Command::Memex(_) => "memex",
            Command::Crowd(_) => "crowd",
            Command::Tagmap(_) => "tagmap",
            Command::Twins(_) => "twins",
            Command::Suppress(_) => "suppress",
            Command::Dist(_) => "dist",
            Command::Coverage(_) => "coverage",
            Command::Sniff(_) => "sniff",
            Command::JqQuery(_) => "jq",
            Command::Impact(_) => "impact",
            Command::Focus(_) => "focus",
            Command::Hotspots(_) => "hotspots",
            Command::Layoutmap(_) => "layoutmap",
            Command::Zombie(_) => "zombie",
            Command::Health(_) => "health",
            Command::Audit(_) => "audit",
            Command::Doctor(_) => "doctor",
        }
    }

    /// Get a short description of the command.
    pub fn description(&self) -> &'static str {
        match self {
            Command::Auto(_) => "Full auto-scan with stack detection (default)",
            Command::Scan(_) => "Build/update snapshot for current HEAD (supports --watch)",
            Command::Tree(_) => "Display LOC tree / structural overview",
            Command::Slice(_) => "Extract holographic context for a file",
            Command::Find(_) => "Search symbols/files with regex filters",
            Command::Dead(_) => "Detect unused exports / dead code",
            Command::Cycles(_) => "Detect circular imports",
            Command::Trace(_) => "Trace a Tauri/IPC handler end-to-end",
            Command::Commands(_) => "Show Tauri command bridges (FE <-> BE)",
            Command::Events(_) => "Show event flow and issues",
            Command::Info(_) => "Show snapshot metadata and project info",
            Command::Lint(_) => "Structural lint/policy checks",
            Command::Report(_) => "Generate HTML/JSON reports",
            Command::Help(_) => "Show help for commands",
            Command::Version => "Show version information",
            Command::Query(_) => "Query snapshot data (who-imports, where-symbol, component-of)",
            Command::Diff(_) => "Compare snapshots and show semantic delta",
            Command::Memex(_) => "Index analysis into AI memory (vector DB)",
            Command::Crowd(_) => "Detect functional crowds (similar files clustering)",
            Command::Tagmap(_) => "Unified search: files + crowd + dead around a keyword",
            Command::Twins(_) => "Show symbol registry and dead parrots (0 imports)",
            Command::Suppress(_) => "Manage false positive suppressions",
            Command::Routes(_) => "List backend/web routes (FastAPI/Flask)",
            Command::Dist(_) => "Analyze bundle distribution using source maps",
            Command::Coverage(_) => "Analyze test coverage gaps (structural coverage)",
            Command::Sniff(_) => "Sniff for code smells (twins + dead parrots + crowds)",
            Command::JqQuery(_) => "Query snapshot with jq-style filters (loct '.filter')",
            Command::Impact(_) => "Analyze impact of modifying/removing a file",
            Command::Focus(_) => "Extract holographic context for a directory",
            Command::Hotspots(_) => "Show import frequency heatmap (core vs peripheral)",
            Command::Layoutmap(_) => "Analyze CSS layout (z-index, position, grid/flex)",
            Command::Zombie(_) => "Find zombie code (dead exports + orphan files + shadows)",
            Command::Health(_) => "Quick health check (cycles + dead + twins summary)",
            Command::Audit(_) => "Full audit (cycles + dead + twins + zombie + crowds)",
            Command::Doctor(_) => "Interactive diagnostics with actionable recommendations",
        }
    }

    /// Generate the main help text listing the core commands.
    pub fn format_help() -> String {
        let mut help = String::new();
        help.push_str(&format!(
            "loctree {} - AI-oriented codebase analyzer\n\n",
            env!("CARGO_PKG_VERSION")
        ));

        help.push_str("SCAN (produces artifacts):\n");
        help.push_str(
            "  loct                  Analyze -> .loctree/{snapshot,findings,agent,manifest}.json\n",
        );
        help.push_str("  loct --fresh          Ignore cache, full rescan\n");
        help.push_str("  loct --watch          Continuous scan on file changes\n\n");

        help.push_str("QUERY (reads artifacts):\n");
        help.push_str("  loct slice <file>     Context for a file (deps + consumers)\n");
        help.push_str("  loct find <pattern>   Search symbols across languages\n");
        help.push_str("  loct '<jq-query>'     Query any artifact with jq\n");
        help.push_str(
            "  loct query <type>     Graph: who-imports, where-defined, component-of\n\n",
        );

        help.push_str("OUTPUT (stdout, pipe-friendly):\n");
        help.push_str("  loct --for-ai         AI context bundle\n");
        help.push_str("  loct --findings       All issues\n");
        help.push_str("  loct --summary        Health score + counts only\n\n");

        help.push_str("ALIASES:\n");
        help.push_str("  loct s <file>         = slice\n");
        help.push_str("  loct f <pattern>      = find\n");
        help.push_str("  loct h                = --summary\n\n");

        help.push_str("ARTIFACTS:\n");
        help.push_str("  .loctree/snapshot.json   Graph data (jq-able)\n");
        help.push_str("  .loctree/findings.json   All issues (dead, cycles, twins...)\n");
        help.push_str("  .loctree/agent.json      AI bundle\n");
        help.push_str("  .loctree/manifest.json   Index for tooling\n\n");

        help.push_str("Start: cat .loctree/manifest.json\n\n");

        help.push_str("GLOBAL OPTIONS:\n");
        help.push_str("    --json           Output as JSON (stdout only)\n");
        help.push_str("    --quiet          Suppress non-essential output\n");
        help.push_str("    --verbose        Show detailed progress\n");
        help.push_str("    --color <mode>   Color mode: auto|always|never\n");
        help.push_str("    --fresh          Force rescan even if snapshot exists\n");
        help.push_str("    --no-scan        Fail if no snapshot (don't auto-scan)\n");
        help.push_str("    --fail-stale     Fail if snapshot is stale (CI mode)\n");
        help.push_str("    --findings       Output findings.json to stdout\n");
        help.push_str("    --summary        Output summary only to stdout\n");
        help.push_str("    --py-root <path> Additional Python package root for imports\n");
        help.push_str("    --help           Show this help\n");
        help.push_str("    --version        Show version\n\n");

        help.push_str("ANALYSIS:\n");
        help.push_str("  loct dead             Find unused exports\n");
        help.push_str("  loct cycles           Circular imports\n");
        help.push_str("  loct twins            Dead parrots + duplicates\n");
        help.push_str("  loct health           Quick health check\n");
        help.push_str("  loct audit            Full codebase audit\n\n");

        help.push_str("INSTANT (<100ms):\n");
        help.push_str("  loct focus <dir>      Directory context\n");
        help.push_str("  loct hotspots         Import frequency heatmap\n");
        help.push_str("  loct commands         Tauri FE↔BE bridges\n");
        help.push_str("  loct coverage         Test coverage gaps\n");
        help.push_str("  loct impact <file>    What breaks if changed\n\n");

        help.push_str("MORE:\n");
        help.push_str("  loct --help-full      All 27 commands\n");
        help.push_str("  loct <cmd> --help     Per-command help\n");
        help.push_str("  loct --help-legacy    Deprecated flag migration\n");

        help
    }

    /// Generate help text for a specific subcommand (new CLI).
    pub fn format_command_help(command: &str) -> Option<&'static str> {
        match command {
            "auto" => Some(AUTO_HELP),
            "agent" => Some(AGENT_HELP),
            "scan" => Some(SCAN_HELP),
            "tree" => Some(TREE_HELP),
            "slice" => Some(SLICE_HELP),
            "find" => Some(FIND_HELP),
            "dead" | "unused" => Some(DEAD_HELP),
            "cycles" => Some(CYCLES_HELP),
            "trace" => Some(TRACE_HELP),
            "commands" => Some(COMMANDS_HELP),
            "events" => Some(EVENTS_HELP),
            "info" => Some(INFO_HELP),
            "lint" => Some(LINT_HELP),
            "report" => Some(REPORT_HELP),
            "query" => Some(QUERY_HELP),
            "impact" => Some(IMPACT_HELP),
            "diff" => Some(DIFF_HELP),
            "memex" => Some(MEMEX_HELP),
            "crowd" => Some(CROWD_HELP),
            "tagmap" => Some(TAGMAP_HELP),
            "twins" => Some(TWINS_HELP),
            "routes" => Some(ROUTES_HELP),
            "dist" => Some(DIST_HELP),
            "coverage" => Some(COVERAGE_HELP),
            "sniff" => Some(SNIFF_HELP),
            "suppress" => Some(SUPPRESS_HELP),
            "focus" => Some(FOCUS_HELP),
            "hotspots" => Some(HOTSPOTS_HELP),
            "layoutmap" => Some(LAYOUTMAP_HELP),
            "zombie" => Some(ZOMBIE_HELP),
            "health" => Some(HEALTH_HELP),
            "audit" => Some(AUDIT_HELP),
            "doctor" => Some(DOCTOR_HELP),
            "jq" => Some(JQ_HELP),
            _ => None,
        }
    }

    /// Generate the full help text with ALL commands (auto-generated).
    /// This replaces the hardcoded format_usage_full() in loct.rs.
    pub fn format_help_full() -> String {
        let mut help = String::new();
        help.push_str(&format!(
            "loctree {} - AI-oriented codebase analyzer (Full Reference)\n\n",
            env!("CARGO_PKG_VERSION")
        ));

        help.push_str("PHILOSOPHY: Scan once, query everything.\n");
        help.push_str(
            "            Run `loct` to create artifacts, then query with subcommands.\n\n",
        );

        // === INSTANT COMMANDS (< 100ms) ===
        help.push_str("=== INSTANT COMMANDS (<100ms) ===\n\n");
        let instant_cmds = [
            ("focus <dir>", "Holographic context for a directory"),
            ("hotspots", "Import frequency heatmap (core vs peripheral)"),
            ("commands", "Tauri FE↔BE handler bridges"),
            ("events", "Event emit/listen flow analysis"),
            ("coverage", "Test coverage gaps (structural)"),
            ("health", "Quick health check (cycles + dead + twins)"),
            ("slice <file>", "Context for a file (deps + consumers)"),
            ("impact <file>", "What breaks if you modify this file"),
            ("query <type>", "Graph queries (who-imports, where-symbol)"),
        ];
        for (cmd, desc) in instant_cmds {
            help.push_str(&format!("  loct {:<16} {}\n", cmd, desc));
        }
        help.push('\n');

        // === ANALYSIS COMMANDS ===
        help.push_str("=== ANALYSIS COMMANDS ===\n\n");
        let analysis_cmds = [
            ("dead", "Find unused exports / dead code"),
            ("cycles", "Detect circular import chains"),
            ("twins", "Find dead parrots (0 imports) + duplicate exports"),
            ("zombie", "Combined: dead + orphan files + shadows"),
            ("audit", "Full codebase audit (all checks)"),
            ("crowd <kw>", "Functional clustering around keyword"),
            ("tagmap <kw>", "Unified search: files + crowd + dead"),
            ("sniff", "Code smells aggregate (twins + dead + crowds)"),
        ];
        for (cmd, desc) in analysis_cmds {
            help.push_str(&format!("  loct {:<16} {}\n", cmd, desc));
        }
        help.push('\n');

        // === FRAMEWORK-SPECIFIC ===
        help.push_str("=== FRAMEWORK-SPECIFIC ===\n\n");
        let framework_cmds = [
            ("trace <handler>", "Trace Tauri handler end-to-end"),
            ("routes", "List FastAPI/Flask routes"),
            ("dist", "Bundle analysis via source maps"),
            ("layoutmap", "CSS z-index/position/grid analysis"),
        ];
        for (cmd, desc) in framework_cmds {
            help.push_str(&format!("  loct {:<16} {}\n", cmd, desc));
        }
        help.push('\n');

        // === MANAGEMENT ===
        help.push_str("=== MANAGEMENT ===\n\n");
        let mgmt_cmds = [
            ("doctor", "Interactive diagnostics with recommendations"),
            ("suppress", "Manage false positive suppressions"),
            ("diff", "Compare snapshots between branches/commits"),
            ("memex", "Index analysis into AI memory (vector DB)"),
        ];
        for (cmd, desc) in mgmt_cmds {
            help.push_str(&format!("  loct {:<16} {}\n", cmd, desc));
        }
        help.push('\n');

        // === CORE WORKFLOW ===
        help.push_str("=== CORE WORKFLOW ===\n\n");
        let core_cmds = [
            ("auto", "Full scan → .loctree/ artifacts (default)"),
            ("scan", "Build/update snapshot (supports --watch)"),
            ("tree", "Directory tree with LOC counts"),
            ("find <pattern>", "Search symbols/files with regex"),
            ("report", "Generate HTML/JSON reports"),
            ("lint", "Structural lint and policy checks"),
        ];
        for (cmd, desc) in core_cmds {
            help.push_str(&format!("  loct {:<16} {}\n", cmd, desc));
        }
        help.push('\n');

        // === JQ QUERIES ===
        help.push_str("=== JQ QUERIES ===\n\n");
        help.push_str("  loct '.metadata'              Extract metadata from snapshot\n");
        help.push_str("  loct '.files | length'        Count analyzed files\n");
        help.push_str("  loct '.dead_parrots[]'        List dead exports\n");
        help.push_str("  loct '.cycles[]'              List circular imports\n\n");

        // === GLOBAL OPTIONS ===
        help.push_str("=== GLOBAL OPTIONS ===\n\n");
        help.push_str("  --json             Output as JSON\n");
        help.push_str("  --fresh            Force rescan (ignore cache)\n");
        help.push_str("  --verbose          Detailed progress\n");
        help.push_str("  --fail             Exit non-zero on findings (CI mode)\n");
        help.push_str("  --sarif            SARIF 2.1.0 output for CI\n\n");

        // === ARTIFACTS ===
        help.push_str("=== ARTIFACTS (.loctree/) ===\n\n");
        help.push_str("  snapshot.json      Full dependency graph (jq-queryable)\n");
        help.push_str("  findings.json      All issues (dead, cycles, twins...)\n");
        help.push_str("  agent.json         AI-optimized bundle with health_score\n");
        help.push_str("  manifest.json      Index for tooling integration\n\n");

        // === PER-COMMAND HELP ===
        help.push_str("=== PER-COMMAND HELP ===\n\n");
        help.push_str("  loct <command> --help    Detailed help for any command\n");
        help.push_str("  loct --help-legacy       Legacy flag migration guide\n\n");

        // === EXAMPLES ===
        help.push_str("=== EXAMPLES ===\n\n");
        help.push_str("  # Quick analysis\n");
        help.push_str("  loct                       # Scan repo, create artifacts\n");
        help.push_str("  loct health                # Quick health check\n");
        help.push_str("  loct hotspots              # Find hub files (47ms!)\n\n");

        help.push_str("  # Deep analysis\n");
        help.push_str("  loct focus src/features/   # Directory context (67ms!)\n");
        help.push_str("  loct coverage              # Test gaps (49ms!)\n");
        help.push_str("  loct audit                 # Full audit\n\n");

        help.push_str("  # AI integration\n");
        help.push_str("  loct slice src/main.rs --json | claude\n");
        help.push_str("  loct --for-ai > context.json\n\n");

        help.push_str("  # CI integration\n");
        help.push_str("  loct lint --fail --sarif > loctree.sarif\n");
        help.push_str("  loct health --json | jq '.summary.health_score'\n");

        help
    }

    /// Generate legacy help text with migration hints.
    pub fn format_legacy_help() -> String {
        let mut help = String::new();
        help.push_str("loctree - Legacy Flag Reference\n\n");
        help.push_str("These flags are deprecated and will be removed in v1.0.\n");
        help.push_str("Please migrate to the new subcommand interface.\n\n");

        help.push_str("LEGACY FLAG              -> NEW COMMAND\n");
        help.push_str("-------------------------------------------\n");
        help.push_str("loct                     -> loct auto (unchanged)\n");
        help.push_str("loct --tree              -> loct tree\n");
        help.push_str("loct -A                  -> loct report\n");
        help.push_str("loct -A --dead           -> loct dead\n");
        help.push_str("loct -A --circular       -> loct cycles\n");
        help.push_str("loct -A --entrypoints    -> loct lint --entrypoints\n");
        help.push_str("loct -A --symbol NAME    -> loct find --symbol NAME\n");
        help.push_str("loct -A --impact FILE    -> loct find --impact FILE\n");
        help.push_str("loct --for-ai            -> loct auto --agent-json\n");
        help.push_str("loct slice PATH          -> loct slice PATH (unchanged)\n");

        help.push_str("\nFor the new command reference, run: loct --help\n");

        help
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::command::options::{
        CyclesOptions, DeadOptions, ScanOptions, SliceOptions, TreeOptions,
    };

    #[test]
    fn test_command_default_is_auto() {
        let cmd = Command::default();
        assert_eq!(cmd.name(), "auto");
    }

    #[test]
    fn test_command_names() {
        assert_eq!(Command::Scan(ScanOptions::default()).name(), "scan");
        assert_eq!(Command::Tree(TreeOptions::default()).name(), "tree");
        assert_eq!(Command::Slice(SliceOptions::default()).name(), "slice");
        assert_eq!(Command::Dead(DeadOptions::default()).name(), "dead");
        assert_eq!(Command::Cycles(CyclesOptions::default()).name(), "cycles");
    }

    #[test]
    fn test_help_format_contains_commands() {
        let help = Command::format_help();
        assert!(help.contains("slice"));
        assert!(help.contains("find"));
    }

    #[test]
    fn test_legacy_help_format_contains_mappings() {
        let help = Command::format_legacy_help();
        assert!(help.contains("--tree"));
        assert!(help.contains("-A --dead"));
        assert!(help.contains("loct dead"));
    }

    #[test]
    fn test_command_specific_help_exists() {
        let tree_help = Command::format_command_help("tree").unwrap();
        assert!(tree_help.contains("loct tree"));
        assert!(Command::format_command_help("unknown").is_none());
    }
}
