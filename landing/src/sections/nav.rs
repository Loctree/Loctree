use leptos::prelude::*;
use super::VERSION;

#[component]
pub fn Nav() -> impl IntoView {
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
                    <span class="nav-version">{VERSION}</span>
                </a>
                <div class="nav-links">
                    <a href="#features" class="nav-link">"Features"</a>
                    <a href="#slice" class="nav-link">"Slice"</a>
                    <a href="#cli" class="nav-link">"CLI"</a>
                    <a href="https://docs.rs/loctree" target="_blank" class="nav-link">"Docs"</a>
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
