use std::collections::{HashMap, HashSet};

use globset::GlobSet;
use heck::ToSnakeCase;

use super::report::CommandGap;

pub type CommandUsage = HashMap<String, Vec<(String, usize, String)>>;

fn normalize_cmd_name(name: &str) -> String {
    let mut buffered = String::new();
    for ch in name.chars() {
        if ch.is_alphanumeric() {
            buffered.push(ch);
        } else {
            buffered.push('_');
        }
    }
    buffered
        .to_snake_case()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn strip_excluded_paths(
    paths: &[(String, usize, String)],
    focus: &Option<GlobSet>,
    exclude: &Option<GlobSet>,
) -> Vec<(String, usize)> {
    paths
        .iter()
        .filter_map(|(p, line, _)| {
            let pb = std::path::Path::new(p);
            if let Some(ex) = exclude
                && ex.is_match(pb)
            {
                return None;
            }
            if let Some(focus_globs) = focus
                && !focus_globs.is_match(pb)
            {
                return None;
            }
            Some((p.clone(), *line))
        })
        .collect()
}

pub fn compute_command_gaps(
    fe_commands: &CommandUsage,
    be_commands: &CommandUsage,
    focus_set: &Option<GlobSet>,
    exclude_set: &Option<GlobSet>,
) -> (Vec<CommandGap>, Vec<CommandGap>) {
    let fe_norms: HashMap<String, String> = fe_commands
        .keys()
        .map(|k| (k.clone(), normalize_cmd_name(k)))
        .collect();
    let be_norms: HashMap<String, String> = be_commands
        .keys()
        .map(|k| (k.clone(), normalize_cmd_name(k)))
        .collect();
    let be_norm_set: HashSet<String> = be_norms.values().cloned().collect();
    let fe_norm_set: HashSet<String> = fe_norms.values().cloned().collect();

    let missing_handlers: Vec<CommandGap> = fe_commands
        .iter()
        .filter_map(|(name, locs)| {
            let norm = fe_norms
                .get(name)
                .cloned()
                .unwrap_or_else(|| normalize_cmd_name(name));
            if be_norm_set.contains(&norm) {
                return None;
            }
            let kept = strip_excluded_paths(locs, focus_set, exclude_set);
            if kept.is_empty() {
                None
            } else {
                let impl_name = locs
                    .iter()
                    .find(|(p, l, _)| p == &kept[0].0 && *l == kept[0].1)
                    .map(|(_, _, n)| n.clone());
                Some(CommandGap {
                    name: name.clone(),
                    implementation_name: impl_name,
                    locations: kept,
                })
            }
        })
        .collect();

    let unused_handlers: Vec<CommandGap> = be_commands
        .iter()
        .filter_map(|(name, locs)| {
            let norm = be_norms
                .get(name)
                .cloned()
                .unwrap_or_else(|| normalize_cmd_name(name));
            if fe_norm_set.contains(&norm) {
                return None;
            }
            let kept = strip_excluded_paths(locs, focus_set, exclude_set);
            if kept.is_empty() {
                None
            } else {
                let impl_name = locs
                    .iter()
                    .find(|(p, l, _)| p == &kept[0].0 && *l == kept[0].1)
                    .map(|(_, _, n)| n.clone());
                Some(CommandGap {
                    name: name.clone(),
                    implementation_name: impl_name,
                    locations: kept,
                })
            }
        })
        .collect();

    (missing_handlers, unused_handlers)
}

/// Compute gaps for backend handlers that are defined but never registered with Tauri.
///
/// `be_commands` is the full backend command usage map (including both registered and
/// unregistered handlers). `registered_impls` is the set of Rust function names that
/// are actually registered via `tauri::generate_handler![...]` across the project.
///
/// We treat a command name as "unregistered" if **none** of its implementation symbols
/// appear in `registered_impls`. Paths are filtered through `focus_set` / `exclude_set`
/// in the same way as in `compute_command_gaps`.
pub fn compute_unregistered_handlers(
    be_commands: &CommandUsage,
    registered_impls: &std::collections::HashSet<String>,
    focus_set: &Option<GlobSet>,
    exclude_set: &Option<GlobSet>,
) -> Vec<CommandGap> {
    be_commands
        .iter()
        .filter_map(|(name, locs)| {
            // If any impl symbol for this command is registered, skip it.
            let has_registered_impl = locs
                .iter()
                .any(|(_, _, impl_name)| registered_impls.contains(impl_name));
            if has_registered_impl {
                return None;
            }

            let kept = strip_excluded_paths(locs, focus_set, exclude_set);
            if kept.is_empty() {
                return None;
            }

            let impl_name = locs
                .iter()
                .find(|(p, l, _)| p == &kept[0].0 && *l == kept[0].1)
                .map(|(_, _, n)| n.clone());

            Some(CommandGap {
                name: name.clone(),
                implementation_name: impl_name,
                locations: kept,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use globset::{Glob, GlobSetBuilder};

    #[test]
    fn matches_commands_across_casing() {
        let mut fe: CommandUsage = HashMap::new();
        fe.insert(
            "fetchUserData".into(),
            vec![("src/fe.ts".into(), 10usize, "fetchUserData".into())],
        );
        let mut be: CommandUsage = HashMap::new();
        be.insert(
            "fetch_user_data".into(),
            vec![("src/be.rs".into(), 20usize, "fetch_user_data".into())],
        );
        let (missing, unused) = compute_command_gaps(&fe, &be, &None, &None);
        assert!(missing.is_empty(), "should detect matching handler");
        assert!(unused.is_empty(), "should detect frontend usage");
    }

    #[test]
    fn ignores_excluded_paths_before_gap_report() {
        let mut builder = GlobSetBuilder::new();
        builder.add(Glob::new("**/ignored/**").expect("valid glob"));
        let exclude_set = Some(builder.build().expect("build globset"));
        let mut fe: CommandUsage = HashMap::new();
        fe.insert(
            "audio-play".into(),
            vec![("ignored/fe.ts".into(), 5usize, "audio-play".into())],
        );
        let mut be: CommandUsage = HashMap::new();
        be.insert(
            "audio_play".into(),
            vec![("src/handler.rs".into(), 8usize, "audio_play".into())],
        );
        let (missing, unused) = compute_command_gaps(&fe, &be, &None, &exclude_set);
        assert!(missing.is_empty());
        assert!(unused.is_empty());
    }
}
