//! Command enum definition for the CLI interface.
//!
//! Vibecrafted with AI Agents by VetCoders (c)2025 The Loctree Team
//! Co-Authored-By: Maciej <void@div0.space> & Klaudiusz <the1st@whoai.am>

use super::options::*;

/// The canonical command enum for the `loct <command>` interface.
///
/// Each variant maps to a handler module. This enum is the single source
/// of truth for CLI commands and backs both parser and help output.
#[derive(Debug, Clone)]
pub enum Command {
    /// Automatic full scan with stack detection (default when no command given).
    Auto(AutoOptions),

    /// Build/update snapshot for current HEAD.
    Scan(ScanOptions),

    /// Display LOC tree / structural overview.
    Tree(TreeOptions),

    /// Produce 3-layer holographic context for a path.
    Slice(SliceOptions),

    /// Search symbols/files/impact/similar.
    Find(FindOptions),

    /// Detect unused exports / dead code.
    Dead(DeadOptions),

    /// Detect circular imports / structural cycles.
    Cycles(CyclesOptions),

    /// Trace a Tauri/IPC handler end-to-end.
    Trace(TraceOptions),

    /// Show Tauri command bridges (FE <-> BE mappings).
    Commands(CommandsOptions),

    /// Show backend/web routes (FastAPI/Flask/etc.)
    Routes(RoutesOptions),

    /// Show event flow (ghost events, orphan handlers, races).
    Events(EventsOptions),

    /// Show pipeline summary (events, commands, risks).
    Pipelines(PipelinesOptions),

    /// Show AI insights summary.
    Insights(InsightsOptions),

    /// Show manifest summaries (package.json, Cargo.toml, pyproject).
    Manifests(ManifestsOptions),

    /// Snapshot metadata and project info.
    Info(InfoOptions),

    /// Structural lint/policy checks.
    Lint(LintOptions),

    /// Generate HTML/JSON reports.
    Report(ReportOptions),

    /// Show help for commands.
    Help(HelpOptions),

    /// Show version.
    Version,

    /// Query snapshot data (who-imports, where-symbol, component-of).
    Query(QueryOptions),

    /// Compare two snapshots and show delta.
    Diff(DiffOptions),

    /// Index analysis into AI memory (vector database).
    Memex(MemexOptions),

    /// Detect functional crowds (similar files clustering).
    Crowd(CrowdOptions),

    /// Unified search around a keyword - files, crowds, and dead exports.
    Tagmap(TagmapOptions),

    /// Show symbol registry and dead parrots (semantic duplicate detection).
    Twins(TwinsOptions),

    /// Manage false positive suppressions.
    Suppress(SuppressOptions),

    /// Analyze bundle distribution using source maps.
    Dist(DistOptions),

    /// Analyze test coverage gaps.
    Coverage(CoverageOptions),

    /// Sniff for code smells (twins + dead parrots + crowds).
    Sniff(SniffOptions),

    /// Query snapshot with jq-style filters (loct '.metadata').
    JqQuery(JqQueryOptions),

    /// Analyze impact of modifying/removing a file.
    Impact(ImpactCommandOptions),

    /// Focus on a directory - extract holographic context for all files.
    Focus(FocusOptions),

    /// Show import frequency heatmap - which files are core vs peripheral.
    Hotspots(HotspotsOptions),

    /// Analyze CSS layout properties (z-index, position, display).
    Layoutmap(LayoutmapOptions),

    /// Find zombie code (dead exports + orphan files + shadow exports).
    Zombie(ZombieOptions),

    /// Quick health check summary (cycles + dead + twins).
    Health(HealthOptions),

    /// Full audit - comprehensive analysis with actionable findings.
    Audit(AuditOptions),

    /// Interactive diagnostics with actionable recommendations.
    Doctor(DoctorOptions),

    /// Generate architectural refactoring plan based on module analysis.
    Plan(PlanOptions),
}

impl Default for Command {
    fn default() -> Self {
        Command::Auto(AutoOptions::default())
    }
}
