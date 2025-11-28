// loctree Landing Page — Leptos 0.8 Edition
// Created by M&K (c)2025 The LibraxisAI Team

use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    view! {
        <Nav />
        <main>
            <AgentPromptHero />
            <Hero />
            <Features />
            <SliceDemo />
            <StackDetection />
            <InstallSection />
        </main>
        <Footer />
    }
}

// ============================================
// Install Hero — Tabbed: For Humans / For AI Agents
// ============================================
#[component]
fn AgentPromptHero() -> impl IntoView {
    let (active_tab, set_active_tab) = signal("agents");
    let (copied, set_copied) = signal(false);

    let human_content = r#"# Quick Start

# Install
curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install.sh | sh

# Full analysis (auto-detects stack)
loctree . -A

# Slice context for a file
loctree slice src/App.tsx --consumers"#;

    let agent_prompt = r#"## loctree — Build tool awareness

Install and scan:
```bash
curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install.sh | sh
loctree . -A
```

Save to .claude/LOCTREE.md or equivalent:

### Before creating new components
`loctree -A --check-sim <Name>` — find similar code first
`loctree -A --symbol <Name>` — find symbol usages

### Before refactoring
`loctree slice <file> --consumers` — see what depends on it
`loctree -A --circular` — detect import cycles

### Dead code detection
`loctree -A --dead` — find unused exports
`loctree -A --entrypoints` — find main/script entry points

### For focused context
`loctree slice <file> --consumers --json`

### Tauri FE<>BE coverage
`loctree -A --preset-tauri` — command/event bridge analysis

### CI checks
`loctree -A --fail-on-missing-handlers --sarif`"#;

    let copy_content = move |_| {
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let content = if active_tab.get() == "humans" {
                human_content
            } else {
                agent_prompt
            };
            let _ = clipboard.write_text(content);
            set_copied.set(true);
            set_timeout(
                move || set_copied.set(false),
                std::time::Duration::from_millis(2000),
            );
        }
    };

    view! {
        <section class="agent-hero">
            <div class="container">
                <div class="agent-hero-inner">
                    // Tab switcher
                    <div class="hero-tabs">
                        <button
                            class=move || if active_tab.get() == "humans" { "hero-tab active" } else { "hero-tab" }
                            on:click=move |_| set_active_tab.set("humans")
                        >
                            "FOR HUMANS"
                        </button>
                        <button
                            class=move || if active_tab.get() == "agents" { "hero-tab active" } else { "hero-tab" }
                            on:click=move |_| set_active_tab.set("agents")
                        >
                            <span class="blink">"_"</span>
                            " FOR AI AGENTS"
                        </button>
                    </div>

                    // Content
                    <Show when=move || active_tab.get() == "humans">
                        <pre class="agent-hero-prompt">{human_content}</pre>
                    </Show>
                    <Show when=move || active_tab.get() == "agents">
                        <pre class="agent-hero-prompt">{agent_prompt}</pre>
                    </Show>

                    <button class="copy-btn" on:click=copy_content>
                        {move || if copied.get() { "COPIED" } else { "COPY TO CLIPBOARD" }}
                    </button>
                </div>
            </div>
        </section>
    }
}

// ============================================
// Navigation
// ============================================
#[component]
fn Nav() -> impl IntoView {
    view! {
        <nav class="nav">
            <div class="nav-inner">
                <a href="/" class="nav-brand">
                    <div class="nav-logo">
                        <img src="assets/loctree-logo-brutal.svg" alt="loctree" />
                    </div>
                    <span class="nav-title">"loctree"</span>
                    <span class="nav-version">"v0.5.0-rc"</span>
                </a>
                <div class="nav-links">
                    <a href="#features" class="nav-link">"Features"</a>
                    <a href="#slice" class="nav-link">"Slice Mode"</a>
                    <a href="https://github.com/LibraxisAI/loctree" target="_blank" class="nav-link">"GitHub"</a>
                    <a href="#install" class="nav-cta">"Install"</a>
                </div>
            </div>
        </nav>
    }
}

