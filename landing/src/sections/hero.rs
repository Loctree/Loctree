use super::VERSION;
use leptos::prelude::*;

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
                            <span class="hero-title-accent">"Dead code dies here."</span>
                        </h1>
                        <p class="hero-description">
                            "Static code analysis tool built for agentic codebase context management. "
                            "Find Dead Parrots, Ministry of Silly Exports, and 148-file features before they haunt you."
                        </p>
                        <p class="hero-description hero-byline">
                            "Developed with 💀 by Loctree Team"
                        </p>
                        <div class="hero-actions">
                            <a href="#install" class="btn btn-primary">
                                "Get Started"
                            </a>
                            <a href="https://github.com/Loctree/Loctree" target="_blank" class="btn btn-secondary">
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
                // First command - crowd detection
                <div class="terminal-line">
                    <span class="terminal-prompt">"$"</span>
                    <span class="terminal-command">"loct crowd assistant"</span>
                </div>
                <div class="terminal-output highlight" style="margin-top: 8px;">
                    "╭─ CROWD: \"assistant\" ────────────────────╮"
                </div>
                <div class="terminal-output warning">
                    "│ Crowd Score: 10.0/10 (HIGH - oh no!)"
                </div>
                <div class="terminal-output">"│"</div>
                <div class="terminal-output">"│ FILES IN CROWD (38 files)"</div>
                <div class="terminal-output">"│   AssistantHost.tsx    ████ 50 importers"</div>
                <div class="terminal-output">"│   useAssistant.ts      ██   12 importers"</div>
                <div class="terminal-output error">"│   useAssistantOld.ts        0 importers"</div>
                <div class="terminal-output error">"│   AssistantLegacy.tsx       0 importers"</div>
                <div class="terminal-output">"│"</div>
                <div class="terminal-output warning">"│ DEAD PARROTS DETECTED: 2"</div>
                <div class="terminal-output highlight">
                    "╰──────────────────────────────────────────╯"
                </div>

                // Second command - dead exports
                <div class="terminal-line" style="margin-top: 16px;">
                    <span class="terminal-prompt">"$"</span>
                    <span class="terminal-command">"loct dead"</span>
                </div>
                <div class="terminal-output error" style="margin-top: 8px;">
                    "Dead Exports (14 found):"
                </div>
                <div class="terminal-output">"  - formatLegacyDate in utils.ts:42"</div>
                <div class="terminal-output">"  - OldPatientForm in forms/index.ts:7"</div>
                <div class="terminal-output muted">"  ... and 12 more corpses"</div>
                <div class="terminal-output success" style="margin-top: 8px;">
                    "Time to clean up the morgue!"
                </div>
            </div>
        </div>
    }
}
