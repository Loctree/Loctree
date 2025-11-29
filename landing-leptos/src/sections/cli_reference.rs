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
                            <code class="cli-cmd">"loctree"</code>
                            <span class="cli-desc">"Scan and save snapshot (default)"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loctree slice <file>"</code>
                            <span class="cli-desc">"Holographic slice for AI context"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"loctree -A"</code>
                            <span class="cli-desc">"Import/export analyzer mode"</span>
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
                        <h3 class="cli-group-title">"Analyzer Options (-A)"</h3>
                        <div class="cli-item">
                            <code class="cli-cmd">"--circular"</code>
                            <span class="cli-desc">"Find circular imports"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"--dead"</code>
                            <span class="cli-desc">"Find unused exports"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"--entrypoints"</code>
                            <span class="cli-desc">"List entry points"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"--check <query>"</code>
                            <span class="cli-desc">"Find similar components"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"--impact <file>"</code>
                            <span class="cli-desc">"Show what imports target"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"--symbol <name>"</code>
                            <span class="cli-desc">"Search for symbol"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"--sarif"</code>
                            <span class="cli-desc">"SARIF 2.1.0 output for CI"</span>
                        </div>
                    </div>

                    <div class="cli-group">
                        <h3 class="cli-group-title">"Pipeline Checks"</h3>
                        <div class="cli-item">
                            <code class="cli-cmd">"--fail-on-missing-handlers"</code>
                            <span class="cli-desc">"FE->BE integrity"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"--fail-on-ghost-events"</code>
                            <span class="cli-desc">"Unused events"</span>
                        </div>
                        <div class="cli-item">
                            <code class="cli-cmd">"--fail-on-races"</code>
                            <span class="cli-desc">"Race conditions"</span>
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
