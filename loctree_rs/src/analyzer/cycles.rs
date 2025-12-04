//! Circular import detection using Tarjan's SCC algorithm.
//!
//! Finds strongly connected components (cycles) in the import graph.
//! Normalizes module paths to collapse barrels and extensions before detection.

use std::cmp::min;
use std::collections::{HashMap, HashSet};

use serde_json::json;

use super::root_scan::normalize_module_id;

struct TarjanData {
    index: usize,
    indices: HashMap<String, usize>,
    lowlinks: HashMap<String, usize>,
    stack: Vec<String>,
    on_stack: HashSet<String>,
    sccs: Vec<Vec<String>>,
}

pub fn find_cycles(edges: &[(String, String, String)]) -> Vec<Vec<String>> {
    // Normalize modules to collapse barrels (index files) and JS/TS extensions.
    // Keep a canonical display path for each normalized module id so output remains readable.
    let mut canonical: HashMap<String, String> = HashMap::new();
    let mut normalized_edges: Vec<(String, String)> = Vec::new();

    for (from, to, _) in edges {
        let norm_from = normalize_module_id(from).as_key();
        let norm_to = normalize_module_id(to).as_key();
        canonical
            .entry(norm_from.clone())
            .or_insert_with(|| from.clone());
        canonical
            .entry(norm_to.clone())
            .or_insert_with(|| to.clone());
        normalized_edges.push((norm_from, norm_to));
    }

    // Run Tarjan on normalized module ids
    let cycles_norm = find_cycles_normalized(&normalized_edges);

    // Map back to canonical display paths
    cycles_norm
        .into_iter()
        .map(|cycle| {
            cycle
                .into_iter()
                .map(|id| canonical.get(&id).cloned().unwrap_or(id))
                .collect()
        })
        .collect()
}

fn find_cycles_normalized(edges: &[(String, String)]) -> Vec<Vec<String>> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_nodes = HashSet::new();

    for (from, to) in edges {
        adj.entry(from.clone()).or_default().push(to.clone());
        all_nodes.insert(from.clone());
        all_nodes.insert(to.clone());
    }

    let mut data = TarjanData {
        index: 0,
        indices: HashMap::new(),
        lowlinks: HashMap::new(),
        stack: Vec::new(),
        on_stack: HashSet::new(),
        sccs: Vec::new(),
    };

    let mut nodes: Vec<_> = all_nodes.into_iter().collect();
    nodes.sort();

    for node in nodes {
        if !data.indices.contains_key(&node) {
            strongconnect(&node, &adj, &mut data);
        }
    }

    // Filter SCCs that form cycles.
    // An SCC is a cycle if it has > 1 node, OR it has 1 node with a self-loop.
    data.sccs
        .into_iter()
        .filter(|scc| {
            if scc.len() > 1 {
                return true;
            }
            // Check self loop
            if let Some(node) = scc.first()
                && let Some(neighbors) = adj.get(node)
            {
                return neighbors.contains(node);
            }
            false
        })
        .collect()
}

fn strongconnect(node: &str, adj: &HashMap<String, Vec<String>>, data: &mut TarjanData) {
    data.indices.insert(node.to_string(), data.index);
    data.lowlinks.insert(node.to_string(), data.index);
    data.index += 1;
    data.stack.push(node.to_string());
    data.on_stack.insert(node.to_string());

    if let Some(neighbors) = adj.get(node) {
        for w in neighbors {
            if !data.indices.contains_key(w) {
                strongconnect(w, adj, data);
                let v_low = *data
                    .lowlinks
                    .get(node)
                    .expect("Tarjan: node lowlink must exist after init");
                let w_low = *data
                    .lowlinks
                    .get(w)
                    .expect("Tarjan: neighbor lowlink must exist after recursion");
                data.lowlinks.insert(node.to_string(), min(v_low, w_low));
            } else if data.on_stack.contains(w) {
                let v_low = *data
                    .lowlinks
                    .get(node)
                    .expect("Tarjan: node lowlink must exist after init");
                let w_index = *data
                    .indices
                    .get(w)
                    .expect("Tarjan: neighbor index must exist if visited");
                data.lowlinks.insert(node.to_string(), min(v_low, w_index));
            }
        }
    }

    let v_low = *data
        .lowlinks
        .get(node)
        .expect("Tarjan: node lowlink must exist after init");
    let v_index = *data
        .indices
        .get(node)
        .expect("Tarjan: node index must exist after init");

    if v_low == v_index {
        let mut scc = Vec::new();
        loop {
            let w = data
                .stack
                .pop()
                .expect("Tarjan: stack must contain node that was pushed");
            data.on_stack.remove(&w);
            scc.push(w.clone());
            if w == node {
                break;
            }
        }
        data.sccs.push(scc);
    }
}

