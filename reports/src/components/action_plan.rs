//! Action plan panel - first-shot tasks with fix + verify.

use leptos::prelude::*;

use crate::components::icons::{ICON_CLIPBOARD_LIST, Icon};
use crate::types::PriorityTask;

/// Action plan panel showing top priority tasks.
#[component]
pub fn ActionPlanPanel(tasks: Vec<PriorityTask>) -> impl IntoView {
    if tasks.is_empty() {
        return view! { "" }.into_any();
    }

    view! {
        <div class="panel action-plan-panel">
            <h3>
                <Icon path=ICON_CLIPBOARD_LIST class="icon-sm" />
                "Action Plan"
            </h3>
            <p class="muted">"First-shot tasks with why, fix, and verify commands."</p>
            <ol class="action-list">
                {tasks.into_iter().map(|task| {
                    let risk_class = match task.risk.as_str() {
                        "high" => "risk-high",
                        "medium" => "risk-medium",
                        _ => "risk-low",
                    };
                    let verify_cmd = task.verify_cmd.clone();
                    view! {
                        <li class="action-item">
                            <div class="action-head">
                                <span class="action-priority">{format!("#{}", task.priority)}</span>
                                <code class="action-target">{task.target.clone()}</code>
                                <span class="action-kind">{task.kind.clone()}</span>
                                <span class=format!("action-risk {}", risk_class)>{task.risk}</span>
                            </div>
                            <div class="action-why">{task.why.clone()}</div>
                            <div class="action-fix">
                                <span class="action-label">"Fix"</span>
                                <span>{task.fix_hint.clone()}</span>
                            </div>
                            <div class="action-verify">
                                <span class="action-label">"Verify"</span>
                                <code>{verify_cmd.clone()}</code>
                                <button class="copy-btn" data-copy=verify_cmd>"Copy"</button>
                            </div>
                            <div class="action-location">
                                <span class="action-label">"Location"</span>
                                <span>{task.location}</span>
                            </div>
                        </li>
                    }
                }).collect::<Vec<_>>()}
            </ol>
        </div>
    }
    .into_any()
}

// Tests live in reports/src/lib.rs via render_report.
