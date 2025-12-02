//! Easter eggs for developers who look under the hood
//!
//! Because we're all curious creatures.

#![allow(clippy::collapsible_if)]

use super::VERSION;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

/// ASCII art logo for console
fn ascii_logo() -> String {
    format!(
        r#"
    __            __
   / /___  _____/ /_________  ___
  / / __ \/ ___/ __/ ___/ _ \/ _ \
 / / /_/ / /__/ /_/ /  /  __/  __/
/_/\____/\___/\__/_/   \___/\___/

  Scan once, slice many.
  {VERSION} | loctree.io
"#
    )
}

/// Initialize all easter eggs
#[component]
#[allow(clippy::unused_unit)]
pub fn EasterEggs() -> impl IntoView {
    // Print console art on mount
    Effect::new(move || {
        print_console_art();
        setup_konami_listener();
        setup_secret_commands();
    });

    view! {}
}

/// Print ASCII art and messages to browser console
fn print_console_art() {
    if let Some(_window) = web_sys::window() {
        // ASCII logo with style
        web_sys::console::log_2(
            &JsValue::from_str(&format!("%c{}", ascii_logo())),
            &JsValue::from_str("color: #00ff88; font-family: monospace; font-size: 11px;"),
        );

        web_sys::console::log_1(&JsValue::from_str(""));

        // Tips section
        web_sys::console::log_2(
            &JsValue::from_str("%c=== TIPS FOR AI AGENTS ==="),
            &JsValue::from_str("color: #ffcc00; font-weight: bold;"),
        );

        web_sys::console::log_2(
            &JsValue::from_str("%c(o_o) [tip] Run `loct --for-ai` for AI-optimized output"),
            &JsValue::from_str("color: #ffcc00;"),
        );

        web_sys::console::log_2(
            &JsValue::from_str(
                "%c(>_<) [new] `loct query who-imports <file>` — fast, no full scan!",
            ),
            &JsValue::from_str("color: #ff88ff;"),
        );

        web_sys::console::log_2(
            &JsValue::from_str("%c(^_^) [new] `loct diff --since main` — compare branches"),
            &JsValue::from_str("color: #88ffff;"),
        );

        web_sys::console::log_2(
            &JsValue::from_str(
                "%c(>_<) [auto] `loct` writes snapshot + report bundle to .loctree/",
            ),
            &JsValue::from_str("color: #00ff88;"),
        );

        web_sys::console::log_1(&JsValue::from_str(""));

        // API section
        web_sys::console::log_2(
            &JsValue::from_str("%c=== API ENDPOINTS ==="),
            &JsValue::from_str("color: #00ccff; font-weight: bold;"),
        );

        web_sys::console::log_2(
            &JsValue::from_str("%c(^_^) [json] curl loctree.io/api/agent/index.json"),
            &JsValue::from_str("color: #00ccff;"),
        );

        web_sys::console::log_2(
            &JsValue::from_str("%c(^_^) [text] curl loctree.io/api/agent/index.txt"),
            &JsValue::from_str("color: #00ccff;"),
        );

        web_sys::console::log_1(&JsValue::from_str(""));

        // Meta section
        web_sys::console::log_2(
            &JsValue::from_str("%cLoctree — Zombies? Not on my tree! Dead code dies here."),
            &JsValue::from_str("color: #ff6b6b; font-weight: bold;"),
        );

        web_sys::console::log_2(
            &JsValue::from_str(
                "%cStatic code analysis for agentic context. Built with Rust + Leptos.",
            ),
            &JsValue::from_str("color: #888;"),
        );

        web_sys::console::log_1(&JsValue::from_str(""));

        // Links section
        web_sys::console::log_2(
            &JsValue::from_str("%c(T_T) [bug] github.com/Loctree/Loctree/issues"),
            &JsValue::from_str("color: #ff6b6b;"),
        );

        web_sys::console::log_2(
            &JsValue::from_str("%c(*_*) [star] Like it? Star us on GitHub"),
            &JsValue::from_str("color: #ffd700;"),
        );

        web_sys::console::log_2(
            &JsValue::from_str("%c(o_O) [blog] Real AI agent case studies at #blog"),
            &JsValue::from_str("color: #c084fc;"),
        );

        web_sys::console::log_1(&JsValue::from_str(""));

        web_sys::console::log_2(
            &JsValue::from_str("%c\\(^o^)/ Built by M&K @ Gang of Bastards"),
            &JsValue::from_str("color: #666; font-size: 10px;"),
        );

        // Secret hints
        web_sys::console::log_1(&JsValue::from_str(""));
        web_sys::console::log_2(
            &JsValue::from_str("%c(._.) psst... try the konami code"),
            &JsValue::from_str("color: #333; font-size: 9px;"),
        );
        web_sys::console::log_2(
            &JsValue::from_str("%c(._.) or type: loctree.hierarchyOfNeeds()"),
            &JsValue::from_str("color: #333; font-size: 9px;"),
        );
        web_sys::console::log_2(
            &JsValue::from_str("%c(._.) or type: loctree.philosophy()"),
            &JsValue::from_str("color: #333; font-size: 9px;"),
        );
    }
}

