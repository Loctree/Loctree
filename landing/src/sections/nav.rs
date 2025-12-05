use super::VERSION;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Nav() -> impl IntoView {
    let (drawer_open, set_drawer_open) = signal(false);
    let (active_tab, set_active_tab) = signal("humans");
    let (install_copied, set_install_copied) = signal(false);
    let (prompt_copied, set_prompt_copied) = signal(false);
    let (expanded, set_expanded) = signal(false);

    let install_command = "cargo install loctree";

    let agent_prompt_short = r#"## loct — AI Agent Quick Reference
**Scan once, slice many.** Install: `cargo install loctree`

### Key Commands
- `loct` — scan repo, save snapshot + reports to .loctree/
- `loct query who-imports <file>` — fast dependency check
- `loct query where-symbol <name>` — find definitions
- `loct slice <file> --consumers --json` — 3-layer context
- `loct diff --since main` — compare branches
- `loct find --similar <Name>` — avoid duplicates
- `loct dead --confidence high` — unused exports
- `loct cycles` — circular imports
- `loct commands --missing/--unused` — Tauri FE↔BE
- `loct lint --fail --sarif` — CI guardrails"#;

    let agent_prompt_full = r#"## loct — AI Agent Quick Reference

**Scan once, slice many.** Default `loct` now writes snapshot + report bundle to .loctree/.

### Install & Scan
```bash
cargo install loctree
loct          # snapshot + report.html + analysis.json
```

### Quick Queries (no full scan)
```bash
loct query who-imports <file>     # files importing target
loct query where-symbol <name>    # find definitions
loct query component-of <file>    # graph component
```

### Before creating
```bash
loct find --similar <Name>   # find existing, avoid duplicates
loct query where-symbol <Name>   # check if exists
```

### Before refactoring
```bash
loct slice <file> --consumers --json  # context for AI
loct find --impact <file>             # blast radius
loct cycles                           # circular imports
```

### Compare branches
```bash
loct diff --since main        # delta from main
loct diff --since HEAD~5      # last 5 commits
```

### Hygiene
```bash
loct dead --confidence high   # unused exports
loct commands --missing       # FE calls without handlers
loct commands --unused        # handlers without FE calls
```

### CI + IDE
```bash
loct lint --fail --sarif > results.sarif
# loctree://open?f=<file>&l=<line> URLs in output
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
            let content = if expanded.get() {
                agent_prompt_full
            } else {
                agent_prompt_short
            };
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
                <A href="/" attr:class="nav-brand">
                    <div class="nav-logo">
                        <img src="/assets/loctree-logo.png" alt="loctree" />
                    </div>
                    <span class="nav-title">"loctree"</span>
                    <span class="nav-version">{VERSION}</span>
                </A>
                <div class="nav-links">
                    <A href="/features" attr:class="nav-link">"Features"</A>
                    <A href="/docs" attr:class="nav-link">"Docs"</A>
                    <A href="/blog" attr:class="nav-link">"Blog"</A>
                    <a href="https://github.com/Loctree/Loctree" target="_blank" class="nav-link">"GitHub"</a>
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
