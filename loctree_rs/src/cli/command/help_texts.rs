//! Static help text constants for CLI commands.
//!
//! Each constant provides detailed usage documentation for a specific command.
//!
//! Created by M&K (c)2025 The LibraxisAI Team
//! Co-Authored-By: Maciej <void@div0.space> & Klaudiusz <the1st@whoai.am>

pub(super) const AUTO_HELP: &str =
    "loct auto - Full auto-scan with stack detection (default command)

USAGE:
    loct auto [OPTIONS] [PATHS...]
    loct [OPTIONS] [PATHS...]    # 'auto' is the default command

DESCRIPTION:
    Performs a comprehensive analysis of your codebase:
    - Detects project type and language stack automatically
    - Builds dependency graph and import relationships
    - Analyzes code structure and exports
    - Identifies potential issues (dead code, cycles, etc.)

OPTIONS:
    --full-scan          Force full rescan (ignore cache)
    --scan-all           Scan all files including hidden/ignored
    --for-agent-feed     Output optimized format for AI agents (JSONL stream)
    --agent-json         Emit a single agent bundle JSON (alias: loct agent)
    --no-duplicates      Hide duplicate export sections in CLI output
    --no-dynamic-imports Hide dynamic import sections in CLI output
    --help, -h           Show this help message

ARGUMENTS:
    [PATHS...]           Root directories to scan (default: current directory)

EXAMPLES:
    loct                         # Auto-scan current directory
    loct auto                    # Explicit auto command
    loct auto --full-scan        # Force full rescan
    loct auto src/ lib/          # Scan specific directories
    loct --for-agent-feed        # AI-optimized output (JSONL stream)
    loct --agent-json            # One-shot agent bundle JSON";

pub(super) const AGENT_HELP: &str =
    "loct agent - Agent bundle JSON (shortcut for auto --agent-json)

USAGE:
    loct agent [PATHS...]

DESCRIPTION:
    Runs the auto scan and emits a single JSON tuned for AI agents:
    handlers, duplicates, dead exports, dynamic imports, cycles, and top files.
    The bundle is also saved to ./.loctree/agent.json for reuse.

OPTIONS:
    --full-scan          Force full rescan (ignore cache)
    --scan-all           Scan all files including hidden/ignored
    --help, -h           Show this help message

ARGUMENTS:
    [PATHS...]           Root directories to scan (default: current directory)

EXAMPLES:
    loct agent                   # Agent bundle for current directory
    loct agent src/              # Agent bundle for src/";

pub(super) const SCAN_HELP: &str = "loct scan - Build/update snapshot for current HEAD

USAGE:
    loct scan [OPTIONS] [PATHS...]

DESCRIPTION:
    Scans the codebase and updates the internal snapshot database.
    Builds the dependency graph and prepares data for other commands.
    Unlike 'auto', it only builds the snapshot without extra analysis.

OPTIONS:
    --full-scan       Force full rescan, ignore cached data
    --scan-all        Include hidden and ignored files
    --watch           Watch for changes and re-scan automatically
    --help, -h        Show this help message

ARGUMENTS:
    [PATHS...]        Root directories to scan (default: current directory)

EXAMPLES:
    loct scan                    # Scan current directory
    loct scan --full-scan        # Force complete rescan
    loct scan src/ lib/          # Scan specific directories
    loct scan --scan-all         # Include all files (even hidden)
    loct scan --watch            # Watch mode with live refresh";

pub(super) const TREE_HELP: &str = "loct tree - Display LOC tree / structural overview

USAGE:
    loct tree [OPTIONS] [PATHS...]

DESCRIPTION:
    Hierarchical tree of the codebase with LOC metrics.
    Similar to 'tree' but with LOC and gitignore handling.

OPTIONS:
    --depth <N>, -L <N>    Maximum depth (default: unlimited)
    --summary [N]          Show top N largest items (default: 5)
    --top [N]              Only show top N largest items (default: 50)
    --loc <N>              Only show items with LOC >= N
    --min-loc <N>          Alias for --loc
    --show-hidden, -H      Include hidden files/directories
    --find-artifacts       Highlight build/generated artifacts
    --show-ignored         Show gitignored files
    --help, -h             Show this help message

ARGUMENTS:
    [PATHS...]             Roots to analyze (default: current directory)

EXAMPLES:
    loct tree                       # Full tree
    loct tree --depth 3             # Limit depth
    loct tree --summary 10          # Top 10 largest
    loct tree --loc 100             # LOC threshold
    loct tree src/ --show-hidden    # Include dotfiles";

pub(super) const SLICE_HELP: &str = "loct slice - Extract file + dependencies for AI context

USAGE:
    loct slice <TARGET_PATH> [OPTIONS]

DESCRIPTION:
    Extracts a 'holographic slice' - the target file plus all its dependencies.
    Perfect for feeding focused context to AI assistants.

    Shows what the file USES, not what USES it.
    For reverse dependencies, use --consumers or 'loct query who-imports'.

OPTIONS:
    --consumers, -c    Include reverse dependencies (files that import this)
    --depth <N>        Maximum dependency depth to traverse
    --root <PATH>      Project root for resolving imports
    --rescan           Force snapshot update (includes new/uncommitted files)
    --help, -h         Show this help message