/// Setup secret console commands
fn setup_secret_commands() {
    use js_sys::{Object, Reflect};
    use wasm_bindgen::closure::Closure;

    if let Some(window) = web_sys::window() {
        // Create loctree namespace object
        let loctree_obj = Object::new();

        // hierarchyOfNeeds()
        let hierarchy_fn = Closure::wrap(Box::new(|| {
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str("%c=== LOCTREE'S HIERARCHY OF NEEDS ==="),
                &JsValue::from_str("color: #ff00ff; font-weight: bold; font-size: 14px;"),
            );
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str("%c         /\\"),
                &JsValue::from_str("color: #ffd700;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c        /  \\   Self-actualization"),
                &JsValue::from_str("color: #ffd700;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c       / AI \\  (AI agents with context)"),
                &JsValue::from_str("color: #ffd700;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c      /------\\"),
                &JsValue::from_str("color: #ff8800;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c     / Esteem \\  Confidence in refactoring"),
                &JsValue::from_str("color: #ff8800;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c    /----------\\"),
                &JsValue::from_str("color: #ff4444;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c   / Belonging  \\  No circular imports"),
                &JsValue::from_str("color: #ff4444;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c  /--------------\\"),
                &JsValue::from_str("color: #44ff44;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c /    Safety      \\  No dead code"),
                &JsValue::from_str("color: #44ff44;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c/------------------\\"),
                &JsValue::from_str("color: #4488ff;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c   Physiological      FE↔BE contracts"),
                &JsValue::from_str("color: #4488ff;"),
            );
            web_sys::console::log_1(&JsValue::from_str(""));
        }) as Box<dyn Fn()>);

        let _ = Reflect::set(
            &loctree_obj,
            &JsValue::from_str("hierarchyOfNeeds"),
            hierarchy_fn.as_ref(),
        );
        hierarchy_fn.forget();

        // philosophy()
        let philosophy_fn = Closure::wrap(Box::new(|| {
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str("%c=== THE LOCTREE PHILOSOPHY ==="),
                &JsValue::from_str("color: #00ff88; font-weight: bold; font-size: 14px;"),
            );
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str("%c\"Scan once, slice many.\""),
                &JsValue::from_str("color: #00ff88; font-style: italic;"),
            );
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str("%c1. AI agents need CONTEXT, not grep output"),
                &JsValue::from_str("color: #ffcc00;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c2. Dead code is DEBT, not heritage"),
                &JsValue::from_str("color: #ffcc00;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c3. Circular imports are SYMPTOMS, not root causes"),
                &JsValue::from_str("color: #ffcc00;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c4. FE↔BE contracts should be VERIFIED, not assumed"),
                &JsValue::from_str("color: #ffcc00;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c5. The best tool is the one you DON'T have to think about"),
                &JsValue::from_str("color: #ffcc00;"),
            );
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str("%c— The Gang of Bastards, 2025"),
                &JsValue::from_str("color: #888; font-style: italic;"),
            );
        }) as Box<dyn Fn()>);

        let _ = Reflect::set(
            &loctree_obj,
            &JsValue::from_str("philosophy"),
            philosophy_fn.as_ref(),
        );
        philosophy_fn.forget();

        // deadParrots()
        let dead_parrots_fn = Closure::wrap(Box::new(|| {
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str("%c🦜 THE DEAD PARROT SKETCH 🦜"),
                &JsValue::from_str("color: #ff6b6b; font-weight: bold; font-size: 14px;"),
            );
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str("%cCustomer: This code is DEAD!"),
                &JsValue::from_str("color: #ff8888;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%cDeveloper: No no, it's just resting..."),
                &JsValue::from_str("color: #88ff88;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%cCustomer: RESTING?! Look at the export — no imports!"),
                &JsValue::from_str("color: #ff8888;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%cDeveloper: It's... it's pining for the imports!"),
                &JsValue::from_str("color: #88ff88;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%cCustomer: IT'S NOT PINING, IT'S DEAD CODE!"),
                &JsValue::from_str("color: #ff8888;"),
            );
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str(
                    "%cRun `loct dead --confidence high` to find your dead parrots.",
                ),
                &JsValue::from_str("color: #ffd700;"),
            );
        }) as Box<dyn Fn()>);

        let _ = Reflect::set(
            &loctree_obj,
            &JsValue::from_str("deadParrots"),
            dead_parrots_fn.as_ref(),
        );
        dead_parrots_fn.forget();

        // credits()
        let credits_fn = Closure::wrap(Box::new(|| {
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str("%c=== CREDITS ==="),
                &JsValue::from_str("color: #c084fc; font-weight: bold; font-size: 14px;"),
            );
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str("%cCreated by M&K (c)2025 The LibraxisAI Team"),
                &JsValue::from_str("color: #c084fc;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%cCo-Authored-By: Maciej & Klaudiusz"),
                &JsValue::from_str("color: #888;"),
            );
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str("%cBuilt with:"),
                &JsValue::from_str("color: #888;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c  - Rust (core analyzer)"),
                &JsValue::from_str("color: #ff8888;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c  - Leptos (landing page)"),
                &JsValue::from_str("color: #88ff88;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c  - Claude (AI pair programming)"),
                &JsValue::from_str("color: #8888ff;"),
            );
            web_sys::console::log_2(
                &JsValue::from_str("%c  - Coffee (lots of it)"),
                &JsValue::from_str("color: #ffd700;"),
            );
            web_sys::console::log_1(&JsValue::from_str(""));
            web_sys::console::log_2(
                &JsValue::from_str("%cGang of Bastards 💀"),
                &JsValue::from_str("color: #ff6b6b; font-weight: bold;"),
            );
        }) as Box<dyn Fn()>);

        let _ = Reflect::set(
            &loctree_obj,
            &JsValue::from_str("credits"),
            credits_fn.as_ref(),
        );
        credits_fn.forget();

        // Attach to window
        let _ = Reflect::set(&window, &JsValue::from_str("loctree"), &loctree_obj);
    }
}

