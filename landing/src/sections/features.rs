use super::VERSION;
use leptos::prelude::*;

#[component]
pub fn Features() -> impl IntoView {
    let eyebrow = format!("{} Features", VERSION);
    view! {
        <section id="features" class="features">
            <div class="container">
                <div class="section-header">
                    <p class="section-eyebrow">{eyebrow}</p>
                    <h2 class="section-title">"Zombies? Not on my tree!"</h2>
                    <p class="section-description">
                        "Static code analysis tool built for agentic codebase context management. "
                        "Dead Parrots, Ministry of Silly Exports, circular imports — we hunt them all."
                    </p>
                </div>
                <div class="features-grid">
                    <FeatureCard
                        icon="[1]"
                        title="Holographic Slice"
                        description="3-layer context extraction: Core (target), Deps (imports), Consumers (what uses it). Pipe directly to AI."
                        code=Some("loct slice src/App.tsx --consumers --json")
                    />
                    <FeatureCard
                        icon="[2]"
                        title="Multi-Language Support"
                        description="Rust, Go, TypeScript/JavaScript, Python, Svelte, Vue, Dart/Flutter. Auto-detects stack and configures sensible ignores."
                        code=Some("Supports 7+ languages with library mode")
                    />
                    <FeatureCard
                        icon="[3]"
                        title="Crowd Detection"
                        description="Find files clustering around the same thing. 5 hooks all doing 'auth'? Ministry of Silly Exports? We'll find them."
                        code=Some("loct crowd auth")
                    />
                    <FeatureCard
                        icon="[4]"
                        title="Circular Import Detection"
                        description="Find circular dependencies that compile but break at runtime. Uses SCC algorithm."
                        code=Some("loct cycles")
                    />
                    <FeatureCard
                        icon="[5]"
                        title="Dead Parrot Detection"
                        description="Find exports that look alive but nobody imports. \"It's not dead, it's resting!\" No, it's dead. Remove it."
                        code=Some("loct dead --confidence high")
                    />
                    <FeatureCard
                        icon="[6]"
                        title="Impact Analysis"
                        description="See what breaks if you change a file. Understand dependencies before refactoring."
                        code=Some("loct find --impact src/utils/api.ts")
                    />
                    <FeatureCard
                        icon="[7]"
                        title="Symbol Search"
                        description="Find where any symbol is defined and used across the codebase."
                        code=Some("loct find --symbol useAuth")
                    />
                    <FeatureCard
                        icon="[8]"
                        title="Incremental Scanning"
                        description="After first scan, uses mtime to skip unchanged files. Typical: \"47 cached, 3 fresh\"."
                        code=Some("--full-scan to force re-analysis")
                    />
                    <FeatureCard
                        icon="[9]"
                        title="Entry Points"
                        description="List all main functions and script entry points in your project."
                        code=Some("loct lint --entrypoints")
                    />
                    <FeatureCard
                        icon="[10]"
                        title="Tauri Bridge Analysis"
                        description="FE-BE coverage. Matches invoke() calls to #[tauri::command]. Handles rename attributes and semantic matching."
                        code=Some("loct commands")
                    />
                    <FeatureCard
                        icon="[11]"
                        title="Handler Tracing"
                        description="Trace why a handler appears unused. Shows string literals, exports, and dynamic usage patterns."
                        code=Some("loct commands --unused")
                    />
                    <FeatureCard
                        icon="[12]"
                        title="Confidence Scoring"
                        description="HIGH/LOW confidence for unused handlers. LOW = potential dynamic usage detected. Filter with --confidence."
                        code=Some("loct dead --confidence high")
                    />
                    <FeatureCard
                        icon="[13]"
                        title="CI Pipeline Checks"
                        description="Fail builds on missing handlers, ghost events, or race conditions."
                        code=Some("loct lint --fail")
                    />
                    <FeatureCard
                        icon="[14]"
                        title="SARIF Output"
                        description="SARIF 2.1.0 output for GitHub Actions, GitLab CI, and other CI/CD systems."
                        code=Some("loct lint --sarif > results.sarif")
                    />
                    <FeatureCard
                        icon="[15]"
                        title="Query API"
                        description="Quick graph queries: who-imports, where-symbol, component-of. Fast answers without full analysis."
                        code=Some("loct query who-imports src/utils.ts")
                    />
                    <FeatureCard
                        icon="[16]"
                        title="Snapshot Diff"
                        description="Compare snapshots to see what changed. Track new cycles, dead exports, and graph changes."
                        code=Some("loct diff --since main")
                    />
                    <FeatureCard
                        icon="[17]"
                        title="IDE Integration URLs"
                        description="SARIF includes loctree://open URLs for direct file:line navigation in VS Code, JetBrains, etc."
                        code=Some("loctree://open?f=src/app.ts&l=42")
                    />
                    <FeatureCard
                        icon="[18]"
                        title="MCP Server"
                        description="Model Context Protocol server for AI agents. Claude, Cursor, and other MCP clients get native access to slices, dead code, and crowd detection."
                        code=Some("loctree-mcp stdio")
                    />
                    <FeatureCard
                        icon="[19]"
                        title="Bundle Distribution Analysis"
                        description="Verify tree-shaking effectiveness. Parse source maps with VLQ decoding to find exports that exist in source but are eliminated from production bundles."
                        code=Some("loct dist dist/bundle.js.map src/")
                    />
                </div>
            </div>
        </section>
    }
}

#[component]
fn FeatureCard(
    icon: &'static str,
    title: &'static str,
    description: &'static str,
    code: Option<&'static str>,
) -> impl IntoView {
    let (copied, set_copied) = signal(false);

    view! {
        <article class="feature-card">
            <div class="feature-icon">{icon}</div>
            <h3 class="feature-title">{title}</h3>
            <p class="feature-description">{description}</p>
            {code.map(|c| {
                let code_text = c;
                let copy_code = move |_| {
                    if let Some(window) = web_sys::window() {
                        let clipboard = window.navigator().clipboard();
                        let _ = clipboard.write_text(code_text);
                        set_copied.set(true);
                        set_timeout(
                            move || set_copied.set(false),
                            std::time::Duration::from_millis(1500),
                        );
                    }
                };
                view! {
                    <div class="feature-code-box">
                        <code class="feature-code-text">{c}</code>
                        <button class="feature-copy-btn" on:click=copy_code>
                            {move || if copied.get() { "ok" } else { "cp" }}
                        </button>
                    </div>
                }
            })}
        </article>
    }
}
