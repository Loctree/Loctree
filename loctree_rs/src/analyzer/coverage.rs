use std::collections::{HashMap, HashSet};

use globset::GlobSet;

use super::report::CommandGap;

pub type CommandUsage = HashMap<String, Vec<(String, usize, String)>>;

fn normalize_cmd_name(name: &str) -> String {
    let mut out = String::new();
    let mut last_was_lower = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_uppercase() && last_was_lower && !out.is_empty() {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            last_was_lower = ch.is_ascii_lowercase();
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
            last_was_lower = false;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        name.to_lowercase()
    } else {
        out
    }
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
            if let Some(ex) = exclude {
                if ex.is_match(pb) {
                    return None;
                }
            }
            if let Some(focus_globs) = focus {
                if !focus_globs.is_match(pb) {
                    return None;
                }
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