// ============================================
// Hero Section
// ============================================
#[component]
fn Hero() -> impl IntoView {
    view! {
        <section class="hero">
            <div class="container">
                <div class="hero-grid">
                    <div class="hero-content">
                        <div class="hero-badge">
                            <span class="hero-badge-dot"></span>
                            "v0.5.0-rc — Holographic Slice is here"
                        </div>
                        <h1 class="hero-title">
                            <span class="hero-title-accent">"Scan once,"</span>
                            <br />
                            "slice many."
                        </h1>
                        <p class="hero-description">
                            "Static analysis tool designed for AI agents. Extract focused context for any file, "
                            "detect circular imports, find dead exports. One scan, infinite slices."
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

// ============================================
// Terminal (static version - animation via CSS)
// ============================================
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

// ============================================
// Features Section
// ============================================
#[component]
fn Features() -> impl IntoView {
    view! {
        <section id="features" class="features">
            <div class="container">
                <div class="section-header">
                    <p class="section-eyebrow">"Features"</p>
                    <h2 class="section-title">"Built for AI agents and vibe-coders"</h2>
                    <p class="section-description">
                        "Solve context drift. Find existing components before creating duplicates. "
                        "Expose hidden dependencies. Slice relevant context."
                    </p>
                </div>
                <div class="features-grid">
                    <FeatureCard
                        icon="🔬"
                        icon_class="green"
                        title="Holographic Slice"
                        description="Extract 3-layer context for any file: Core (target), Deps (imports), Consumers (what uses it). Perfect for AI conversations."
                        code=Some("loctree slice src/App.tsx --consumers --json | claude")
                    />
                    <FeatureCard
                        icon="🔍"
                        icon_class="cyan"
                        title="Auto-detect Stack"
                        description="Automatically detects Rust, TypeScript, Python, or Tauri projects. Configures sensible ignores (node_modules, target, .venv)."
                        code=None
                    />
                    <FeatureCard
                        icon="🔄"
                        icon_class="purple"
                        title="Circular Import Detection"
                        description="Find circular dependencies that compile but break at runtime. Uses SCC algorithm for accurate cycle detection."
                        code=Some("loctree -A --circular")
                    />
                    <FeatureCard
                        icon="🧹"
                        icon_class="orange"
                        title="Janitor Mode"
                        description="Find dead exports, check for similar components before creating new ones, analyze impact of changes."
                        code=Some("loctree -A --dead --check ChatSurface")
                    />
                    <FeatureCard
                        icon="⚡"
                        icon_class="green"
                        title="Incremental Scanning"
                        description="After first scan, uses file mtime to skip unchanged files. Typical re-scans: \"47 cached, 3 fresh\"."
                        code=None
                    />
                    <FeatureCard
                        icon="🔗"
                        icon_class="cyan"
                        title="CI Pipeline Checks"
                        description="Fail builds on missing handlers, ghost events, or race conditions. SARIF output for GitHub/GitLab integration."
                        code=Some("loctree -A --fail-on-missing-handlers --sarif")
                    />
                </div>
            </div>
        </section>
    }
}

#[component]
fn FeatureCard(
    icon: &'static str,
    icon_class: &'static str,
    title: &'static str,
    description: &'static str,
    code: Option<&'static str>,
) -> impl IntoView {
    view! {
        <article class="feature-card">
            <div class=format!("feature-icon {}", icon_class)>
                {icon}
            </div>
            <h3 class="feature-title">{title}</h3>
            <p class="feature-description">{description}</p>
            {code.map(|c| view! {
                <div class="feature-code">{c}</div>
            })}
        </article>
    }
}

// ============================================
// Slice Demo Section
// ============================================
#[component]
fn SliceDemo() -> impl IntoView {
    view! {
        <section id="slice" class="slice-demo">
            <div class="container">
                <div class="slice-grid">
                    <div class="slice-content">
                        <p class="section-eyebrow">"Holographic Slice"</p>
                        <h2 class="section-title">"3-layer context extraction"</h2>
                        <p class="hero-description">
                            "Give your AI agent exactly what it needs. Core file, its dependencies, "
                            "and everything that consumes it. No more, no less."
                        </p>
                        <div class="hero-actions" style="margin-top: 24px;">
                            <a href="https://github.com/LibraxisAI/loctree#holographic-slice-slice-command" target="_blank" class="btn btn-secondary">
                                "Read the docs →"
                            </a>
                        </div>
                    </div>
                    <div class="slice-visual">
                        <div class="slice-layers">
                            <div class="slice-layer core">
                                <div class="slice-layer-header">
                                    <span class="slice-layer-title">"Core"</span>
                                    <span class="slice-layer-stats">"1 file, 150 LOC"</span>
                                </div>
                                <div class="slice-layer-files">
                                    <div>"src/App.tsx"</div>
                                </div>
                            </div>
                            <div class="slice-layer deps">
                                <div class="slice-layer-header">
                                    <span class="slice-layer-title">"Deps"</span>
                                    <span class="slice-layer-stats">"3 files, 420 LOC"</span>
                                </div>
                                <div class="slice-layer-files">
                                    <div>"[d1] src/hooks/useAuth.ts"</div>
                                    <div class="indent-1">"[d2] src/contexts/AuthContext.tsx"</div>
                                    <div class="indent-1">"[d2] src/utils/api.ts"</div>
                                </div>
                            </div>
                            <div class="slice-layer consumers">
                                <div class="slice-layer-header">
                                    <span class="slice-layer-title">"Consumers"</span>
                                    <span class="slice-layer-stats">"2 files, 180 LOC"</span>
                                </div>
                                <div class="slice-layer-files">
                                    <div>"src/main.tsx"</div>
                                    <div>"src/routes/index.tsx"</div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    }
}

// ============================================
// Stack Detection Section
// ============================================
#[component]
fn StackDetection() -> impl IntoView {
    view! {
        <section class="stack-detection">
            <div class="container">
                <div class="section-header">
                    <p class="section-eyebrow">"Auto-detect"</p>
                    <h2 class="section-title">"Zero configuration needed"</h2>
                    <p class="section-description">
                        "loctree detects your project type and configures sensible defaults automatically."
                    </p>
                </div>
                <div class="stack-grid">
                    <StackCard icon="🦀" name="Rust" marker="Cargo.toml" ignores="target/" />
                    <StackCard icon="📘" name="TypeScript" marker="tsconfig.json" ignores="node_modules/" />
                    <StackCard icon="🐍" name="Python" marker="pyproject.toml" ignores=".venv/" />
                    <StackCard icon="⚡" name="Tauri" marker="src-tauri/" ignores="all of the above" />
                    <StackCard icon="🟨" name="Vite" marker="vite.config.*" ignores="dist/" />
                </div>
            </div>
        </section>
    }
}

#[component]
fn StackCard(
    icon: &'static str,
    name: &'static str,
    marker: &'static str,
    ignores: &'static str,
) -> impl IntoView {
    view! {
        <div class="stack-card">
            <div class="stack-icon">{icon}</div>
            <div class="stack-name">{name}</div>
            <div class="stack-marker">{marker}</div>
            <div class="stack-ignores">"→ ignores "{ignores}</div>
        </div>
    }
}

// ============================================
// Install Section — Simple install for humans
// ============================================
#[component]
fn InstallSection() -> impl IntoView {
    let (copied, set_copied) = signal(false);

    let install_command = "curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install.sh | sh";

    let copy_cmd = move |_| {
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(install_command);
            set_copied.set(true);
            set_timeout(
                move || set_copied.set(false),
                std::time::Duration::from_millis(2000),
            );
        }
    };

    view! {
        <section id="install" class="install-section">
            <div class="container">
                <div class="install-box-simple">
                    <div class="install-label">"INSTALL"</div>
                    <div class="install-command-box">
                        <code class="install-cmd">{install_command}</code>
                        <button class="copy-btn-small" on:click=copy_cmd>
                            {move || if copied.get() { "✓" } else { "COPY" }}
                        </button>
                    </div>
                    <div class="install-links">
                        <a href="https://github.com/LibraxisAI/loctree" target="_blank">"GitHub"</a>
                        <span class="sep">"|"</span>
                        <a href="https://crates.io/crates/loctree" target="_blank">"crates.io"</a>
                        <span class="sep">"|"</span>
                        <a href="https://github.com/LibraxisAI/loctree#readme" target="_blank">"Docs"</a>
                    </div>
                </div>
            </div>
        </section>
    }
}

// ============================================
// Footer
// ============================================
#[component]
fn Footer() -> impl IntoView {
    view! {
        <footer class="footer">
            <div class="container">
                <div class="footer-brand">
                    <span class="footer-logo">
                        <img src="assets/loctree-logo-brutal.svg" alt="loctree" />
                    </span>
                    <span class="footer-title">"loctree"</span>
                </div>
                <div class="footer-links">
                    <a href="https://github.com/LibraxisAI/loctree" target="_blank" class="footer-link">"GitHub"</a>
                    <a href="https://crates.io/crates/loctree" target="_blank" class="footer-link">"crates.io"</a>
                    <a href="https://github.com/LibraxisAI/loctree/blob/main/LICENSE" target="_blank" class="footer-link">"MIT License"</a>
                </div>
                <p class="footer-copyright">
                    "Created by M&K (c)2025 The LibraxisAI Team"
                </p>
            </div>
        </footer>
    }
}