EXAMPLES:
    loct slice src/main.rs              # File + its dependencies
    loct slice src/utils.ts --consumers # Include reverse deps
    loct slice lib/api.ts --depth 2     # Limit to 2 levels
    loct slice src/app.tsx --json       # JSON output for AI tools
    loct slice src/new-file.ts --rescan # Slice a newly created file

RELATED COMMANDS:
    loct query who-imports <file>    Find all importers
    loct auto --for-agent-feed       Full codebase context
    loct focus <dir>                 Slice for a directory";

pub(super) const FIND_HELP: &str = "loct find - Semantic search for symbols and parameters

USAGE:
    loct find [QUERY] [OPTIONS]

DESCRIPTION:
    Semantic search for symbols (functions, classes, types) AND function parameters.
    Uses regex patterns to match names in your codebase.

    Returns three types of matches:
    - Symbol Matches: exported functions, classes, types
    - Parameter Matches: function parameter names (NEW in 0.8.4)
    - Semantic Matches: similar symbol names (fuzzy matching)

    NOT impact analysis - for dependency impact, use 'loct impact'.
    NOT dead code detection - use 'loct dead' or 'loct twins'.

OPTIONS:
    --symbol <PATTERN>   Search for symbols matching regex
    --file <PATTERN>     Search for files matching regex
    --similar <SYMBOL>   Find symbols with similar names (fuzzy)
    --dead               Only show dead/unused symbols
    --exported           Only show exported symbols
    --lang <LANG>        Filter by language (ts, rs, js, py, etc.)
    --limit <N>          Maximum results to show
    --help, -h           Show this help message

EXAMPLES:
    loct find request              # Find 'request' in symbols AND params
    loct find --symbol \".*Config$\" # Regex: symbols ending with Config
    loct find --file \"utils\"       # Files containing \"utils\" in path
    loct find --dead --exported    # Dead exported symbols

OUTPUT:
    === Symbol Matches (10) ===
      src/auth.py:45 - export def login
    === Parameter Matches (34) ===
      src/auth.py:45 - request: Request in login()
    === Semantic Matches (5) ===
      loginUser (score: 0.85) in src/users.py

RELATED COMMANDS:
    loct dead              Find unused exports / dead code
    loct twins             Find duplicate exports and dead parrots
    loct slice <file>      Extract file dependencies
    loct query where-symbol  Find where a symbol is defined";

pub(super) const DEAD_HELP: &str = "loct dead - Detect unused exports / dead code

USAGE:
    loct dead [OPTIONS] [PATHS...]

DESCRIPTION:
    Detects unused exports with confidence levels and optional
    inclusion of tests/helpers. Integrates with quick wins.

OPTIONS:
    --confidence <lvl>   normal|high (default: normal)
    --top <N>            Limit results to top N (default: 20)
    --full, --all        Show all results (ignore --top limit)
    --with-tests         Include test files
    --with-helpers       Include helper files
    --with-shadows       Detect shadow exports (same symbol, multiple files)
    --help, -h           Show this help message

EXAMPLES:
    loct dead
    loct dead --confidence high
    loct dead --with-tests
    loct dead --with-shadows";

pub(super) const CYCLES_HELP: &str = "loct cycles - Detect circular import chains

USAGE:
    loct cycles [OPTIONS] [PATHS...]

DESCRIPTION:
    Detects circular dependencies in your import graph.
    Example: A -> B -> C -> A

    Circular imports cause:
    - Runtime initialization errors
    - Build/bundling failures
    - Flaky test behavior

OPTIONS:
    --path <PATTERN>     Filter to files matching pattern
    --help, -h           Show this help message

EXAMPLES:
    loct cycles                # Detect all cycles
    loct cycles src/           # Only analyze src/
    loct cycles --json         # JSON output for CI

RELATED COMMANDS:
    loct slice <file>       See what a file depends on
    loct query who-imports  Find reverse dependencies
    loct lint --fail        Run as CI check";

pub(super) const TRACE_HELP: &str = "loct trace - Trace a Tauri/IPC handler end-to-end

USAGE:
    loct trace <handler> [ROOTS...]

DESCRIPTION:
    Investigates why a handler is missing/unused:
    - Backend definition (file, line, exposed name)
    - Frontend invoke() calls and plain mentions
    - Registration status in generate_handler![]
    - Verdict + suggestion to fix

OPTIONS:
    --help, -h           Show this help message

ARGUMENTS:
    <handler>            Handler name to trace (exposed or internal)
    [ROOTS...]           Root directories to scan (default: current directory)

EXAMPLES:
    loct trace toggle_assistant
    loct trace standard_command apps/desktop";

pub(super) const JQ_HELP: &str = "loct jq - Query snapshot with jq-style filters

USAGE:
    loct '<filter>' [OPTIONS]