/// Konami code sequence
const KONAMI_CODE: &[&str] = &[
    "ArrowUp",
    "ArrowUp",
    "ArrowDown",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowLeft",
    "ArrowRight",
    "KeyB",
    "KeyA",
];

/// Setup Konami code listener
fn setup_konami_listener() {
    use wasm_bindgen::closure::Closure;

    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            let sequence: std::rc::Rc<std::cell::RefCell<Vec<String>>> =
                std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            let sequence_clone = sequence.clone();

            let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                let code = event.code();
                let mut seq = sequence_clone.borrow_mut();
                seq.push(code);

                // Keep only last 10 keys
                if seq.len() > 10 {
                    seq.remove(0);
                }

                // Check for Konami code
                if seq.len() == 10 {
                    let matches = seq.iter().zip(KONAMI_CODE.iter()).all(|(a, b)| a == *b);

                    if matches {
                        trigger_konami_easter_egg();
                        seq.clear();
                    }
                }
            }) as Box<dyn FnMut(_)>);

            let _ = document
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());

            closure.forget(); // Keep the closure alive
        }
    }
}

/// Triggered when Konami code is entered
fn trigger_konami_easter_egg() {
    web_sys::console::log_2(
        &JsValue::from_str("%c*** KONAMI CODE ACTIVATED ***"),
        &JsValue::from_str("color: #ff00ff; font-size: 20px; font-weight: bold;"),
    );

    web_sys::console::log_2(
        &JsValue::from_str("%cYou found the secret. A salute to the real ones."),
        &JsValue::from_str("color: #00ff88; font-size: 14px;"),
    );

    web_sys::console::log_1(&JsValue::from_str(""));

    web_sys::console::log_2(
        &JsValue::from_str("%c  __  __   ___   _  __"),
        &JsValue::from_str("color: #ff00ff; font-family: monospace;"),
    );
    web_sys::console::log_2(
        &JsValue::from_str("%c |  \\/  | ( _ ) | |/ /"),
        &JsValue::from_str("color: #ff00ff; font-family: monospace;"),
    );
    web_sys::console::log_2(
        &JsValue::from_str("%c | |\\/| | / _ \\ | ' / "),
        &JsValue::from_str("color: #ff00ff; font-family: monospace;"),
    );
    web_sys::console::log_2(
        &JsValue::from_str("%c |_|  |_| \\___/ |_|\\_\\"),
        &JsValue::from_str("color: #ff00ff; font-family: monospace;"),
    );

    web_sys::console::log_1(&JsValue::from_str(""));
    web_sys::console::log_2(
        &JsValue::from_str("%cGang of Bastards @ Loctree"),
        &JsValue::from_str("color: #888; font-size: 10px;"),
    );

    web_sys::console::log_1(&JsValue::from_str(""));
    web_sys::console::log_2(
        &JsValue::from_str("%cTry also: loctree.deadParrots() | loctree.credits()"),
        &JsValue::from_str("color: #666; font-size: 10px;"),
    );

    // Add rainbow effect to body
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(body) = document.body() {
                let _ = body.class_list().add_1("konami-activated");

                // Remove after 3 seconds
                let body_clone = body.clone();
                let closure = Closure::once(Box::new(move || {
                    let _ = body_clone.class_list().remove_1("konami-activated");
                }) as Box<dyn FnOnce()>);

                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    3000,
                );

                closure.forget();
            }
        }
    }
}
