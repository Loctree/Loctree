use leptos::prelude::*;

#[component]
pub fn ForAgents() -> impl IntoView {
    let (prompt_copied, set_prompt_copied) = signal(false);

    let agent_prompt = r#"## loctree Quick Reference

**Install:** `curl -fsSL https://loctree.io/install.sh | sh`

### Before creating components
```bash
loctree -A --check <Name>      # find similar existing
loctree -A --symbol <Name>     # find definitions & usage
```

### Before refactoring
```bash
loctree slice <file> --consumers --json  # what depends on it
loctree -A --impact <file>               # what breaks if removed
loctree -A --circular                    # detect import cycles
```

### Tauri FE<>BE analysis
```bash
loctree -A --preset-tauri src src-tauri/src
loctree trace <handler> ./src            # investigate unused handler
```

### Dead code & CI
```bash
loctree -A --dead                        # unused exports
loctree -A --fail-on-missing-handlers    # CI check
loctree -A --sarif > results.sarif       # GitHub/GitLab integration
```"#;

    let copy_prompt = move |_| {
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(agent_prompt);
            set_prompt_copied.set(true);
            set_timeout(
                move || set_prompt_copied.set(false),
                std::time::Duration::from_millis(2000),
            );
        }
    };

    view! {
        <section id="for-agents" class="for-agents">
            <div class="container">
                <div class="section-header">
                    <p class="section-eyebrow">"For AI Agents"</p>
                    <h2 class="section-title">"Stop grepping. Start analyzing."</h2>
                    <p class="section-description">
                        "loctree does the work that agents often try to do manually. "
                        "Copy this prompt to your agent's context."
                    </p>
                </div>

                <div class="agents-grid">
                    // Left: Common mistakes
                    <div class="mistakes-panel">
                        <h3 class="panel-title">"Common Mistakes"</h3>
                        <div class="mistake-list">
                            <MistakeRow
                                wrong="grep -r \"import\" src | wc -l"
                                right="loctree -A --circular"
                                label="Finding circular imports"
                            />
                            <MistakeRow
                                wrong="grep -r \"EVENT\" | grep const"
                                right="loctree -A --json | jq .pipeline"
                                label="Finding ghost events"
                            />
                            <MistakeRow
                                wrong="find . -name \"*.ts\" -exec grep ..."
                                right="loctree -A --dead"
                                label="Finding unused code"
                            />
                            <MistakeRow
                                wrong="Manual file-by-file reading"
                                right="loctree slice <file> --consumers"
                                label="Understanding dependencies"
                            />
                            <MistakeRow
                                wrong="grep invoke | grep -v \"#\""
                                right="loctree trace <handler>"
                                label="Investigating handlers"
                            />
                        </div>
                    </div>

                    // Right: Prompt to copy
                    <div class="prompt-panel">
                        <h3 class="panel-title">"Agent Prompt"</h3>
                        <div class="prompt-container">
                            <pre class="prompt-text">{agent_prompt}</pre>
                            <button class="prompt-copy-btn" on:click=copy_prompt>
                                {move || if prompt_copied.get() { "Copied!" } else { "Copy to clipboard" }}
                            </button>
                        </div>
                    </div>
                </div>

                // Quick examples
                <div class="quick-examples">
                    <h3 class="examples-title">"One-liners for common tasks"</h3>
                    <div class="examples-grid">
                        <ExampleCard
                            task="AI context for a file"
                            cmd="loctree slice src/App.tsx --consumers --json | claude"
                        />
                        <ExampleCard
                            task="Check before creating component"
                            cmd="loctree -A --check ChatPanel"
                        />
                        <ExampleCard
                            task="Tauri handler investigation"
                            cmd="loctree trace toggle_assistant ./src ./src-tauri"
                        />
                        <ExampleCard
                            task="CI pipeline check"
                            cmd="loctree -A --fail-on-missing-handlers --sarif"
                        />
                    </div>
                </div>
            </div>
        </section>
    }
}

#[component]
fn MistakeRow(
    wrong: &'static str,
    right: &'static str,
    label: &'static str,
) -> impl IntoView {
    view! {
        <div class="mistake-row">
            <div class="mistake-label">{label}</div>
            <div class="mistake-wrong">
                <span class="mistake-icon">"x"</span>
                <code>{wrong}</code>
            </div>
            <div class="mistake-right">
                <span class="mistake-icon">"ok"</span>
                <code>{right}</code>
            </div>
        </div>
    }
}

#[component]
fn ExampleCard(
    task: &'static str,
    cmd: &'static str,
) -> impl IntoView {
    let (copied, set_copied) = signal(false);
    let cmd_str = cmd;

    let copy_cmd = move |_| {
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(cmd_str);
            set_copied.set(true);
            set_timeout(
                move || set_copied.set(false),
                std::time::Duration::from_millis(1500),
            );
        }
    };

    view! {
        <div class="example-card">
            <div class="example-task">{task}</div>
            <div class="example-cmd-row">
                <code class="example-cmd">{cmd}</code>
                <button class="example-copy" on:click=copy_cmd>
                    {move || if copied.get() { "ok" } else { "cp" }}
                </button>
            </div>
        </div>
    }
}