DESCRIPTION:
    Execute jq-style filter expressions on the latest snapshot JSON.
    Automatically finds the most recent snapshot in .loctree/ directory.

    The filter syntax follows jq conventions:
    - .metadata          Extract metadata field
    - .files[]           Iterate over files array
    - .files[0]          Get first file
    - .[\"key\"]         Access key with special characters

OPTIONS:
    -r, --raw-output         Output raw strings, not JSON
    -c, --compact-output     Compact JSON output (no pretty-printing)
    -e, --exit-status        Set exit code based on output (0 if truthy)
    --arg <name> <value>     Pass string variable to filter
    --argjson <name> <json>  Pass JSON variable to filter
    --snapshot <path>        Use specific snapshot file instead of latest
    --help, -h               Show this help message

EXAMPLES:
    loct '.metadata'                         # Extract metadata
    loct '.files | length'                   # Count files
    loct '.files[] | .path'                  # List file paths
    loct '.metadata.total_loc' -r            # Raw number output
    loct '.files[] | select(.lang == \"ts\")' -c
    loct '.files[] | select(.loc > 500)' -c

NOTE:
    This command requires jaq dependencies (enabled by default in the CLI build).";

pub(super) const COMMANDS_HELP: &str = "loct commands - Tauri FE<->BE handler coverage analysis

USAGE:
    loct commands [OPTIONS] [PATHS...]

DESCRIPTION:
    Analyzes Tauri command bridge contracts between frontend and backend.

    Detects:
    - Missing handlers: FE calls invoke('cmd') but no BE #[tauri::command]
    - Unused handlers: BE has #[tauri::command] but FE never calls it
    - Matched handlers: Both FE and BE exist (healthy)

OPTIONS:
    --name <PATTERN>     Filter to commands matching pattern
    --missing-only       Show only missing handlers
    --unused-only        Show only unused handlers
    --limit <N>          Maximum results to show
    --help, -h           Show this help message

EXAMPLES:
    loct commands                    # Full coverage report
    loct commands --missing-only     # Only missing handlers
    loct commands --json --missing   # JSON for CI

RELATED COMMANDS:
    loct events        Analyze Tauri event flow
    loct lint --tauri  Full Tauri contract validation";

pub(super) const EVENTS_HELP: &str = "loct events - Show event flow and issues

USAGE:
    loct events [OPTIONS] [PATHS...]

DESCRIPTION:
    Analyzes event emit/listen pairs, ghost events, and race conditions.

OPTIONS:
    --races             Enable race detection (async/await gaps)
    --no-duplicates     Hide duplicate sections in CLI output
    --no-dynamic-imports Hide dynamic import sections in CLI output
    --help, -h          Show this help message

EXAMPLES:
    loct events
    loct events --races";

pub(super) const ROUTES_HELP: &str = "loct routes - List backend/web routes (FastAPI/Flask)

USAGE:
    loct routes [OPTIONS] [PATHS...]

DESCRIPTION:
    Detects Python web routes based on common decorators:
    - FastAPI: @app.get/post/put/delete/patch, @router.*, @api_router.*
    - Flask:   @app.route, @blueprint.route, .route(...)

OPTIONS:
    --framework <NAME>   Filter by framework label (fastapi, flask)
    --path <PATTERN>     Filter by route path substring
    --help, -h           Show this help message

EXAMPLES:
    loct routes
    loct routes --framework fastapi
    loct routes --path /patients";

pub(super) const INFO_HELP: &str = "loct info - Show snapshot metadata and project info

USAGE:
    loct info

DESCRIPTION:
    Prints snapshot metadata, detected stack, and analysis summary.

OPTIONS:
    --help, -h          Show this help message";

pub(super) const LINT_HELP: &str = "loct lint - Structural lint and policy checks

USAGE:
    loct lint [OPTIONS] [PATHS...]

DESCRIPTION:
    Runs structural linting: entrypoints, handlers, ghost events, races.

OPTIONS:
    --entrypoints        List entrypoints
    --sarif              Emit SARIF
    --tauri              Apply Tauri presets
    --fail               Exit non-zero on findings
    --no-duplicates      Hide duplicate sections in CLI output
    --no-dynamic-imports Hide dynamic import sections in CLI output
    --help, -h           Show this help message

EXAMPLES:
    loct lint
    loct lint --fail --sarif";

pub(super) const REPORT_HELP: &str = "loct report - Generate HTML/JSON reports

USAGE:
    loct report [OPTIONS] [PATHS...]

DESCRIPTION:
    Runs full analysis and emits HTML/JSON/SARIF reports.

OPTIONS:
    --output <FILE>      Output HTML path
    --serve              Serve report locally
    --port <N>           Port for --serve
    --editor <NAME>      Editor integration (code/cursor/jetbrains)
    --help, -h           Show this help message

EXAMPLES:
    loct report --output report.html
    loct report --serve --port 4173";

pub(super) const QUERY_HELP: &str = "loct query - Graph queries (who-imports, who-exports, etc.)

USAGE:
    loct query <KIND> <TARGET>

DESCRIPTION:
    Query the import graph and symbol index for specific relationships.
    Targeted queries against the dependency graph built by 'loct scan'.

QUERY KINDS:
    who-imports <FILE>      Find all files that import the file (reverse deps)
    where-symbol <SYMBOL>   Find where a symbol is defined/exported
    component-of <FILE>     Show which component/module contains the file

OPTIONS:
    --help, -h              Show this help message

EXAMPLES:
    loct query who-imports src/utils.ts       # What imports utils.ts?
    loct query where-symbol PatientRecord     # Where is it defined?
    loct query component-of src/ui/Button.tsx # What owns Button?

RELATED COMMANDS:
    loct slice <file>           Show what a file depends on
    loct find --symbol <name>   Search for symbols by pattern
    loct dead                   Find symbols with 0 imports";

pub(super) const IMPACT_HELP: &str = "loct impact - Analyze impact of modifying/removing a file

USAGE:
    loct impact <FILE> [OPTIONS]

DESCRIPTION:
    Shows \"what breaks if you modify or remove this file\" by traversing
    the reverse dependency graph. Finds all direct and transitive consumers.

    This is different from 'query who-imports':
    - who-imports: Finds direct importers only
    - impact: Finds ALL affected files (direct + transitive)

    Useful for:
    - Understanding change impact before refactoring
    - Identifying critical files (high fan-out)
    - Safe deletion analysis

OPTIONS:
    --depth <N>          Limit traversal depth (default: unlimited)
    --root <PATH>        Project root (default: current directory)
    --json               Output as JSON for agent consumption
    --help, -h           Show this help message

ARGUMENTS:
    <FILE>               Path to the file to analyze (required)

EXAMPLES:
    loct impact src/utils.ts                # Full impact analysis
    loct impact src/api.ts --depth 2        # Limit to 2 levels deep
    loct impact lib/helpers.ts --json       # JSON output
    loct impact src/core.ts --root ./       # Specify project root

OUTPUT FORMAT:
    Direct consumers (5 files):
      src/app.ts (import)
      src/lib.ts (import)
      ...

    Transitive impact (23 files total):
      [depth 2] src/page.tsx (import)
      ...

    Warning: Removing this file would break 28 files (max depth: 3)";

pub(super) const DIFF_HELP: &str = "loct diff - Compare snapshots between branches/commits

USAGE:
    loct diff --since <SNAPSHOT> [--to <SNAPSHOT>] [OPTIONS]

DESCRIPTION:
    Compares two code snapshots and shows semantic differences.

    Unlike git diff (line changes), this shows structural changes:
    - New/removed files and symbols
    - Import graph changes
    - New dead code introduced (regressions)
    - New circular dependencies

OPTIONS:
    --since <SNAPSHOT>   Base snapshot to compare from (required)
    --to <SNAPSHOT>      Target snapshot (default: current working tree)
    --auto-scan-base     Auto-create git worktree and scan target branch
    --problems-only      Show only regressions (new dead code, new cycles)
    --help, -h           Show this help message

EXAMPLES:
    loct diff --since main                    # Compare main to working tree
    loct diff --since HEAD~1                  # Compare to previous commit
    loct diff --since main --auto-scan-base   # Auto-scan main branch
    loct diff --since v1.0.0 --to v2.0.0      # Compare two tags

RELATED COMMANDS:
    loct scan             Run scan to create snapshot
    loct auto --full-scan Force full rescan";

pub(super) const MEMEX_HELP: &str = "loct memex - Index analysis into AI memory (vector DB)

USAGE:
    loct memex [OPTIONS]

DESCRIPTION:
    Pushes analysis artifacts to vector memory for agents.

OPTIONS:
    --help, -h          Show this help message";

pub(super) const CROWD_HELP: &str =
    "loct crowd - Detect functional crowds (similar files clustering)

USAGE:
    loct crowd [PATTERN]

DESCRIPTION:
    Groups related files around a seed pattern (name or path fragment).

OPTIONS:
    --help, -h          Show this help message

EXAMPLES:
    loct crowd cache
    loct crowd session";

pub(super) const TAGMAP_HELP: &str = "loct tagmap - Unified search around a keyword

USAGE:
    loct tagmap <KEYWORD> [OPTIONS]

DESCRIPTION:
    Aggregates three analyses into one view:
    1. FILES:  All files with keyword in path or name
    2. CROWD:  Functional clustering around the keyword
    3. DEAD:   Dead exports related to the keyword

    Perfect for understanding everything about a domain/feature at once.

OPTIONS:
    --include-tests    Include test files in analysis
    --limit <N>        Maximum results per section (default: 20)
    --json             Output as JSON for AI tools
    --help, -h         Show this help message

ARGUMENTS:
    <KEYWORD>          Keyword to search for (required)

EXAMPLES:
    loct tagmap patient           # Everything about 'patient' feature
    loct tagmap auth              # Auth-related files, crowds, dead code
    loct tagmap message --json    # JSON output for AI processing
    loct tagmap api --limit 10    # Limit results

OUTPUT FORMAT:
    === TAGMAP: 'patient' ===

    FILES MATCHING KEYWORD (12):
      src/features/patients/PatientsList.tsx
      src/hooks/usePatient.ts
      ...

    CROWD ANALYSIS (8 files):
      Score: 7.2/10
      Members: PatientsList, PatientDetail, PatientForm...
      Issues: Consider consolidating similar files

    DEAD EXPORTS (3):
      oldPatientHandler in src/api/patients.ts
      PatientV1 in src/types/patient.ts
      ...

RELATED COMMANDS:
    loct crowd <pattern>    Detailed crowd analysis
    loct dead               All dead exports
    loct find <query>       Symbol/file search
    loct focus <dir>        Directory-level context";

pub(super) const TWINS_HELP: &str =
    "loct twins - Find dead parrots (0 imports) and duplicate exports

USAGE:
    loct twins [OPTIONS] [PATH]

DESCRIPTION:
    Detects semantic issues in your export/import graph:

    Dead Parrots:   Exports with 0 imports anywhere in the codebase
                    (Monty Python reference - code that looks alive but isn't used)

    Exact Twins:    Same symbol name exported from multiple files
                    (can cause import confusion)

    Barrel Chaos:   Re-export anti-patterns
                    (missing index.ts, deep re-export chains)

    This is a code smell detector - findings are hints, not verdicts.

OPTIONS:
    --path <DIR>           Root directory to analyze
    --dead-only            Show only dead parrots (0 imports)
    --include-tests        Include test files (excluded by default)
    --include-suppressed   Show suppressed findings too
    --help, -h             Show this help message

EXAMPLES:
    loct twins                    # Full analysis
    loct twins --dead-only        # Only exports with 0 imports
    loct twins src/               # Analyze specific directory
    loct twins --include-tests    # Include test files
    loct twins --include-suppressed  # Include suppressed items

SUPPRESSION:
    Mark findings as false positives (they won't show in subsequent runs):
    loct suppress twins <symbol>              # Suppress a twin
    loct suppress twins <symbol> --file <f>   # Suppress only in specific file
    loct suppress --list                      # Show all suppressions
    loct suppress --clear                     # Clear all suppressions

RELATED COMMANDS:
    loct dead              Detailed dead code analysis
    loct sniff             Aggregate code smell analysis
    loct suppress          Manage false positive suppressions
    loct find --dead       Search for specific dead symbols";

pub(super) const DIST_HELP: &str = "loct dist - Analyze bundle distribution using source maps

USAGE:
    loct dist --source-map <PATH> --src <DIR>

DESCRIPTION:
    Compares source code exports with bundled JavaScript to find truly dead exports.
    Uses source maps to detect code that was completely tree-shaken out by the bundler.

    This is different from regular dead code detection:
    - Regular: Finds exports with 0 imports in your source code
    - Dist: Finds exports removed from the production bundle

    Useful for:
    - Validating tree-shaking effectiveness
    - Finding code that can be safely removed
    - Understanding bundle size optimizations

OPTIONS:
    --source-map <PATH>    Path to source map file (e.g., dist/main.js.map)
    --src <DIR>            Source directory to scan (e.g., src/)
    --help, -h             Show this help message

EXAMPLES:
    loct dist --source-map dist/main.js.map --src src/
    loct dist --source-map build/app.js.map --src app/src/

OUTPUT (JSON):
    {
      \"sourceExports\": 500,
      \"bundledExports\": 120,
      \"deadExports\": [
        { \"file\": \"src/utils.ts\", \"line\": 42, \"name\": \"unusedHelper\" }
      ],
      \"reduction\": \"76%\"
    }";

pub(super) const COVERAGE_HELP: &str =
    "loct coverage - Analyze test coverage gaps (structural coverage)

USAGE:
    loct coverage [OPTIONS] [PATHS...]

DESCRIPTION:
    Performs structural test coverage analysis by cross-referencing:
    - Frontend invoke/emit calls (what the app uses)
    - Backend handlers and events (what the app exposes)
    - Test file imports (what tests actually cover)

    Unlike line coverage tools, this shows:
    - Which handlers have no corresponding tests
    - Which events are emitted but never tested
    - Which exports are tested but never used in production

    This is semantic coverage - not 'how many lines' but 'what functionality'.

OPTIONS:
    --handlers-only       Only show handler gaps (skip events/exports)
    --events-only         Only show event gaps (skip handlers/exports)
    --min-severity <LVL>  Filter by minimum severity: critical, high, medium, low
    --json                Output as JSON for programmatic use
    --help, -h            Show this help message

ARGUMENTS:
    [PATHS...]            Root directories to scan (default: current directory)

EXAMPLES:
    loct coverage                          # Show all coverage gaps
    loct coverage --handlers-only          # Focus on untested handlers
    loct coverage --min-severity high      # Only critical/high issues
    loct coverage --json                   # Machine-readable output

OUTPUT:
    Groups findings by severity:
    - CRITICAL: Handlers without any test (used in production!)
    - HIGH: Events emitted but never tested
    - MEDIUM: Exports without test imports
    - LOW: Tests that import unused code

    Each gap shows the source location and usage context.";

pub(super) const SNIFF_HELP: &str = "loct sniff - Sniff for code smells (aggregate analysis)

USAGE:
    loct sniff [OPTIONS]

DESCRIPTION:
    Aggregates all smell-level findings worth checking:

    Twins:        Same symbol name in multiple files
                  - Can cause import confusion

    Dead Parrots: Exports with 0 imports
                  - Potentially unused code

    Crowds:       Files with similar dependency patterns
                  - Possible duplication or fragmentation

    Output is friendly and non-judgmental. These are hints, not verdicts.

OPTIONS:
    --path <DIR>           Root directory to analyze (default: current directory)
    --dead-only            Show only dead parrots (skip twins and crowds)
    --twins-only           Show only twins (skip dead parrots and crowds)
    --crowds-only          Show only crowds (skip twins and dead parrots)
    --include-tests        Include test files in analysis (default: false)
    --min-crowd-size <N>   Minimum crowd size to report (default: 2)
    --json                 Output as JSON for programmatic use
    --help, -h             Show this help message

EXAMPLES:
    loct sniff                    # Full code smell analysis
    loct sniff --dead-only        # Only dead parrots
    loct sniff --twins-only       # Only duplicate names
    loct sniff --crowds-only      # Only similar file clusters
    loct sniff --include-tests    # Include test files
    loct sniff --json             # Machine-readable output

OUTPUT:
    Aggregates three types of code smells:
    - TWINS: Same symbol exported from multiple files
    - DEAD PARROTS: Exports with 0 imports
    - CROWDS: Files clustering around similar functionality

    Each section provides actionable suggestions for consolidation or cleanup.";

pub(super) const SUPPRESS_HELP: &str = "loct suppress - Mark findings as false positives

USAGE:
    loct suppress <type> <symbol> [OPTIONS]
    loct suppress --list
    loct suppress --clear

DESCRIPTION:
    Manages false positive suppressions so reviewed findings don't appear
    in subsequent runs. Suppressions are stored in .loctree/suppressions.toml.

    Use this when you've reviewed a finding and determined it's intentional:
    - FE/BE type mirrors (same type defined in TypeScript and Rust)
    - Intentional re-exports for public APIs
    - Entry points that appear 'dead' but are used externally

TYPES:
    twins          Exact twin (same symbol in multiple files)
    dead_parrot    Dead parrot (export with 0 imports)
    dead_export    Dead export (unused export)
    circular       Circular import

OPTIONS:
    --file <PATH>      Only suppress in specific file (default: all files)
    --reason <TEXT>    Document why this is a false positive
    --list             Show all current suppressions
    --clear            Remove all suppressions
    --help, -h         Show this help message

EXAMPLES:
    loct suppress twins Message              # Suppress 'Message' twin everywhere
    loct suppress twins Message --file src/types.ts  # Only in specific file
    loct suppress dead_parrot unusedHelper --reason 'Used via dynamic import'
    loct suppress --list                     # View all suppressions
    loct suppress --clear                    # Reset suppressions

STORAGE:
    Suppressions are stored in .loctree/suppressions.toml
    Commit this file to share suppressions with your team.

RELATED COMMANDS:
    loct twins         Find twins and dead parrots (--include-suppressed to show all)
    loct dead          Find unused exports
    loct sniff         Aggregate smell analysis";

pub(super) const FOCUS_HELP: &str = "loct focus - Extract holographic context for a directory

USAGE:
    loct focus <DIRECTORY> [OPTIONS]

DESCRIPTION:
    Like 'slice' but for directories. Extracts a holographic view of a directory:

    Core:       All files within the target directory
    Internal:   Import edges between files inside the directory
    Deps:       External files imported by core (outside the directory)
    Consumers:  Files outside the directory that import core files

    Perfect for understanding feature modules like 'src/features/patients/'.

OPTIONS:
    --consumers, -c    Include files that import from this directory
    --depth <N>        Maximum depth for external dependency traversal
    --root <PATH>      Project root (default: current directory)
    --json             Output as JSON for agent consumption
    --help, -h         Show this help message

ARGUMENTS:
    <DIRECTORY>        Path to the directory to analyze (required)

EXAMPLES:
    loct focus src/features/patients/           # Focus on patients feature
    loct focus src/components/ --consumers      # Include external consumers
    loct focus lib/utils/ --depth 1             # Limit external dep depth
    loct focus src/api/ --json                  # JSON output for AI tools

OUTPUT FORMAT:
    Focus: src/features/patients/

    Core (12 files, 2,340 LOC):
      src/features/patients/index.ts
      src/features/patients/PatientsList.tsx
      ...

    Internal edges: 18 imports within directory

    External Deps (8 files, 890 LOC):
      [d1] src/components/Button.tsx
      ...

    Consumers (3 files, 450 LOC):
      src/App.tsx
      ...

    Total: 23 files, 3,680 LOC

RELATED COMMANDS:
    loct slice <file>       Extract context for a single file
    loct impact <file>      Show what breaks if you change a file
    loct crowd <pattern>    Find files clustering around a pattern";

pub(super) const HOTSPOTS_HELP: &str =
    "loct hotspots - Import frequency heatmap (core vs peripheral)

USAGE:
    loct hotspots [OPTIONS]

DESCRIPTION:
    Ranks files by how often they are imported (in-degree) to identify:

    CORE:       Files imported by 10+ others (critical infrastructure)
    SHARED:     Files imported by 3-9 others (shared utilities)
    PERIPHERAL: Files imported by 1-2 others (feature-specific)
    LEAF:       Files with 0 importers (entry points or dead code)

    This helps AI agents understand which files are risky to modify
    (high in-degree = many dependents) vs safe to refactor (low in-degree).

OPTIONS:
    --min <N>              Minimum import count to show (default: 1)
    --limit <N>            Maximum files to show (default: 50)
    --leaves               Show only leaf nodes (0 importers)
    --coupling             Include out-degree (files that import many others)
    --root <PATH>          Project root (default: current directory)
    --json                 Output as JSON for agent consumption
    --help, -h             Show this help message

EXAMPLES:
    loct hotspots                    # Show top 50 most-imported files
    loct hotspots --limit 20         # Top 20 only
    loct hotspots --leaves           # Find leaf nodes (entry points / dead)
    loct hotspots --coupling         # Show both in-degree and out-degree
    loct hotspots --min 5            # Only files with 5+ importers
    loct hotspots --json             # JSON output for AI tools

OUTPUT FORMAT:
    Import Hotspots (42 files analyzed)

    CORE (10+ importers):
      [32] src/utils/helpers.ts           # hub module
      [18] src/components/Button.tsx

    SHARED (3-9 importers):
      [7]  src/hooks/useAuth.ts
      [5]  src/api/client.ts

    PERIPHERAL (1-2 importers):
      [2]  src/features/login/form.tsx
      [1]  src/features/login/types.ts

    LEAF (0 importers):
      src/pages/index.tsx               # entry point
      src/features/old/legacy.ts        # possibly dead

    With --coupling:
      [in:32 out:3]  src/utils/helpers.ts    # hub, low coupling
      [in:2  out:15] src/features/main.tsx   # feature root, high coupling

RELATED COMMANDS:
    loct dead               Find unused exports
    loct impact <file>      Show what breaks if you modify a file
    loct focus <dir>        Extract context for a directory";

pub(super) const LAYOUTMAP_HELP: &str = "loct layoutmap - Analyze CSS layout properties

USAGE:
    loct layoutmap [OPTIONS]

DESCRIPTION:
    Extracts and analyzes layout-related CSS properties from your codebase:

    Z-INDEX:    Shows all z-index values across CSS/SCSS files, sorted by value.
                Helps identify layering conflicts and understand UI stacking.

    POSITION:   Lists sticky/fixed positioned elements.
                Useful for understanding what elements persist during scroll.

    DISPLAY:    Identifies grid/flex layouts and their locations.
                Maps out the layout architecture of your components.

OPTIONS:
    --zindex-only          Show only z-index values
    --sticky-only          Show only sticky/fixed position elements
    --grid-only            Show only grid/flex layouts
    --min-zindex <N>       Filter z-index values >= N (default: show all)
    --exclude <PATTERN>    Exclude paths matching glob (can be repeated)
    --root <PATH>          Project root (default: current directory)
    --json                 Output as JSON for agent consumption
    --help, -h             Show this help message

EXAMPLES:
    loct layoutmap                  # Full CSS layout analysis
    loct layoutmap --zindex-only    # Only z-index hierarchy
    loct layoutmap --sticky-only    # Only sticky/fixed elements
    loct layoutmap --min-zindex 100 # High z-index values (likely overlays)
    loct layoutmap --exclude .obsidian --exclude prototype  # Skip dirs
    loct layoutmap --json           # JSON output for AI tools

OUTPUT FORMAT:
    Z-INDEX HIERARCHY:
      [9999] src/components/Modal.css:15       .modal-overlay
      [1000] src/components/Toast.css:8        .toast-container
      [ 100] src/components/Dropdown.css:23    .dropdown-menu
      [  10] src/components/Header.css:5       .header

    STICKY/FIXED ELEMENTS:
      [fixed]  src/components/Header.css:12    .header
      [sticky] src/components/Sidebar.css:5    .sidebar-nav

    GRID/FLEX LAYOUTS:
      [grid]   src/layouts/Dashboard.css:8     .dashboard-grid
      [flex]   src/components/Card.css:3       .card-content

RELATED COMMANDS:
    loct crowd              Find functionally similar components
    loct find <pattern>     Search for CSS selectors or properties";

pub(super) const ZOMBIE_HELP: &str = "loct zombie - Find all zombie code (combined analysis)

USAGE:
    loct zombie [OPTIONS] [PATHS...]

DESCRIPTION:
    Combines three sources of dead code into one actionable report:

    DEAD EXPORTS:     Unused exports detected by dead code analysis
                      (symbols with 0 imports)

    ORPHAN FILES:     Files with 0 importers (not imported by any other file)
                      Entry points are OK, but others might be dead

    SHADOW EXPORTS:   Same symbol exported by multiple files where some
                      have 0 imports (likely consolidation candidates)

    This is a comprehensive zombie hunter - finds all forms of potentially
    dead code in a single scan.

OPTIONS:
    --include-tests    Include test files in analysis (default: false)
    --json             Output as JSON for programmatic use
    --help, -h         Show this help message

ARGUMENTS:
    [PATHS...]         Root directories to scan (default: current directory)

EXAMPLES:
    loct zombie                    # Find all zombie code
    loct zombie --include-tests    # Include test files
    loct zombie src/               # Analyze specific directory
    loct zombie --json             # Machine-readable output

OUTPUT FORMAT:
    Zombie Code Report

    Dead Exports (3):
      src/utils/old.ts:15 - unusedFunction
      src/hooks/legacy.ts:8 - useLegacyHook
      ...

    Orphan Files (0 importers, 2):
      src/features/patients/PatientsList.tsx (504 LOC)
      src/components/deprecated/OldButton.tsx (89 LOC)

    Shadow Exports (1):
      conversationHostStore exported by 2 files, 1 dead

    Total: 6 zombie items, ~950 LOC to review

RELATED COMMANDS:
    loct dead               Detailed dead export analysis
    loct twins              Dead parrots and semantic duplicates
    loct hotspots --leaves  Find leaf nodes (0 importers)
    loct sniff              Code smell analysis";

pub(super) const HEALTH_HELP: &str = "loct health - Quick health check summary

USAGE:
    loct health [OPTIONS] [PATHS...]

DESCRIPTION:
    One-shot summary of all structural issues in your codebase:
    - Cycles: Circular import count (hard vs structural)
    - Dead: Unused exports (high confidence count)
    - Twins: Duplicate symbol names across files

    Use this as a quick sanity check before commits or in CI.
    Run individual commands for detailed analysis.

OPTIONS:
    --include-tests    Include test files in analysis (default: false)
    --json             Output as JSON for programmatic use
    --help, -h         Show this help message

ARGUMENTS:
    [PATHS...]         Root directories to scan (default: current directory)

EXAMPLES:
    loct health                    # Quick health summary
    loct health --include-tests    # Include test files
    loct health src/               # Analyze specific directory
    loct health --json             # Machine-readable output

OUTPUT FORMAT:
    Health Check Summary

    Cycles:      3 total (2 hard, 1 structural)
    Dead:        6 high confidence, 24 low
    Twins:       2 duplicate symbol groups

    Run `loct cycles`, `loct dead`, `loct twins` for details.

RELATED COMMANDS:
    loct cycles    Detailed circular import analysis
    loct dead      Detailed dead export analysis
    loct twins     Duplicate export analysis
    loct zombie    Combined dead/orphan/shadow analysis";

pub(super) const AUDIT_HELP: &str = "loct audit - Full codebase audit with actionable findings

USAGE:
    loct audit [OPTIONS] [PATHS...]

DESCRIPTION:
    Comprehensive analysis combining all structural checks into one report.
    Perfect for getting a complete picture of codebase health on day one.

    Includes:
    - Cycles: Circular imports (hard + structural)
    - Dead exports: Unused code with 0 imports
    - Twins: Same symbol exported from multiple files
    - Orphan files: Files with 0 importers (not entry points)
    - Shadow exports: Consolidation candidates
    - Crowds: Files with similar dependency patterns

    Each finding includes actionable suggestions for cleanup.

OPTIONS:
    --include-tests    Include test files in analysis (default: false)
    --json             Output as JSON for programmatic use
    --help, -h         Show this help message

ARGUMENTS:
    [PATHS...]         Root directories to scan (default: current directory)

EXAMPLES:
    loct audit                     # Full audit of current directory
    loct audit --include-tests     # Include test files
    loct audit src/                # Audit specific directory
    loct audit --json              # Machine-readable output for CI

OUTPUT FORMAT:
    Full Codebase Audit

    CYCLES (3 total)
      2 hard cycles (breaking)
      1 structural cycle

    DEAD EXPORTS (12 total)
      6 high confidence
      6 low confidence

    TWINS (2 groups)
      useAuth exported from 2 files
      formatDate exported from 3 files

    ORPHAN FILES (4 files, 1,200 LOC)
      src/legacy/old-utils.ts (450 LOC)
      src/deprecated/helper.ts (320 LOC)
      ...

    SHADOW EXPORTS (1)
      store exported by 2 files, 1 dead

    CROWDS (2 clusters)
      API handlers: 5 similar files
      Form components: 3 similar files

    ----------------------
    Total: 22 findings to review
    Run individual commands for details.

RELATED COMMANDS:
    loct health    Quick summary (cycles + dead + twins only)
    loct zombie    Dead exports + orphans + shadows
    loct sniff     Code smells (twins + dead + crowds)
    loct cycles    Detailed cycle analysis
    loct dead      Detailed dead export analysis";

pub(super) const DOCTOR_HELP: &str =
    "loct doctor - Interactive diagnostics with categorized findings

USAGE:
    loct doctor [OPTIONS] [PATHS...]

DESCRIPTION:
    Interactive troubleshooting command that analyzes your codebase and
    categorizes findings into actionable groups:

    1. Auto-fixable: Issues with clear automated solutions
    2. Needs Review: Potential issues requiring human judgment
    3. Suggested Suppressions: Patterns to add to .loctignore

    Unlike 'audit' which dumps everything, doctor guides you through
    the most impactful fixes first.

OPTIONS:
    --include-tests        Include test files in analysis (default: false)
    --apply-suppressions   Auto-add suggested patterns to .loctignore
    --help, -h             Show this help message

ARGUMENTS:
    [PATHS...]             Root directories to scan (default: current directory)

EXAMPLES:
    loct doctor                        # Interactive diagnostics
    loct doctor --apply-suppressions   # Auto-add .loctignore patterns
    loct doctor src/                   # Analyze specific directory
    loct doctor --include-tests        # Include test files

OUTPUT FORMAT:
    Codebase Diagnostics

    AUTO-FIXABLE (3 issues)
    ------------------------
    v Dead export: formatLegacy in src/utils.ts
      -> Remove or add @loctignore:dead-ok

    NEEDS REVIEW (2 issues)
    ------------------------
    ? Cycle: src/a.ts <-> src/b.ts (structural)
      -> Consider extracting shared types

    SUGGESTED SUPPRESSIONS
    ------------------------
    Add to .loctignore:
      @loctignore:dead-ok src/generated/**

    Run with --apply-suppressions to auto-add.

RELATED COMMANDS:
    loct health    Quick summary (cycles + dead + twins)
    loct audit     Full audit without categorization
    loct dead      Detailed dead export analysis
    loct cycles    Detailed cycle analysis";
