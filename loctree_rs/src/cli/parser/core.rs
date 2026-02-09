//! Core parsing logic: syntax detection, global options extraction, and main entry point.
//!
//! This module contains the main `parse_command` function that serves as the entry point
//! for the new CLI parser, along with syntax detection and global option handling.

use std::path::PathBuf;

use super::super::command::{Command, GlobalOptions, HelpOptions, ParsedCommand};
use super::analysis_commands::{
    parse_cycles_command, parse_dead_command, parse_find_command, parse_impact_command,
    parse_query_command, parse_sniff_command, parse_twins_command,
};
use super::context_commands::{
    parse_coverage_command, parse_focus_command, parse_hotspots_command, parse_slice_command,
    parse_trace_command,
};
use super::helpers::{SUBCOMMANDS, is_jq_filter, parse_color_mode, suggest_similar_command};
use super::misc_commands::{
    parse_audit_command, parse_crowd_command, parse_dist_command, parse_doctor_command,
    parse_health_command, parse_help_command, parse_layoutmap_command, parse_plan_command,
    parse_suppress_command, parse_tagmap_command, parse_zombie_command,
};
use super::output_commands::{
    parse_diff_command, parse_info_command, parse_insights_command, parse_jq_query_command,
    parse_lint_command, parse_manifests_command, parse_memex_command, parse_pipelines_command,
    parse_report_command,
};
use super::scan_commands::{parse_auto_command, parse_scan_command, parse_tree_command};
use super::tauri_commands::{parse_commands_command, parse_events_command, parse_routes_command};

/// Check if the argument list appears to use new-style subcommands.
///
/// Returns true if the first non-flag argument is a known subcommand,
/// or if only global flags like --help/--version are present.
pub fn uses_new_syntax(args: &[String]) -> bool {
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        // Skip global flags that can appear before subcommand
        if arg == "--json"
            || arg == "--quiet"
            || arg == "--verbose"
            || arg == "--library-mode"
            || arg == "--python-library"
            || arg == "--fresh"
            || arg == "--no-scan"
            || arg == "--fail-stale"
            || arg == "--for-ai"
            || arg == "--findings"
            || arg == "--summary"
            || arg == "--watch"
            || arg == "-v"
            || arg == "-q"
        {
            i += 1;
            continue;
        }

        // Handle flags with optional/required values
        if arg.starts_with("--color") || arg.starts_with("--py-root") {
            // --color=auto or --py-root=Lib (value in same arg)
            if arg.contains('=') {
                i += 1;
            } else {
                // --color auto or --py-root Lib (value in next arg)
                i += 2;
            }
            continue;
        }

        // These are always valid in new syntax (not legacy-specific)
        if arg == "--help"
            || arg == "-h"
            || arg == "--help-legacy"
            || arg == "--help-full"
            || arg == "--version"
            || arg == "-V"
        {
            return true;
        }
        // If we hit a flag, it's likely legacy syntax
        if arg.starts_with('-') {
            return false;
        }
        // First positional argument - check if it's a subcommand or jq filter
        return SUBCOMMANDS.contains(&arg.as_str()) || is_jq_filter(arg);
    }
    // No arguments = default to auto (new syntax)
    true
}

