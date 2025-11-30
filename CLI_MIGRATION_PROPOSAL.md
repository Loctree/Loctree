Here’s a clean, final recommendation phrased as instructions for an Agent working on the CLI.

⸻

1. High-level rule: no ai subcommand
	1.	Do not introduce a loct ai subcommand.
	2.	Commands must describe what they do, not who they are for.
	•	Good: slice, context, tree, dead, cycles, lint, report.
	•	Bad: ai, human, agent, etc.
	3.	AI agents should use the same verbs as humans. Their “API” is:
	•	JSON output (--json),
	•	stable, composable subcommands,
	•	predictable exit codes.

If we later need a higher-level shortcut, use a functional name like loct context or loct suggest, not loct ai.

⸻

2. Final subcommand set

The Agent should implement and maintain the CLI around these subcommands:
	•	loct (no args) or init
	•	loct scan
	•	loct tree
	•	loct slice
	•	loct find
	•	loct dead
	•	loct cycles
	•	loct lint
	•	loct report
	•	loct git

2.1 loct (smart default)
	•	If snapshot exists: show status.
	•	If snapshot does not exist: run scan on the current directory.
	•	Support --help, --version, global flags.

2.2 loct scan
	•	Responsible for building and updating .loctree/snapshot.json.
	•	Supports presets and ignores.
	•	Typical patterns:
	•	loct scan
	•	loct scan src
	•	loct scan --full
	•	loct scan --gitignore --ignore path --ext rs,ts.

2.3 loct tree
	•	Directory tree plus LOC, for humans and AI.
	•	Depth control, summary, hidden files, LOC threshold.
	•	Used when an agent needs a quick structural overview.

2.4 loct slice <file>
	•	Primary “AI-facing” command.
	•	Always supports:
	•	--consumers
	•	--depth <n>
	•	--json
	•	Semantics: three-layer “holographic context”:
	•	Core file, its imports, its consumers.
	•	Agents should prefer:
	•	loct slice <file> --consumers --json
as the default building block for context.

2.5 loct find <query>
	•	Unified “search” surface:
	•	symbol search, file search, fuzzy “check”, impact.
	•	Flags:
	•	--symbol, --file, --check, --impact, --files-only, --json.
	•	Example agent workflows:
	•	Check for similar components before creating a new one.
	•	Impact analysis for a planned refactor.

2.6 loct dead
	•	Dedicated to unused exports / dead code.
	•	Name: keep dead.
	•	Short, consistent with “dead code”.
	•	Better than janitor (too cute) or unused (longer and less specific).
	•	Flags:
	•	--confidence, --limit, --json.

2.7 loct cycles
	•	Dedicated to circular import cycles.
	•	Name: keep cycles.
	•	“Cycles” is short and clearly refers to dependency cycles.
	•	circular is longer and less idiomatic for a verb-style CLI.
	•	Flags:
	•	--json.

2.8 loct lint
	•	Umbrella for quality checks and “coverage” style analysis.
	•	Responsibilities:
	•	Tauri FE↔BE handler coverage, Python race detection, entrypoints, etc.
	•	--fail to turn findings into CI failures.
	•	--sarif to emit SARIF 2.1.0.
	•	Agent usage pattern:
	•	loct lint --fail --sarif > results.sarif in CI.
	•	loct lint --tauri for Tauri coverage.

2.9 loct report
	•	Generates HTML or similar rich reports.
	•	May optionally serve via HTTP (--serve).
	•	Used mainly for humans; still should support --json where applicable.

2.10 loct git <subcommand>
	•	All Git-related/time-dimension analysis lives here.
	•	Example subcommands:
	•	git compare
	•	git blame
	•	git history
	•	Agents should treat this as optional: only use if temporal context is needed.

⸻

3. Decision on trace

For the Agent implementing CLI:
	1.	Do not keep a top-level trace command just for “Tauri tracing”.
	2.	Integrate these concerns into:
	•	loct lint --tauri (coverage, missing handlers, ghost events), and/or
	•	loct git if there are history/timeline aspects.
	3.	If a “trace” concept remains necessary, expose it as a mode/flag, not a new top-level verb:
	•	Example: loct find --tauri-trace inside find, or
	•	loct lint --tauri --trace if it’s about runtime-style relationships.

Goal: keep the top-level verb set small and semantically clear.

⸻

4. Global flags and machine-readability

The Agent must ensure:
	1.	Every subcommand supports:
	•	--json (machine output, stable schema per subcommand)
	•	--quiet, --verbose
	•	--color auto|always|never
	2.	Exit codes are deterministic:
	•	0 for success / no issues.
	•	1 for “issues found but ran successfully” (for lint, dead, etc., when --fail is used).
	•	Distinct non-zero codes for internal failures (IO, parsing), so CI and agents can differentiate.
	3.	JSON schemas for each subcommand are documented and kept backward-compatible as much as possible.

⸻

5. Migration behaviour (for backwards compatibility)

The Agent should implement the migration phases as follows:
	1.	Phase 1 (current minor version):
	•	Keep legacy -A and old flags, but emit a deprecation warning mapping them to new subcommands.
	•	Mapping examples:
	•	loct -A --dead → loct dead
	•	loct -A --circular → loct cycles
	•	loct -A --symbol X → loct find X --symbol
	•	loct --tree → loct tree.
	2.	Phase 2 (next minor / pre-1.0):
	•	Warnings become more explicit (include removal version and new command).
	3.	Phase 3 (1.0.0):
	•	Remove legacy flags entirely; only subcommand interface remains.

Agents generating code, documentation, or examples must only use the new subcommand syntax from now on. Legacy syntax is for backwards compatibility in the binary, not for new content.

⸻

6. How AI agents should actually use loct

When you, as an Agent, integrate Loctree:
	1.	To get structural context for a file:
	•	loct slice <file> --consumers --json
	2.	To check if a component already exists (before creating a new one):
	•	loct find "<name>" --check --json
	3.	To see impact of changing a file:
	•	loct find --impact <file> --json
	4.	To clean up dead code before or after a refactor:
	•	loct dead --confidence high --json
	5.	To enforce code health in CI:
	•	loct scan (once per job)
	•	loct lint --fail --sarif > results.sarif
	6.	To generate a human-friendly overview for the user:
	•	loct tree --summary
	•	loct report -s if they want a clickable report.

No loct ai. Use functional verbs only, plus --json and clear schemas so that agents feel “native” in Unix pipelines instead of having a special lane.