use serde_json::json;

use crate::analyzer::dead_parrots::DeadExport;
use crate::analyzer::report::CommandGap;
use crate::analyzer::RankedDup;

pub struct SarifInputs<'a> {
    pub duplicate_exports: &'a [RankedDup],
    pub missing_handlers: &'a [CommandGap],
    pub unused_handlers: &'a [CommandGap],
    pub dead_exports: &'a [DeadExport],
    pub pipeline_summary: &'a serde_json::Value,
}

pub fn print_sarif(inputs: SarifInputs) {
    let mut results = Vec::new();

    // Duplicate exports
    for dup in inputs.duplicate_exports {
        for file in &dup.files {
            results.push(json!({
                "ruleId": "duplicate-export",
                "level": "warning",
                "message": {
                    "text": format!("Duplicate export '{}' (canonical: {})", dup.name, dup.canonical)
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": file }
                    }
                }]
            }));
        }
    }

    // Missing handlers
    for gap in inputs.missing_handlers {
        for (file, line) in &gap.locations {
            results.push(json!({
                "ruleId": "missing-handler",
                "level": "error",
                "message": {
                    "text": format!("Missing backend handler for command '{}'", gap.name)
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": file },
                        "region": { "startLine": line }
                    }
                }]
            }));
        }
    }

    // Unused handlers
    for gap in inputs.unused_handlers {
        for (file, line) in &gap.locations {
            results.push(json!({
                "ruleId": "unused-handler",
                "level": "warning",
                "message": {
                    "text": format!("Unused backend handler '{}'", gap.name)
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": file },
                        "region": { "startLine": line }
                    }
                }]
            }));
        }
    }

    // Dead exports
    for dead in inputs.dead_exports {
        results.push(json!({
            "ruleId": "dead-export",
            "level": "warning",
            "message": {
                "text": format!("Potential dead export '{}' ({})", dead.symbol, dead.confidence)
            },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": dead.file },
                    "region": { "startLine": dead.line.unwrap_or(1) }
                }
            }]
        }));
    }

    // Ghost events
    if let Some(events) = inputs.pipeline_summary.get("events") {
        if let Some(ghosts) = events.get("ghostEmits").and_then(|v| v.as_array()) {
            for ghost in ghosts {
                let name = ghost["name"].as_str().unwrap_or("?");
                let path = ghost["path"].as_str().unwrap_or("?");
                let line = ghost["line"].as_u64().unwrap_or(1);
                let conf = ghost["confidence"].as_str().unwrap_or("low");

                results.push(json!({
                    "ruleId": "ghost-event",
                    "level": "warning",
                    "message": {
                        "text": format!("Ghost event '{}' (emitted but not listened, confidence: {})", name, conf)
                    },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": { "uri": path },
                            "region": { "startLine": line }
                        }
                    }]
                }));
            }
        }

        if let Some(orphans) = events.get("orphanListeners").and_then(|v| v.as_array()) {
            for orphan in orphans {
                let name = orphan["name"].as_str().unwrap_or("?");
                let path = orphan["path"].as_str().unwrap_or("?");
                let line = orphan["line"].as_u64().unwrap_or(1);

                results.push(json!({
                    "ruleId": "orphan-listener",
                    "level": "warning",
                    "message": {
                        "text": format!("Orphan listener for '{}' (no emitter found)", name)
                    },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": { "uri": path },
                            "region": { "startLine": line }
                        }
                    }]
                }));
            }
        }
    }

    let tool = json!({
        "driver": {
            "name": "loctree",
            "informationUri": "https://github.com/LibraxisAI/loctree",
            "version": env!("CARGO_PKG_VERSION"),
            "rules": [
                { "id": "duplicate-export", "shortDescription": { "text": "Duplicate export detected" } },
                { "id": "missing-handler", "shortDescription": { "text": "Missing backend handler for frontend command" } },
                { "id": "unused-handler", "shortDescription": { "text": "Unused backend handler" } },
                { "id": "dead-export", "shortDescription": { "text": "Export defined but never imported" } },
                { "id": "ghost-event", "shortDescription": { "text": "Event emitted but not listened to" } },
                { "id": "orphan-listener", "shortDescription": { "text": "Event listener without emitter" } }
            ]
        }
    });

    let sarif = json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{
            "tool": tool,
            "results": results
        }]
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&sarif).unwrap_or_default()
    );
}
