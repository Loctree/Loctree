use leptos::prelude::*;

#[component]
pub fn CliReference() -> impl IntoView {
    view! {
        <section id="cli" class="cli-reference">
            <div class="container">
                <div class="section-header">
                    <p class="section-eyebrow">"CLI Reference"</p>
                    <h2 class="section-title">"Full command reference"</h2>
                </div>
                <div class="cli-grid">
                    <div class="cli-group">
                        <h3 class="cli-group-title">"Modes"</h3>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct"</code>
                            <span class="cli-desc">"Auto scan + snapshot (use `loct report --serve` for HTML)"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct slice <file>"</code>
                            <span class="cli-desc">"Holographic slice for AI context"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct find --impact <file>"</code>
                            <span class="cli-desc">"Blast radius / dependency impact"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct dead"</code>
                            <span class="cli-desc">"Unused exports (alias/barrel aware)"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct cycles"</code>
                            <span class="cli-desc">"Detect circular imports"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct commands"</code>
                            <span class="cli-desc">"Tauri FE↔BE coverage (missing/unused)"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct events"</code>
                            <span class="cli-desc">"Emit/listen/races summary"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct query <kind> <target>"</code>
                            <span class="cli-desc">"Quick queries (who-imports, where-symbol)"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct diff --since <id>"</code>
                            <span class="cli-desc">"Compare snapshots, show delta"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct dist <map> <src>"</code>
                            <span class="cli-desc">"Bundle analysis via source maps"</span>
                        </div>
                    </div>

                    <div class="cli-group">
                        <h3 class="cli-group-title">"Slice Options"</h3>
                        <div class="cli-item">
                            <code class="cli-cmd">"--consumers"</code>
                            <span class="cli-desc">"Include files that import target"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"--json"</code>
                            <span class="cli-desc">"JSON output for piping to AI"</span>
                        </div>
                    </div>

                    <div class="cli-group">
                        <h3 class="cli-group-title">"Find / Analyze"</h3>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct find <query>"</code>
                            <span class="cli-desc">"One command: fuzzy match + defs/uses + dead status"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct query who-imports <file>"</code>
                            <span class="cli-desc">"Quick: files that import target"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct query where-symbol <name>"</code>
                            <span class="cli-desc">"Quick: where symbol is defined"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct dead --confidence high"</code>
                            <span class="cli-desc">"Unused exports with stricter filter"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct report --serve"</code>
                            <span class="cli-desc">"HTML with embedded dependency graph + local server"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct lint --sarif"</code>
                            <span class="cli-desc">"SARIF 2.1.0 output for CI"</span>
                        </div>
                    </div>

                    <div class="cli-group">
                        <h3 class="cli-group-title">"Pipeline Checks"</h3>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct lint --fail"</code>
                            <span class="cli-desc">"Fail CI on missing/ghost handlers"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct commands --missing"</code>
                            <span class="cli-desc">"List FE calls without handlers"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loct commands --unused"</code>
                            <span class="cli-desc">"Handlers without FE calls"</span>
                        </div>
                    </div>

                    <div class="cli-group">
                        <h3 class="cli-group-title">"Common Flags"</h3>
                        <div class="cli-item">
                            <code class="cli-cmd">"-g, --gitignore"</code>
                            <span class="cli-desc">"Respect .gitignore"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"--full-scan"</code>
                            <span class="cli-desc">"Ignore mtime cache"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"--verbose"</code>
                            <span class="cli-desc">"Detailed progress"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"--preset-tauri"</code>
                            <span class="cli-desc">"Tauri FE-BE mode"</span>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    }
}