pub fn print_cycles(cycles: &[Vec<String>], json_output: bool) {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "circularImports": cycles }))
                .expect("Failed to serialize circular imports to JSON")
        );
    } else if cycles.is_empty() {
        println!("No circular imports detected.");
    } else {
        println!("Circular imports detected ({} cycles):", cycles.len());
        for (i, cycle) in cycles.iter().enumerate() {
            let mut c = cycle.clone();
            c.reverse(); // Reverse to show cycle in discovery order for readability

            let cycle_str = if c.len() > 12 {
                let first_part = c[..5].join(" -> ");
                let last_part = c[c.len() - 5..].join(" -> ");
                format!(
                    "{} -> ... ({} intermediate) ... -> {}",
                    first_part,
                    c.len() - 10,
                    last_part
                )
            } else {
                c.join(" -> ")
            };

            println!("  Cycle {}: {}", i + 1, cycle_str);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{find_cycles, print_cycles};

    #[test]
    fn detects_simple_cycle() {
        let edges = vec![
            ("a".to_string(), "b".to_string(), "import".to_string()),
            ("b".to_string(), "a".to_string(), "import".to_string()),
        ];
        let cycles = find_cycles(&edges);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);
        assert!(cycles[0].contains(&"a".to_string()));
        assert!(cycles[0].contains(&"b".to_string()));
    }

    #[test]
    fn detects_self_loop() {
        let edges = vec![("a".to_string(), "a".to_string(), "import".to_string())];
        let cycles = find_cycles(&edges);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 1);
        assert_eq!(cycles[0][0], "a");
    }

    #[test]
    fn no_cycle() {
        let edges = vec![
            ("a".to_string(), "b".to_string(), "import".to_string()),
            ("b".to_string(), "c".to_string(), "import".to_string()),
        ];
        let cycles = find_cycles(&edges);
        assert!(cycles.is_empty());
    }

    #[test]
    fn complex_cycle() {
        // a->b->c->a (cycle)
        // d->e
        let edges = vec![
            ("a".to_string(), "b".to_string(), "import".to_string()),
            ("b".to_string(), "c".to_string(), "import".to_string()),
            ("c".to_string(), "a".to_string(), "import".to_string()),
            ("d".to_string(), "e".to_string(), "import".to_string()),
        ];
        let cycles = find_cycles(&edges);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn multiple_cycles() {
        // Two separate cycles: a<->b and c<->d
        let edges = vec![
            ("a".to_string(), "b".to_string(), "import".to_string()),
            ("b".to_string(), "a".to_string(), "import".to_string()),
            ("c".to_string(), "d".to_string(), "import".to_string()),
            ("d".to_string(), "c".to_string(), "import".to_string()),
        ];
        let cycles = find_cycles(&edges);
        assert_eq!(cycles.len(), 2);
    }

    #[test]
    fn nested_cycles() {
        // a->b->c->d->a forms a single cycle
        let edges = vec![
            ("a".to_string(), "b".to_string(), "import".to_string()),
            ("b".to_string(), "c".to_string(), "import".to_string()),
            ("c".to_string(), "d".to_string(), "import".to_string()),
            ("d".to_string(), "a".to_string(), "import".to_string()),
        ];
        let cycles = find_cycles(&edges);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 4);
    }

    #[test]
    fn empty_graph() {
        let edges: Vec<(String, String, String)> = vec![];
        let cycles = find_cycles(&edges);
        assert!(cycles.is_empty());
    }

    #[test]
    fn single_node_no_self_loop() {
        let edges = vec![("a".to_string(), "b".to_string(), "import".to_string())];
        let cycles = find_cycles(&edges);
        assert!(cycles.is_empty());
    }

    #[test]
    fn print_cycles_empty() {
        // Should not panic
        print_cycles(&[], false);
    }

    #[test]
    fn print_cycles_json() {
        let cycles = vec![vec!["a".to_string(), "b".to_string()]];
        // Should not panic
        print_cycles(&cycles, true);
    }

    #[test]
    fn print_cycles_text() {
        let cycles = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "d".to_string(), "e".to_string()],
        ];
        // Should not panic
        print_cycles(&cycles, false);
    }

    #[test]
    fn detects_cycle_with_index_collapse() {
        // Barrel (index.ts) re-export + consumer importing the barrel should still form a cycle
        // after normalizing /index and TS/JS extensions.
        let edges = vec![
            (
                "src/features/ai-suite/index.ts".to_string(),
                "src/features/ai-suite/hooks/useFoo.ts".to_string(),
                "reexport".to_string(),
            ),
            (
                "src/features/ai-suite/hooks/useFoo.tsx".to_string(),
                "src/features/ai-suite/index.tsx".to_string(),
                "import".to_string(),
            ),
        ];
        let cycles = find_cycles(&edges);
        assert_eq!(cycles.len(), 1);
        let cycle = &cycles[0];
        assert_eq!(cycle.len(), 2);
        assert!(cycle.iter().any(|p| p.contains("ai-suite/index")));
        assert!(cycle.iter().any(|p| p.contains("hooks/useFoo")));
    }

    #[test]
    fn collapses_ts_js_family_into_cycles() {
        // Cross-extension imports (ts/js) should normalize to the same module id
        // so cycles are detectable even when files are split by extension.
        let edges = vec![
            (
                "src/utils/a.ts".to_string(),
                "src/utils/b.ts".to_string(),
                "import".to_string(),
            ),
            (
                "src/utils/b.js".to_string(),
                "src/utils/a.js".to_string(),
                "import".to_string(),
            ),
        ];
        let cycles = find_cycles(&edges);
        assert_eq!(cycles.len(), 1);
        let cycle = &cycles[0];
        assert_eq!(cycle.len(), 2);
        assert!(cycle.iter().any(|p| p.contains("utils/a")));
        assert!(cycle.iter().any(|p| p.contains("utils/b")));
    }
}
