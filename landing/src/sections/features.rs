use leptos::prelude::*;
use super::VERSION;

#[component]
pub fn Features() -> impl IntoView {
    let eyebrow = format!("{} Features", VERSION);
    view! {
        <section id="features" class="features">
            <div class="container">
                <div class="section-header">
                    <p class="section-eyebrow">{eyebrow}</p>
                    <h2 class="section-title">"Built for AI agents and vibe-coders"</h2>
                    <p class="section-description">
                        "Solve context drift. Find existing components before creating duplicates. "
                        "Expose hidden dependencies. Slice relevant context."
                    </p>
                </div>
                <div class="features-grid">
                    <FeatureCard
                        icon="[1]"
                        title="Holographic Slice"
                        description="3-layer context extraction: Core (target), Deps (imports), Consumers (what uses it). Pipe directly to AI."
                        code=Some("loctree slice src/App.tsx --consumers --json")
                    />
                    <FeatureCard
                        icon="[2]"
                        title="Auto-detect Stack"
                        description="Detects Rust, TypeScript, Python, Tauri. Configures sensible ignores automatically."
                        code=Some("Cargo.toml -> Rust | tsconfig.json -> TS")
                    />
                    <FeatureCard
                        icon="[3]"
                        title="Duplicate Detection"
                        description="Before creating new components, find similar existing ones. Prevents AI code duplication."
                        code=Some("loct find --similar ChatSurface")
                    />
                    <FeatureCard
                        icon="[4]"
                        title="Circular Import Detection"
                        description="Find circular dependencies that compile but break at runtime. Uses SCC algorithm."
                        code=Some("loct cycles")
                    />
                    <FeatureCard
                        icon="[5]"
                        title="Dead Code Detection"
                        description="Find unused exports and orphaned code. Clean up before it becomes tech debt."
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
