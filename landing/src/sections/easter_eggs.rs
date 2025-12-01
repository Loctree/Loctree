//! Easter eggs for developers who look under the hood
//!
//! Because we're all curious creatures.

#![allow(clippy::collapsible_if)]

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

/// ASCII art logo for console
const ASCII_LOGO: &str = r#"
    __            __
   / /___  _____/ /_________  ___
  / / __ \/ ___/ __/ ___/ _ \/ _ \
 / / /_/ / /__/ /_/ /  /  __/  __/
/_/\____/\___/\__/_/   \___/\___/

  Scan once, slice many.
  v0.5.6 | loctree.io
"#;

/// Initialize all easter eggs
#[component]
#[allow(clippy::unused_unit)]
pub fn EasterEggs() -> impl IntoView {
    // Print console art on mount
    Effect::new(move || {
        print_console_art();
        setup_konami_listener();
    });

    view! {}
}

/// Print ASCII art and messages to browser console
fn print_console_art() {
    if let Some(_window) = web_sys::window() {
        // ASCII logo with style
        web_sys::console::log_2(
            &JsValue::from_str(&format!("%c{}", ASCII_LOGO)),
            &JsValue::from_str("color: #00ff88; font-family: monospace; font-size: 11px;"),
        );

        web_sys::console::log_1(&JsValue::from_str(""));

        web_sys::console::log_2(
            &JsValue::from_str("%c(o_o) [tip] Run `loctree --for-ai` for AI-optimized output"),
            &JsValue::from_str("color: #ffcc00;"),
        );

        web_sys::console::log_2(
            &JsValue::from_str("%c(^_^) [api] curl loctree.io/api/agent/index.txt"),
            &JsValue::from_str("color: #00ccff;"),
        );

        web_sys::console::log_1(&JsValue::from_str(""));

        web_sys::console::log_2(
            &JsValue::from_str(
                "%cLoctree — Zombies? Not on my tree! Dead code dies here. — Static code analysis for agentic context. Built with 💀 by humans... and AI agents.",
            ),
            &JsValue::from_str("color: #ff6b6b; font-weight: bold;"),
        );

        web_sys::console::log_2(
            &JsValue::from_str("%c(T_T) [bug] github.com/Loctree/Loctree/issues"),
            &JsValue::from_str("color: #ff6b6b;"),
        );

        web_sys::console::log_2(
            &JsValue::from_str("%c(*_*) [star] Like it? Star us on GitHub"),
            &JsValue::from_str("color: #ffd700;"),
        );

        web_sys::console::log_1(&JsValue::from_str(""));

        web_sys::console::log_2(
            &JsValue::from_str("%c\\(^o^)/ Built with Rust + Leptos by M&K @ LibraxisAI"),
            &JsValue::from_str("color: #666; font-size: 10px;"),
        );

        // Secret hint
        web_sys::console::log_2(
            &JsValue::from_str("%c(._.) psst... try the konami code"),
            &JsValue::from_str("color: #333; font-size: 9px;"),
        );
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
        &JsValue::from_str("%cGang of Bastards @ LibraxisAI"),
        &JsValue::from_str("color: #888; font-size: 10px;"),
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
