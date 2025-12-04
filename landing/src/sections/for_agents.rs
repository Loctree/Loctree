use leptos::prelude::*;

#[component]
pub fn ForAgents() -> impl IntoView {
    let (prompt_copied, set_prompt_copied) = signal(false);

    let agent_prompt = r#"## loct Quick Reference

**Install:** `cargo install loctree`

### Before creating components
```bash
loct find --similar <Name>     # find similar existing
loct find --symbol <Name>      # find definitions & usage
loct query where-symbol <Name> # quick symbol lookup
```

### Before refactoring
```bash
loct slice <file> --consumers --json  # what depends on it
loct query who-imports <file>         # quick: who imports this
loct find --impact <file>             # what breaks if removed
loct cycles                           # detect import cycles
```

### Tauri FE<>BE analysis
```bash
loct commands                  # show all command bridges
loct commands --unused         # investigate unused handlers
```

### Dead code & CI
```bash
loct dead                      # unused exports
loct lint --fail               # CI check
loct lint --sarif > results.sarif  # GitHub/GitLab integration
loct diff --since main         # what changed since main
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
                                right="loct cycles"
                                label="Finding circular imports"
                            />
                            <MistakeRow
                                wrong="grep -r \"EVENT\" | grep const"
                                right="loct events --json"
                                label="Finding ghost events"
                            />
                            <MistakeRow
                                wrong="find . -name \"*.ts\" -exec grep ..."
                                right="loct dead"
                                label="Finding unused code"
                            />
                            <MistakeRow
                                wrong="Manual file-by-file reading"
                                right="loct slice <file> --consumers"
                                label="Understanding dependencies"
                            />
                            <MistakeRow
                                wrong="grep invoke | grep -v \"#\""
                                right="loct commands --unused"
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
                            cmd="loct slice src/App.tsx --consumers --json | claude"
                        />
                        <ExampleCard
                            task="Check before creating component"
                            cmd="loct find --similar ChatPanel"
                        />
                        <ExampleCard
                            task="Tauri handler investigation"
                            cmd="loct commands --unused"
                        />
                        <ExampleCard
                            task="CI pipeline check"
                            cmd="loct lint --fail --sarif"
                        />
                    </div>
                </div>
            </div>
        </section>
    }
}

#[component]
fn MistakeRow(wrong: &'static str, right: &'static str, label: &'static str) -> impl IntoView {
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
fn ExampleCard(task: &'static str, cmd: &'static str) -> impl IntoView {
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
