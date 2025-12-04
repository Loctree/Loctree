//! Quick Commands component - CLI reference for new v0.5.9+ features
//!
//! Shows helpful CLI commands based on the current report context.

use crate::components::icons::{Icon, ICON_TERMINAL};
use leptos::prelude::*;

/// Quick Commands Panel - shows helpful CLI commands
#[component]
pub fn QuickCommandsPanel(
    /// Root path being analyzed (for contextual examples)
    #[prop(into)]
    root: String,
    /// Whether there are duplicate exports
    has_duplicates: bool,
    /// Whether there are command gaps (Tauri)
    has_command_issues: bool,
) -> impl IntoView {
    // Get a sample file for examples (just use root path with a placeholder)
    let sample_file = if root.contains("src") {
        format!("{}/App.tsx", root)
    } else {
        format!("{}/src/main.ts", root)
    };

    view! {
        <div class="quick-commands-panel">
            <h3>
                <Icon path=ICON_TERMINAL />
                "Quick Commands"
                <span class="badge-new">"v0.6"</span>
            </h3>

            <div class="commands-grid">
                // Query commands section
                <div class="command-group">
                    <h4>"Query API"</h4>
                    <p class="command-desc">"Fast graph queries without full analysis"</p>
                    <div class="command-list">
                        <CommandItem
                            cmd=format!("loct query who-imports {}", sample_file)
                            desc="Files that import target"
                        />
                        <CommandItem
                            cmd="loct query where-symbol useAuth".to_string()
                            desc="Find symbol definitions"
                        />
                        <CommandItem
                            cmd=format!("loct query component-of {}", sample_file)
                            desc="Graph component containing file"
                        />
                    </div>
                </div>

                // Diff commands section
                <div class="command-group">
                    <h4>"Snapshot Diff"</h4>
                    <p class="command-desc">"Compare snapshots to track changes"</p>
                    <div class="command-list">
                        <CommandItem
                            cmd="loct diff --since main".to_string()
                            desc="Compare against main branch"
                        />
                        <CommandItem
                            cmd="loct diff --since HEAD~5".to_string()
                            desc="Delta since 5 commits ago"
                        />
                    </div>
                </div>

                // Context-aware suggestions
                {has_duplicates.then(|| view! {
                    <div class="command-group highlight">
                        <h4>"Suggested"</h4>
                        <p class="command-desc">"Based on current analysis"</p>
                        <div class="command-list">
                            <CommandItem
                                cmd="loct find --similar Button".to_string()
                                desc="Find before creating duplicates"
                            />
                            <CommandItem
                                cmd="loct dead --confidence high".to_string()
                                desc="Unused exports (safe to remove)"
                            />
                        </div>
                    </div>
                })}

                {has_command_issues.then(|| view! {
                    <div class="command-group highlight">
                        <h4>"Tauri"</h4>
                        <p class="command-desc">"FE↔BE coverage commands"</p>
                        <div class="command-list">
                            <CommandItem
                                cmd="loct commands --missing".to_string()
                                desc="FE calls without handlers"
                            />
                            <CommandItem
                                cmd="loct commands --unused".to_string()
                                desc="Handlers without FE calls"
                            />
                        </div>
                    </div>
                })}
            </div>

            <div class="commands-footer">
                <p>
                    <code>"loctree://open?f=<file>&l=<line>"</code>
                    " - IDE integration URLs in SARIF output"
                </p>
            </div>
        </div>
    }
}

/// Single command item with copy button
#[component]
fn CommandItem(#[prop(into)] cmd: String, #[prop(into)] desc: String) -> impl IntoView {
    let cmd_for_copy = cmd.clone();

    view! {
        <div class="command-item">
            <code class="command-code">{cmd}</code>
            <span class="command-desc-inline">{desc}</span>
            <button
                class="copy-btn"
                data-copy=cmd_for_copy
                title="Copy to clipboard"
            >
                "📋"
            </button>
        </div>
    }
}