/// Parse command-line arguments into a ParsedCommand.
///
/// This is the main entry point for the new CLI parser. It:
/// 1. Extracts global options (--json, --quiet, etc.)
/// 2. Identifies the subcommand
/// 3. Parses command-specific options
///
/// Returns `None` if the arguments should be handled by the legacy parser.
pub fn parse_command(args: &[String]) -> Result<Option<ParsedCommand>, String> {
    // Quick check: if this looks like legacy syntax, return None
    if !uses_new_syntax(args) {
        return Ok(None);
    }

    let mut global = GlobalOptions::default();
    let mut remaining_args: Vec<String> = Vec::new();
    let mut subcommand: Option<String> = None;
    let mut for_ai_alias = false;
    let mut watch_alias = false;
    let mut help_requested = false;

    // Check for jq-style query before extracting global options
    // This allows: loct '.metadata' to work without conflicts
    if !args.is_empty() && is_jq_filter(&args[0]) {
        return parse_jq_query_command(args, &global).map(Some);
    }

    // First pass: extract global options and find subcommand
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        match arg.as_str() {
            "--json" => {
                global.json = true;
                i += 1;
            }
            "--quiet" | "-q" => {
                global.quiet = true;
                i += 1;
            }
            "--verbose" | "-v" => {
                global.verbose = true;
                i += 1;
            }
            "--color" => {
                if let Some(value) = args.get(i + 1) {
                    global.color = parse_color_mode(value)?;
                    i += 2;
                } else {
                    global.color = crate::types::ColorMode::Always;
                    i += 1;
                }
            }
            _ if arg.starts_with("--color=") => {
                let value = arg.trim_start_matches("--color=");
                global.color = parse_color_mode(value)?;
                i += 1;
            }
            "--for-ai" => {
                for_ai_alias = true;
                i += 1;
            }
            "--watch" => {
                watch_alias = true;
                remaining_args.push(arg.clone());
                i += 1;
            }
            "--library-mode" => {
                global.library_mode = true;
                i += 1;
            }
            "--python-library" => {
                global.python_library = true;
                i += 1;
            }
            "--fresh" => {
                global.fresh = true;
                i += 1;
            }
            "--no-scan" => {
                global.no_scan = true;
                i += 1;
            }
            "--fail-stale" => {
                global.fail_stale = true;
                i += 1;
            }
            "--findings" => {
                global.findings = true;
                i += 1;
            }
            "--summary" => {
                global.summary_only_output = true;
                i += 1;
            }
            "--py-root" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--py-root requires a path".to_string())?;
                global.py_roots.push(PathBuf::from(value));
                i += 2;
            }
            _ if arg.starts_with("--py-root=") => {
                let value = arg.trim_start_matches("--py-root=");
                global.py_roots.push(PathBuf::from(value));
                i += 1;
            }
            "--help" | "-h" => {
                help_requested = true;
                i += 1;
            }
            "--help-legacy" => {
                return Ok(Some(ParsedCommand::new(
                    Command::Help(HelpOptions {
                        legacy: true,
                        ..Default::default()
                    }),
                    global,
                )));
            }
            "--help-full" => {
                return Ok(Some(ParsedCommand::new(
                    Command::Help(HelpOptions {
                        full: true,
                        ..Default::default()
                    }),
                    global,
                )));
            }
            "--version" | "-V" => {
                return Ok(Some(ParsedCommand::new(Command::Version, global)));
            }
            _ if arg.starts_with('-') => {
                // Unknown flag - pass to command-specific parser
                remaining_args.push(arg.clone());
                i += 1;
            }
            _ => {
                // Positional argument
                if subcommand.is_none() && SUBCOMMANDS.contains(&arg.as_str()) {
                    subcommand = Some(arg.clone());
                } else {
                    remaining_args.push(arg.clone());
                }
                i += 1;
            }
        }
    }

    if subcommand.is_none() && watch_alias {
        subcommand = Some("scan".to_string());
    }

    if help_requested {
        return Ok(Some(ParsedCommand::new(
            Command::Help(HelpOptions {
                command: subcommand.clone(),
                ..Default::default()
            }),
            global,
        )));
    }

    let mut command = match subcommand.as_deref() {
        None | Some("auto") => parse_auto_command(&remaining_args)?,
        Some("agent") => {
            let cmd = parse_auto_command(&remaining_args)?;
            match cmd {
                Command::Auto(mut opts) => {
                    opts.for_agent_feed = true;
                    opts.agent_json = true;
                    Command::Auto(opts)
                }
                other => other,
            }
        }
        Some("scan") => parse_scan_command(&remaining_args)?,
        Some("tree") => parse_tree_command(&remaining_args)?,
        Some("slice") | Some("s") => parse_slice_command(&remaining_args)?,
        Some("find") | Some("f") => parse_find_command(&remaining_args)?,
        Some("dead") | Some("unused") | Some("d") => parse_dead_command(&remaining_args)?,
        Some("cycles") | Some("c") => parse_cycles_command(&remaining_args)?,
        Some("trace") => parse_trace_command(&remaining_args)?,
        Some("commands") => parse_commands_command(&remaining_args)?,
        Some("events") => parse_events_command(&remaining_args)?,
        Some("pipelines") => parse_pipelines_command(&remaining_args)?,
        Some("insights") => parse_insights_command(&remaining_args)?,
        Some("manifests") => parse_manifests_command(&remaining_args)?,
        Some("routes") => parse_routes_command(&remaining_args)?,
        Some("info") => parse_info_command(&remaining_args)?,
        Some("lint") => parse_lint_command(&remaining_args)?,
        Some("report") => parse_report_command(&remaining_args)?,
        Some("help") => parse_help_command(&remaining_args)?,
        Some("query") | Some("q") => parse_query_command(&remaining_args)?,
        Some("impact") | Some("i") => parse_impact_command(&remaining_args)?,
        Some("diff") => parse_diff_command(&remaining_args)?,
        Some("memex") => parse_memex_command(&remaining_args)?,
        Some("crowd") => parse_crowd_command(&remaining_args)?,
        Some("tagmap") => parse_tagmap_command(&remaining_args)?,
        Some("twins") | Some("t") => parse_twins_command(&remaining_args)?,
        Some("suppress") => parse_suppress_command(&remaining_args)?,
        Some("sniff") => parse_sniff_command(&remaining_args)?,
        Some("dist") => parse_dist_command(&remaining_args)?,
        Some("coverage") => parse_coverage_command(&remaining_args)?,
        Some("focus") => parse_focus_command(&remaining_args)?,
        Some("hotspots") => parse_hotspots_command(&remaining_args)?,
        Some("layoutmap") => parse_layoutmap_command(&remaining_args)?,
        Some("zombie") => parse_zombie_command(&remaining_args)?,
        Some("health") | Some("h") => parse_health_command(&remaining_args)?,
        Some("audit") => parse_audit_command(&remaining_args)?,
        Some("doctor") => parse_doctor_command(&remaining_args)?,
        Some("plan") | Some("p") => parse_plan_command(&remaining_args)?,
        Some(unknown) => {
            // Try to find a similar command using fuzzy matching
            let suggestion = suggest_similar_command(unknown);
            return Err(format!(
                "Unknown command '{}'. {}Run 'loct --help' for available commands.",
                unknown,
                suggestion.map_or(String::new(), |s| format!("Did you mean: {}?\n", s))
            ));
        }
    };

    if for_ai_alias {
        match command {
            Command::Auto(ref mut opts) => {
                opts.for_agent_feed = true;
                opts.agent_json = true;
                opts.full_scan = true;
            }
            _ => {
                return Err(
                    "--for-ai is only supported with the default scan (no subcommand)".to_string(),
                );
            }
        }
    }

    Ok(Some(ParsedCommand::new(command, global)))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uses_new_syntax() {
        // New syntax
        assert!(uses_new_syntax(&[]));
        assert!(uses_new_syntax(&["scan".into()]));
        assert!(uses_new_syntax(&["tree".into()]));
        assert!(uses_new_syntax(&["--json".into(), "scan".into()]));
        assert!(uses_new_syntax(&["--watch".into()]));
        assert!(uses_new_syntax(&["--for-ai".into()]));

        // Legacy syntax
        assert!(!uses_new_syntax(&["--tree".into()]));
        assert!(!uses_new_syntax(&["-A".into()]));
        assert!(!uses_new_syntax(&["-A".into(), "--dead".into()]));
    }

    #[test]
    fn test_parse_auto_default() {
        let result = parse_command(&[]).unwrap().unwrap();
        assert_eq!(result.command.name(), "auto");
    }

    #[test]
    fn test_parse_for_ai_alias() {
        let args = vec!["--for-ai".into()];
        let result = parse_command(&args).unwrap().unwrap();
        if let Command::Auto(opts) = result.command {
            assert!(opts.for_agent_feed);
            assert!(opts.agent_json);
            assert!(opts.full_scan);
        } else {
            panic!("Expected Auto command");
        }
    }

    #[test]
    fn test_parse_watch_alias_defaults_to_scan() {
        let args = vec!["--watch".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "scan");
        if let Command::Scan(opts) = result.command {
            assert!(opts.watch);
        } else {
            panic!("Expected Scan command");
        }
    }

    #[test]
    fn test_parse_scan_command() {
        let args = vec!["scan".into(), "--full-scan".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "scan");
        if let Command::Scan(opts) = result.command {
            assert!(opts.full_scan);
        } else {
            panic!("Expected Scan command");
        }
    }

    #[test]
    fn test_parse_tree_command_with_depth() {
        let args = vec!["tree".into(), "--depth".into(), "3".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "tree");
        if let Command::Tree(opts) = result.command {
            assert_eq!(opts.depth, Some(3));
        } else {
            panic!("Expected Tree command");
        }
    }

    #[test]
    fn test_parse_slice_command() {
        let args = vec!["slice".into(), "src/main.rs".into(), "--consumers".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "slice");
        if let Command::Slice(opts) = result.command {
            assert_eq!(opts.target, "src/main.rs");
            assert!(opts.consumers);
        } else {
            panic!("Expected Slice command");
        }
    }

    #[test]
    fn test_parse_trace_command() {
        let args = vec!["trace".into(), "toggle_assistant".into(), "app".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "trace");
        if let Command::Trace(opts) = result.command {
            assert_eq!(opts.handler, "toggle_assistant");
            assert_eq!(opts.roots, vec![PathBuf::from("app")]);
        } else {
            panic!("Expected Trace command");
        }
    }

    #[test]
    fn test_parse_dead_command() {
        let args = vec!["dead".into(), "--confidence".into(), "high".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "dead");
        if let Command::Dead(opts) = result.command {
            assert_eq!(opts.confidence, Some("high".into()));
        } else {
            panic!("Expected Dead command");
        }
    }

    #[test]
    fn test_parse_global_json_flag() {
        let args = vec!["--json".into(), "scan".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert!(result.global.json);
        assert_eq!(result.command.name(), "scan");
    }

    #[test]
    fn test_parse_help_flag() {
        let args = vec!["--help".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "help");
    }

    #[test]
    fn test_parse_version_flag() {
        let args = vec!["--version".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "version");
    }

    #[test]
    fn test_legacy_syntax_returns_none() {
        let args = vec!["--tree".into()];
        let result = parse_command(&args).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_find_with_regex() {
        let args = vec![
            "find".into(),
            "--symbol".into(),
            ".*patient.*".into(),
            "--lang".into(),
            "ts".into(),
        ];
        let result = parse_command(&args).unwrap().unwrap();
        if let Command::Find(opts) = result.command {
            assert_eq!(opts.symbol, Some(".*patient.*".into()));
            assert_eq!(opts.lang, Some("ts".into()));
        } else {
            panic!("Expected Find command");
        }
    }

    #[test]
    fn test_parse_crowd_command() {
        let args = vec!["crowd".into(), "message".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "crowd");
        if let Command::Crowd(opts) = result.command {
            assert_eq!(opts.pattern, Some("message".into()));
        } else {
            panic!("Expected Crowd command");
        }
    }

    #[test]
    fn test_parse_crowd_auto_detect() {
        let args = vec!["crowd".into(), "--auto".into()];
        let result = parse_command(&args).unwrap().unwrap();
        if let Command::Crowd(opts) = result.command {
            assert!(opts.auto_detect);
            assert!(opts.pattern.is_none());
        } else {
            panic!("Expected Crowd command");
        }
    }

    #[test]
    fn test_parse_jq_query_basic() {
        let args = vec![".metadata".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "jq");
        if let Command::JqQuery(opts) = result.command {
            assert_eq!(opts.filter, ".metadata");
            assert!(!opts.raw_output);
            assert!(!opts.compact_output);
        } else {
            panic!("Expected JqQuery command");
        }
    }

    #[test]
    fn test_parse_jq_query_with_flags() {
        let args = vec![".files[]".into(), "-r".into(), "-c".into()];
        let result = parse_command(&args).unwrap().unwrap();
        if let Command::JqQuery(opts) = result.command {
            assert_eq!(opts.filter, ".files[]");
            assert!(opts.raw_output);
            assert!(opts.compact_output);
        } else {
            panic!("Expected JqQuery command");
        }
    }

    #[test]
    fn test_parse_jq_query_with_arg() {
        let args = vec![
            ".metadata".into(),
            "--arg".into(),
            "name".into(),
            "value".into(),
        ];
        let result = parse_command(&args).unwrap().unwrap();
        if let Command::JqQuery(opts) = result.command {
            assert_eq!(opts.string_args.len(), 1);
            assert_eq!(opts.string_args[0].0, "name");
            assert_eq!(opts.string_args[0].1, "value");
        } else {
            panic!("Expected JqQuery command");
        }
    }

    #[test]
    fn test_parse_jq_query_with_snapshot() {
        let args = vec![
            ".metadata".into(),
            "--snapshot".into(),
            ".loctree/snap.json".into(),
        ];
        let result = parse_command(&args).unwrap().unwrap();
        if let Command::JqQuery(opts) = result.command {
            assert_eq!(
                opts.snapshot_path,
                Some(PathBuf::from(".loctree/snap.json"))
            );
        } else {
            panic!("Expected JqQuery command");
        }
    }
}
