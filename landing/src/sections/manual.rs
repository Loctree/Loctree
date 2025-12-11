use leptos::prelude::*;

#[component]
pub fn Manual() -> impl IntoView {
    view! {
        <section id="manual" class="manual">
            <div class="container">
                // Quick Start
                <div class="manual-section">
                    <h2 class="manual-title">"Quick Start"</h2>
                    <div class="manual-content">
                        <div class="install-box">
                            <code class="install-cmd">"cargo install loctree"</code>
                            <p class="install-note">"Requires Rust 1.75+. Installs "<code>"loct"</code>" and "<code>"loctree"</code>" binaries."</p>
                        </div>
                        <div class="quick-steps">
                            <div class="step">
                                <span class="step-num">"1"</span>
                                <div class="step-content">
                                    <code>"cd your-project && loct"</code>
                                    <p>"Scans codebase, creates snapshot in "<code>".loctree/"</code></p>
                                </div>
                            </div>
                            <div class="step">
                                <span class="step-num">"2"</span>
                                <div class="step-content">
                                    <code>"loct report --serve"</code>
                                    <p>"Opens interactive HTML dashboard at "<code>"localhost:3000"</code></p>
                                </div>
                            </div>
                            <div class="step">
                                <span class="step-num">"3"</span>
                                <div class="step-content">
                                    <code>"loct dead"</code>
                                    <p>"Lists unused exports (Dead Parrots)"</p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                // Core Concepts
                <div class="manual-section">
                    <h2 class="manual-title">"Core Concepts"</h2>
                    <div class="concepts-grid">
                        <ConceptCard
                            title="Dead Parrots"
                            icon="ghost"
                            description="Exported symbols that are never imported anywhere. Like Monty Python's parrot - they look alive but they're not."
                            cmd="loct dead --confidence high"
                        />
                        <ConceptCard
                            title="Crowds"
                            icon="users"
                            description="Files clustering around the same concept (e.g., 5 files with 'message' in the name). Indicates potential duplication or poor organization."
                            cmd="loct crowd message"
                        />
                        <ConceptCard
                            title="Circular Imports"
                            icon="cycle"
                            description="A imports B, B imports C, C imports A. Causes runtime issues, makes code hard to understand."
                            cmd="loct cycles"
                        />
                        <ConceptCard
                            title="Holographic Slices"
                            icon="slice"
                            description="3-layer context: the file + its dependencies + files that import it. Perfect for AI agents."
                            cmd="loct slice src/App.tsx --consumers"
                        />
                        <ConceptCard
                            title="Command Bridges"
                            icon="bridge"
                            description="Tauri FE-BE mapping. Detects invoke() calls without handlers and handlers without calls."
                            cmd="loct commands --missing"
                        />
                        <ConceptCard
                            title="Barrels"
                            icon="barrel"
                            description="Index files that re-export from other modules. Loctree tracks re-export chains for accurate dead code detection."
                            cmd="loct barrels"
                        />
                    </div>
                </div>

                // HTML Report Dashboard
                <div class="manual-section">
                    <h2 class="manual-title">"HTML Report Dashboard"</h2>
                    <p class="manual-description">
                        "The report dashboard provides visual analysis of your codebase. Generate with "<code>"loct report"</code>" or serve live with "<code>"loct report --serve"</code>"."
                    </p>
                    <div class="dashboard-tabs">
                        <DashboardTab
                            name="Overview"
                            description="Summary stats, health score, AI insights. Quick glance at codebase state."
                        />
                        <DashboardTab
                            name="Duplicates"
                            description="Exported symbols defined in multiple files. Ranked by severity and usage."
                        />
                        <DashboardTab
                            name="Crowds"
                            description="File clusters around concepts. Score 0-10 (higher = more problematic). Shows naming collisions, usage asymmetry."
                        />
                        <DashboardTab
                            name="Cycles"
                            description="Circular import chains. Strict cycles (critical) vs lazy cycles (via dynamic imports)."
                        />
                        <DashboardTab
                            name="Dead Code"
                            description="Unused exports with confidence levels. Filter by 'high' or 'very-high' confidence. Click to open in editor."
                        />
                        <DashboardTab
                            name="Commands"
                            description="Tauri FE-BE bridge status. Missing handlers, unused handlers, unregistered commands."
                        />
                        <DashboardTab
                            name="Graph"
                            description="Interactive dependency graph (Cytoscape.js). Filter by component, search nodes, toggle dark mode."
                        />
                        <DashboardTab
                            name="Tree"
                            description="File tree with LOC per file/directory. Searchable, shows aggregated stats."
                        />
                    </div>
                </div>

                // MCP Server
                <div class="manual-section">
                    <h2 class="manual-title">"MCP Server (AI Integration)"</h2>
                    <p class="manual-description">
                        "Native integration with Claude, Cursor, and other MCP-compatible AI tools."
                    </p>
                    <div class="mcp-setup">
                        <h3>"Setup"</h3>
                        <div class="code-block">
                            <pre>{r#"# In Claude Desktop config (~/.config/claude/claude_desktop_config.json)
{
  "mcpServers": {
    "loctree": {
      "command": "loctree-mcp",
      "args": ["stdio"]
    }
  }
}"#}</pre>
                        </div>
                        <h3>"Available Tools"</h3>
                        <div class="mcp-tools">
                            <div class="mcp-tool">
                                <code>"get_slice"</code>
                                <span>"Holographic context for a file"</span>
                            </div>
                            <div class="mcp-tool">
                                <code>"check_dead"</code>
                                <span>"Find dead exports"</span>
                            </div>
                            <div class="mcp-tool">
                                <code>"check_cycles"</code>
                                <span>"Detect circular imports"</span>
                            </div>
                            <div class="mcp-tool">
                                <code>"find_symbol"</code>
                                <span>"Search for symbol definitions"</span>
                            </div>
                            <div class="mcp-tool">
                                <code>"who_imports"</code>
                                <span>"Find files importing a target"</span>
                            </div>
                            <div class="mcp-tool">
                                <code>"project_info"</code>
                                <span>"Codebase overview stats"</span>
                            </div>
                        </div>
                    </div>
                </div>

                // CI Integration
                <div class="manual-section">
                    <h2 class="manual-title">"CI Integration"</h2>
                    <div class="ci-examples">
                        <div class="ci-example">
                            <h3>"GitHub Actions"</h3>
                            <div class="code-block">
                                <pre>{r#"- name: Install loctree
  run: cargo install loctree

- name: Check dead code
  run: loct dead --confidence high --fail

- name: Check cycles
  run: loct cycles --fail

- name: Upload SARIF
  run: loct lint --sarif > results.sarif

- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: results.sarif"#}</pre>
                            </div>
                        </div>
                        <div class="ci-flags">
                            <h3>"CI Flags"</h3>
                            <div class="flag-list">
                                <div class="flag-item">
                                    <code>"--fail"</code>
                                    <span>"Exit non-zero if issues found"</span>
                                </div>
                                <div class="flag-item">
                                    <code>"--sarif"</code>
                                    <span>"Output SARIF 2.1.0 for GitHub/GitLab"</span>
                                </div>
                                <div class="flag-item">
                                    <code>"--json"</code>
                                    <span>"Machine-readable JSON output"</span>
                                </div>
                                <div class="flag-item">
                                    <code>"--quiet"</code>
                                    <span>"Suppress progress output"</span>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                // Supported Languages
                <div class="manual-section">
                    <h2 class="manual-title">"Supported Languages"</h2>
                    <div class="languages-grid">
                        <div class="lang-card full">"TypeScript / JavaScript"<span>"Full support including path aliases, barrel files"</span></div>
                        <div class="lang-card full">"Rust"<span>"Crate analysis, mod.rs detection"</span></div>
                        <div class="lang-card full">"Python"<span>"__init__.py, TYPE_CHECKING blocks"</span></div>
                        <div class="lang-card partial">"CSS / SCSS"<span>"Import tracking"</span></div>
                        <div class="lang-card partial">"Vue / Svelte"<span>"Script section extraction"</span></div>
                    </div>
                </div>
            </div>
        </section>
    }
}

#[component]
fn ConceptCard(
    title: &'static str,
    icon: &'static str,
    description: &'static str,
    cmd: &'static str,
) -> impl IntoView {
    let icon_class = format!("concept-icon icon-{}", icon);
    view! {
        <div class="concept-card">
            <div class=icon_class></div>
            <h3 class="concept-title">{title}</h3>
            <p class="concept-desc">{description}</p>
            <code class="concept-cmd">{cmd}</code>
        </div>
    }
}

#[component]
fn DashboardTab(name: &'static str, description: &'static str) -> impl IntoView {
    view! {
        <div class="dashboard-tab">
            <h4 class="tab-name">{name}</h4>
            <p class="tab-desc">{description}</p>
        </div>
    }
}
