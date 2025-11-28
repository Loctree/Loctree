use serde_json::json;

use crate::types::FileAnalysis;

pub fn find_entrypoints(analyses: &[FileAnalysis]) -> Vec<(String, Vec<String>)> {
    let mut entrypoints = Vec::new();
    for analysis in analyses {
        if !analysis.entry_points.is_empty() {
            entrypoints.push((analysis.path.clone(), analysis.entry_points.clone()));
        }
    }
    entrypoints.sort_by(|a, b| a.0.cmp(&b.0));
    entrypoints
}

pub fn print_entrypoints(entrypoints: &[(String, Vec<String>)], json_output: bool) {
    if json_output {
        let items: Vec<_> = entrypoints
            .iter()
            .map(|(path, types)| {
                json!({
                    "path": path,
                    "types": types
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "entryPoints": items }))
                .expect("Failed to serialize entry points to JSON")
        );
    } else if entrypoints.is_empty() {
        println!("No entry points detected.");
    } else {
        println!("Entry points ({} found):", entrypoints.len());
        for (path, types) in entrypoints {
            let unique: std::collections::HashSet<_> = types.iter().collect();
            let mut sorted: Vec<_> = unique.into_iter().collect();
            sorted.sort();
            println!(
                "  - {}: {}",
                path,
                sorted
                    .into_iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
}
