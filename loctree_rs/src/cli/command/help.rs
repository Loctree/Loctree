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
        help.push_str("loctree 0.7.0 - AI-oriented codebase analyzer\n\n");

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

        help.push_str("ADVANCED COMMANDS:\n");
        help.push_str("  For full command list, run: loct --help-full\n");
        help.push_str("  For legacy flags, run: loct --help-legacy\n");

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
