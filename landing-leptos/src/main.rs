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
            <Hero />
            <Features />
            <SliceDemo />
            <StackDetection />
            <CliReference />
            <InstallSection />
        </main>
        <Footer />
    }
}

// ============================================
// Navigation with Dropdown Installer
// ============================================
#[component]
fn Nav() -> impl IntoView {
    let (drawer_open, set_drawer_open) = signal(false);
    let (active_tab, set_active_tab) = signal("humans");
    let (install_copied, set_install_copied) = signal(false);
    let (prompt_copied, set_prompt_copied) = signal(false);
    let (expanded, set_expanded) = signal(false);

    let install_command = "curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install.sh | sh";

    let agent_prompt_short = r#"## loctree — AI Agent Quick Reference
**Scan once, slice many.** Install: `curl -fsSL .../install.sh | sh`

### Key Commands
- `loctree` — scan project, save to .loctree/snapshot.json
- `loctree slice <file> --consumers --json` — 3-layer context for AI
- `loctree -A --check <Name>` — find similar components (avoid duplicates)
- `loctree -A --symbol <Name>` — find symbol definitions & usage
- `loctree -A --circular` — detect import cycles
- `loctree -A --dead` — find unused exports
- `loctree -A --impact <file>` — what breaks if removed
- `loctree -A --entrypoints` — list main/script entry points
- `loctree -A --preset-tauri src src-tauri/src` — Tauri FE<>BE coverage
- `loctree -A --fail-on-missing-handlers --sarif` — CI checks"#;

    let agent_prompt_full = r#"## loctree — AI Agent Quick Reference

**Scan once, slice many.** Capture structure, extract focused context.

### Install & Scan
```bash
curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install.sh | sh
loctree          # creates .loctree/snapshot.json
```

### Before creating new components
```bash
loctree -A --check <Name>   # find similar (avoids duplicates)
loctree -A --symbol <Name>  # find symbol definitions & usage
```

### Before refactoring
```bash
loctree slice <file> --consumers  # what depends on it
loctree -A --impact <file>        # what breaks if removed
loctree -A --circular             # detect import cycles
```

### Dead code detection
```bash
loctree -A --dead                 # unused exports
loctree -A --entrypoints          # main/script entry points
```

### Focused AI context
```bash
loctree slice <file> --consumers --json | claude
```

### Tauri FE<>BE coverage
```bash
loctree -A --preset-tauri src src-tauri/src
```

### CI pipeline checks
```bash
loctree -A --fail-on-missing-handlers   # FE->BE integrity
loctree -A --fail-on-ghost-events       # unused events
loctree -A --sarif > results.sarif      # SARIF output
```"#;

    let copy_install = move |_| {
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(install_command);
            set_install_copied.set(true);
            set_timeout(
                move || set_install_copied.set(false),
                std::time::Duration::from_millis(2000),
            );
        }
    };

    let copy_prompt = move |_| {
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let content = if expanded.get() { agent_prompt_full } else { agent_prompt_short };
            let _ = clipboard.write_text(content);
            set_prompt_copied.set(true);
            set_timeout(
                move || set_prompt_copied.set(false),
                std::time::Duration::from_millis(2000),
            );
        }
    };

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
                    <a href="#slice" class="nav-link">"Slice"</a>
                    <a href="#cli" class="nav-link">"CLI"</a>
                    <a href="https://github.com/LibraxisAI/loctree" target="_blank" class="nav-link">"GitHub"</a>
                    <button
                        class=move || if drawer_open.get() { "nav-cta active" } else { "nav-cta" }
                        on:click=move |_| set_drawer_open.update(|o| *o = !*o)
                    >
                        {move || if drawer_open.get() { "Close" } else { "Install" }}
                    </button>
                </div>
            </div>

            // Dropdown drawer overlay
            <Show when=move || drawer_open.get()>
                <div class="nav-drawer">
                    <div class="nav-drawer-inner">
                        // Tab switcher
                        <div class="drawer-tabs">
                            <button
                                class=move || if active_tab.get() == "humans" { "drawer-tab active" } else { "drawer-tab" }
                                on:click=move |_| set_active_tab.set("humans")
                            >
                                "FOR HUMANS"
                            </button>
                            <button
                                class=move || if active_tab.get() == "agents" { "drawer-tab active" } else { "drawer-tab" }
                                on:click=move |_| set_active_tab.set("agents")
                            >
                                <span class="blink">"_"</span>
                                " FOR AI AGENTS"
                            </button>
                        </div>

                        // Content - FOR HUMANS
                        <Show when=move || active_tab.get() == "humans">
                            <div class="drawer-content">
                                <div class="code-block-with-copy">
                                    <code class="code-block-content">{install_command}</code>
                                    <button class="code-copy-btn" on:click=copy_install>
                                        {move || if install_copied.get() { "copied" } else { "copy" }}
                                    </button>
                                </div>
                                <p class="prompt-cta">
                                    "Or paste "
                                    <button class="prompt-link" on:click=move |_| set_active_tab.set("agents")>
                                        "this prompt"
                                    </button>
                                    " to your AI Agent. "
                                    <span class="prompt-cta-dim">"Loctree is agentic-friendly by design."</span>
                                </p>
                            </div>
                        </Show>

                        // Content - FOR AI AGENTS
                        <Show when=move || active_tab.get() == "agents">
                            <div class="drawer-content">
                                <div class="prompt-box">
                                    <div class="prompt-box-inner">
                                        <pre class="prompt-content">
                                            {move || if expanded.get() { agent_prompt_full } else { agent_prompt_short }}
                                        </pre>
                                        <button class="prompt-copy-btn" on:click=copy_prompt>
                                            {move || if prompt_copied.get() { "copied" } else { "copy" }}
                                        </button>
                                    </div>
                                    <button class="prompt-expand-btn" on:click=move |_| set_expanded.update(|e| *e = !*e)>
                                        {move || if expanded.get() { "[ - collapse ]" } else { "[ + expand full prompt ]" }}
                                    </button>
                                </div>
                            </div>
                        </Show>
                    </div>
                </div>
            </Show>
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
                    <p class="section-eyebrow">"v0.5.0-rc Features"</p>
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
                        code=Some("loctree -A --check ChatSurface")
                    />
                    <FeatureCard
                        icon="[4]"
                        title="Circular Import Detection"
                        description="Find circular dependencies that compile but break at runtime. Uses SCC algorithm."
                        code=Some("loctree -A --circular")
                    />
                    <FeatureCard
                        icon="[5]"
                        title="Dead Code Detection"
                        description="Find unused exports and orphaned code. Clean up before it becomes tech debt."
                        code=Some("loctree -A --dead --confidence high")
                    />
                    <FeatureCard
                        icon="[6]"
                        title="Impact Analysis"
                        description="See what breaks if you change a file. Understand dependencies before refactoring."
                        code=Some("loctree -A --impact src/utils/api.ts")
                    />
                    <FeatureCard
                        icon="[7]"
                        title="Symbol Search"
                        description="Find where any symbol is defined and used across the codebase."
                        code=Some("loctree -A --symbol useAuth")
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
                        code=Some("loctree -A --entrypoints")
                    />
                    <FeatureCard
                        icon="[10]"
                        title="Tauri Bridge Analysis"
                        description="FE<>BE coverage for Tauri projects. Find missing handlers and ghost events."
                        code=Some("loctree -A --preset-tauri src src-tauri/src")
                    />
                    <FeatureCard
                        icon="[11]"
                        title="CI Pipeline Checks"
                        description="Fail builds on missing handlers, ghost events, or race conditions."
                        code=Some("--fail-on-missing-handlers")
                    />
                    <FeatureCard
                        icon="[12]"
                        title="SARIF Output"
                        description="SARIF 2.1.0 output for GitHub Actions, GitLab CI, and other CI/CD systems."
                        code=Some("loctree -A --sarif > results.sarif")
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
// CLI Reference Section
// ============================================
#[component]
fn CliReference() -> impl IntoView {
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
                            <span class="cli-desc">"Tauri FE<>BE mode"</span>
                        </div>
                    </div>
                </div>
            </div>
        </section>
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
