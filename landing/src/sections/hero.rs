use leptos::prelude::*;
use super::VERSION;

#[component]
pub fn Hero() -> impl IntoView {
    let badge_text = format!("{} — Rust 2024 Edition", VERSION);
    view! {
        <section class="hero">
            <div class="container">
                <div class="hero-grid">
                    <div class="hero-content">
                        <div class="hero-badge">
                            <span class="hero-badge-dot"></span>
                            {badge_text}
                        </div>
                        <h1 class="hero-title">
                            <span class="hero-title-accent">"The project map"</span>
                            <br />
                            "built for AI, not humans."
                        </h1>
                        <p class="hero-description">
                            "Dead code dies here. Static code analysis for agentic codebase context management. "
                            "Real structure, not file listings. A foundation for autonomous development tools."
                        </p>
                        <div class="hero-actions">
                            <a href="#install" class="btn btn-primary">
                                "Get Started"
                            </a>
                            <a href="https://github.com/LibraxisAI/loctree" target="_blank" class="btn btn-secondary">
                                "View on GitHub →"
                            </a>
                        </div>
                    </div>
                    <Terminal />
                </div>
            </div>
        </section>
    }
}

#[component]
fn Terminal() -> impl IntoView {
    view! {
        <div class="hero-terminal">
            <div class="terminal-header">
                <div class="terminal-dot red"></div>
                <div class="terminal-dot yellow"></div>
                <div class="terminal-dot green"></div>
                <span class="terminal-title">"~/my-project"</span>
            </div>
            <div class="terminal-body">
                // First command
                <div class="terminal-line">
                    <span class="terminal-prompt">"$"</span>
                    <span class="terminal-command">"loctree"</span>
                </div>
                <div class="terminal-output muted">"[loctree][detect] Detected: Tauri + Vite"</div>
                <div class="terminal-output muted">"[loctree][progress] 47 cached, 3 fresh"</div>
                <div class="terminal-output success">"✓ Snapshot saved to .loctree/snapshot.json"</div>

                // Second command
                <div class="terminal-line" style="margin-top: 16px;">
                    <span class="terminal-prompt">"$"</span>
                    <span class="terminal-command">"loctree slice src/App.tsx --consumers"</span>
                </div>

                <div class="terminal-output highlight" style="margin-top: 8px;">
                    "Slice for: src/App.tsx"
                </div>
                <div class="terminal-output" style="margin-top: 8px;">
                    "Core (1 files, 150 LOC):"
                </div>
                <div class="terminal-output">"  src/App.tsx"</div>

                <div class="terminal-output" style="margin-top: 8px;">
                    "Deps (3 files, 420 LOC):"
                </div>
                <div class="terminal-output">"  [d1] src/hooks/useAuth.ts"</div>
                <div class="terminal-output muted">"    [d2] src/contexts/AuthContext.tsx"</div>
                <div class="terminal-output" style="margin-top: 8px;">
                    "Consumers (2 files, 180 LOC):"
                </div>
                <div class="terminal-output">"  src/main.tsx"</div>
                <div class="terminal-output">"  src/routes/index.tsx"</div>
                <div class="terminal-output success" style="margin-top: 8px;">
                    "Total: 6 files, 750 LOC"
                </div>
            </div>
        </div>
    }
}
