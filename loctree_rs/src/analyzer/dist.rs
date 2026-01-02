//! Bundle distribution analysis using source maps.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::snapshot::Snapshot;
use crate::types::FileAnalysis;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMapping {
    pub gen_line: usize,
    pub gen_col: usize,
    pub source_idx: Option<usize>,
    pub source_line: Option<usize>,
    pub source_col: Option<usize>,
    pub name_idx: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct SourceMap {
    version: u8,
    sources: Vec<String>,
    #[serde(default)]
    names: Vec<String>,
    mappings: String,
    #[serde(default)]
    #[serde(rename = "sourceRoot")]
    source_root: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DistResult {
    #[serde(rename = "sourceExports")]
    pub source_exports: usize,
    #[serde(rename = "bundledExports")]
    pub bundled_exports: usize,
    #[serde(rename = "deadExports")]
    pub dead_exports: Vec<DeadBundleExport>,
    pub reduction: String,
    #[serde(rename = "symbolLevel")]
    pub symbol_level: bool,
}

#[derive(Debug, Serialize)]
pub struct DeadBundleExport {
    pub file: String,
    pub line: usize,
    pub name: String,
    pub kind: String,
}

const VLQ_BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn decode_base64_char(c: char) -> Option<i32> {
    VLQ_BASE64_CHARS
        .iter()
        .position(|&ch| ch == c as u8)
        .map(|pos| pos as i32)
}

fn decode_vlq_value(chars: &mut impl Iterator<Item = char>) -> Option<i32> {
    let first_char = chars.next()?;
    let mut value = decode_base64_char(first_char)?;
    let negative = (value & 1) != 0;
    let mut result = (value >> 1) & 0xF;
    let mut shift = 4;
    let mut continuation = (value & 0x20) != 0;

    while continuation {
        let ch = chars.next()?;
        value = decode_base64_char(ch)?;
        result |= (value & 0x1F) << shift;
        shift += 5;
        continuation = (value & 0x20) != 0;
    }

    Some(if negative { -result } else { result })
}

fn parse_mappings(mappings: &str) -> Vec<SourceMapping> {
    let mut result = Vec::new();
    let mut gen_line = 0usize;
    let mut source_idx = 0i32;
    let mut source_line = 0i32;
    let mut source_col = 0i32;
    let mut name_idx = 0i32;

    for line in mappings.split(';') {
        let mut gen_col = 0i32;
        if !line.is_empty() {
            for segment in line.split(',') {
                if segment.is_empty() {
                    continue;
                }
                let mut chars = segment.chars();
                if let Some(delta) = decode_vlq_value(&mut chars) {
                    gen_col += delta;
                    let src_idx = decode_vlq_value(&mut chars).map(|d| {
                        source_idx += d;
                        source_idx as usize
                    });
                    let src_line = decode_vlq_value(&mut chars).map(|d| {
                        source_line += d;
                        source_line as usize
                    });
                    let src_col = decode_vlq_value(&mut chars).map(|d| {
                        source_col += d;
                        source_col as usize
                    });
                    let nm_idx = decode_vlq_value(&mut chars).map(|d| {
                        name_idx += d;
                        name_idx as usize
                    });
                    result.push(SourceMapping {
                        gen_line,
                        gen_col: gen_col as usize,
                        source_idx: src_idx,
                        source_line: src_line,
                        source_col: src_col,
                        name_idx: nm_idx,
                    });
                }
            }
        }
        gen_line += 1;
    }
    result
}

fn parse_source_map(path: &Path) -> Result<SourceMap, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read source map: {}", e))?;
    let map: SourceMap = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse source map JSON: {}", e))?;
    if map.version != 3 {
        return Err(format!("Unsupported source map version: {}", map.version));
    }
    Ok(map)
}

fn extract_bundled_sources(map: &SourceMap) -> HashSet<String> {
    let mut sources = HashSet::new();
    for source in &map.sources {
        let normalized = if let Some(ref root) = map.source_root {
            format!(
                "{}/{}",
                root.trim_end_matches('/'),
                source.trim_start_matches("./")
            )
        } else {
            source.trim_start_matches("./").to_string()
        };
        sources.insert(normalized.replace('\\', "/"));
    }
    sources
}

fn extract_bundled_symbols(mappings: &[SourceMapping], names: &[String]) -> HashSet<String> {
    let mut symbols = HashSet::new();
    for mapping in mappings {
        if let Some(name_idx) = mapping.name_idx
            && let Some(symbol_name) = names.get(name_idx)
        {
            symbols.insert(symbol_name.clone());
        }
    }
    symbols
}

fn is_file_in_bundle(file_path: &str, bundled_sources: &HashSet<String>) -> bool {
    bundled_sources.iter().any(|bundled| {
        if bundled == file_path {
            return true;
        }
        if bundled.ends_with(file_path) {
            return true;
        }
        let file_no_ext = file_path
            .trim_end_matches(".ts")
            .trim_end_matches(".tsx")
            .trim_end_matches(".jsx");
        let bundled_no_ext = bundled
            .trim_end_matches(".js")
            .trim_end_matches(".jsx")
            .trim_end_matches(".mjs");
        file_no_ext == bundled_no_ext || bundled_no_ext.ends_with(file_no_ext)
    })
}

fn find_dead_by_symbol(
    analyses: &[FileAnalysis],
    bundled_sources: &HashSet<String>,
    bundled_symbols: &HashSet<String>,
) -> Vec<DeadBundleExport> {
    let mut dead_exports = Vec::new();
    for analysis in analyses {
        let file_path = analysis.path.trim_start_matches("./").replace('\\', "/");
        if !is_file_in_bundle(&file_path, bundled_sources) {
            for export in &analysis.exports {
                dead_exports.push(DeadBundleExport {
                    file: analysis.path.clone(),
                    line: export.line.unwrap_or(0),
                    name: export.name.clone(),
                    kind: export.kind.clone(),
                });
            }
        } else {
            for export in &analysis.exports {
                if !bundled_symbols.contains(&export.name) {
                    dead_exports.push(DeadBundleExport {
                        file: analysis.path.clone(),
                        line: export.line.unwrap_or(0),
                        name: export.name.clone(),
                        kind: export.kind.clone(),
                    });
                }
            }
        }
    }
    dead_exports
}

fn find_dead_bundle_exports(
    analyses: &[FileAnalysis],
    bundled_sources: &HashSet<String>,
) -> Vec<DeadBundleExport> {
    let mut dead_exports = Vec::new();
    for analysis in analyses {
        let file_path = analysis.path.trim_start_matches("./").replace('\\', "/");
        if !is_file_in_bundle(&file_path, bundled_sources) {
            for export in &analysis.exports {
                dead_exports.push(DeadBundleExport {
                    file: analysis.path.clone(),
                    line: export.line.unwrap_or(0),
                    name: export.name.clone(),
                    kind: export.kind.clone(),
                });
            }
        }
    }
    dead_exports
}

pub fn load_or_scan_src(src_dir: &Path) -> Result<Snapshot, String> {
    if !src_dir.exists() {
        return Err(format!(
            "Source directory does not exist: {}",
            src_dir.display()
        ));
    }
    if !src_dir.is_dir() {
        return Err(format!(
            "Source path is not a directory: {}",
            src_dir.display()
        ));
    }
    match Snapshot::load(src_dir) {
        Ok(s) => Ok(s),
        Err(_) => {
            use crate::args::ParsedArgs;
            use std::path::PathBuf;
            let parsed = ParsedArgs {
                use_gitignore: true,
                ..Default::default()
            };
            // Use quiet mode to suppress command bridge summary (not relevant for dist analysis)
            crate::snapshot::run_init_with_options(&[PathBuf::from(src_dir)], &parsed, true)
                .map_err(|e| format!("Failed to scan source directory: {}", e))?;
            Snapshot::load(src_dir)
                .map_err(|e| format!("Failed to load snapshot after scan: {}", e))
        }
    }
}

pub fn calculate_stats(
    analyses: &[FileAnalysis],
    dead_exports: &[DeadBundleExport],
) -> (usize, usize, String) {
    let total_exports: usize = analyses.iter().map(|a| a.exports.len()).sum();
    let bundled = total_exports.saturating_sub(dead_exports.len());
    let reduction_pct = if total_exports > 0 {
        (dead_exports.len() as f64 / total_exports as f64 * 100.0).round() as usize
    } else {
        0
    };
    (total_exports, bundled, format!("{}%", reduction_pct))
}

pub fn analyze_distribution(source_map_path: &Path, src_dir: &Path) -> Result<DistResult, String> {
    let source_map = parse_source_map(source_map_path)?;
    let snapshot = load_or_scan_src(src_dir)?;
    let bundled_sources = extract_bundled_sources(&source_map);

    let (dead_exports, symbol_level) = if !source_map.names.is_empty() {
        let mappings = parse_mappings(&source_map.mappings);
        if !mappings.is_empty() {
            let bundled_symbols = extract_bundled_symbols(&mappings, &source_map.names);
            (
                find_dead_by_symbol(&snapshot.files, &bundled_sources, &bundled_symbols),
                true,
            )
        } else {
            (
                find_dead_bundle_exports(&snapshot.files, &bundled_sources),
                false,
            )
        }
    } else {
        (
            find_dead_bundle_exports(&snapshot.files, &bundled_sources),
            false,
        )
    };

    let (source_exports, bundled_exports, reduction) =
        calculate_stats(&snapshot.files, &dead_exports);
    Ok(DistResult {
        source_exports,
        bundled_exports,
        dead_exports,
        reduction,
        symbol_level,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ExportSymbol;
    use tempfile::TempDir;

    #[test]
    fn test_vlq_decode_simple() {
        assert_eq!(decode_vlq_value(&mut "A".chars()), Some(0));
        assert_eq!(decode_vlq_value(&mut "C".chars()), Some(1));
        assert_eq!(decode_vlq_value(&mut "D".chars()), Some(-1));
    }

    #[test]
    fn test_parse_mappings_simple() {
        let mappings = parse_mappings("AAAA");
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].gen_col, 0);
        assert_eq!(mappings[0].source_idx, Some(0));
    }

    #[test]
    fn test_parse_mappings_with_name() {
        let mappings = parse_mappings("AAAAA");
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].name_idx, Some(0));
    }

    #[test]
    fn test_parse_mappings_multiple_lines() {
        let mappings = parse_mappings("AAAA;AAAA");
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].gen_line, 0);
        assert_eq!(mappings[1].gen_line, 1);
    }

    #[test]
    fn test_parse_source_map_basic() {
        let temp_dir = TempDir::new().unwrap();
        let map_path = temp_dir.path().join("test.js.map");
        fs::write(
            &map_path,
            r#"{"version":3,"sources":["src/index.ts"],"names":["foo"],"mappings":"AAAA"}"#,
        )
        .unwrap();
        let map = parse_source_map(&map_path).unwrap();
        assert_eq!(map.version, 3);
        assert_eq!(map.sources.len(), 1);
    }

    #[test]
    fn test_extract_bundled_sources() {
        let map = SourceMap {
            version: 3,
            sources: vec!["./src/index.ts".to_string()],
            names: vec![],
            mappings: "".to_string(),
            source_root: None,
        };
        let sources = extract_bundled_sources(&map);
        assert!(sources.contains("src/index.ts"));
    }

    #[test]
    fn test_extract_bundled_symbols() {
        let mappings = vec![SourceMapping {
            gen_line: 0,
            gen_col: 0,
            source_idx: Some(0),
            source_line: Some(1),
            source_col: Some(0),
            name_idx: Some(0),
        }];
        let names = vec!["foo".to_string(), "bar".to_string()];
        let symbols = extract_bundled_symbols(&mappings, &names);
        assert!(symbols.contains("foo"));
        assert!(!symbols.contains("bar"));
    }

    #[test]
    fn test_calculate_stats() {
        let analyses = vec![FileAnalysis {
            path: "src/a.ts".to_string(),
            exports: vec![
                ExportSymbol {
                    name: "foo".to_string(),
                    kind: "function".to_string(),
                    export_type: "named".to_string(),
                    line: Some(10),
                },
                ExportSymbol {
                    name: "bar".to_string(),
                    kind: "const".to_string(),
                    export_type: "named".to_string(),
                    line: Some(20),
                },
            ],
            ..Default::default()
        }];
        let dead = vec![DeadBundleExport {
            file: "src/a.ts".to_string(),
            line: 20,
            name: "bar".to_string(),
            kind: "const".to_string(),
        }];
        let (total, bundled, reduction) = calculate_stats(&analyses, &dead);
        assert_eq!(total, 2);
        assert_eq!(bundled, 1);
        assert_eq!(reduction, "50%");
    }

    #[test]
    fn test_find_dead_by_symbol_partial() {
        let analyses = vec![FileAnalysis {
            path: "src/partial.ts".to_string(),
            exports: vec![
                ExportSymbol {
                    name: "usedFunc".to_string(),
                    kind: "function".to_string(),
                    export_type: "named".to_string(),
                    line: Some(10),
                },
                ExportSymbol {
                    name: "deadFunc".to_string(),
                    kind: "function".to_string(),
                    export_type: "named".to_string(),
                    line: Some(20),
                },
            ],
            ..Default::default()
        }];
        let mut bundled_sources = HashSet::new();
        bundled_sources.insert("src/partial.ts".to_string());
        let mut bundled_symbols = HashSet::new();
        bundled_symbols.insert("usedFunc".to_string());
        let dead = find_dead_by_symbol(&analyses, &bundled_sources, &bundled_symbols);
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].name, "deadFunc");
    }
}
