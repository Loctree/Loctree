use std::collections::{HashMap, HashSet};

use super::coverage::CommandUsage;
use super::report::{GraphComponent, GraphData, GraphNode};
use crate::types::FileAnalysis;

pub const MAX_GRAPH_NODES: usize = 8000;
pub const MAX_GRAPH_EDGES: usize = 12000;

fn layout_positions(comps: &[Vec<String>]) -> HashMap<String, (f32, f32)> {
    let cols = (comps.len() as f32).sqrt().ceil() as usize + 1;
    let spacing = 1200f32;
    let mut positions: HashMap<String, (f32, f32)> = HashMap::new();
    for (idx, comp) in comps.iter().enumerate() {
        let row = idx / cols;
        let col = idx % cols;
        let cx = (col as f32) * spacing;
        let cy = (row as f32) * spacing;
        let n = comp.len().max(1) as f32;
        let radius = 160.0 + 30.0 * n.sqrt();
        for (i, node) in comp.iter().enumerate() {
            let theta = (i as f32) * (std::f32::consts::TAU / n);
            let jitter = 12.0 * (i as f32 % 3.0) - 12.0;
            let x = cx + radius * theta.cos() + jitter;
            let y = cy + radius * theta.sin() - jitter;
            positions.insert(node.clone(), (x, y));
        }
    }
    positions
}

#[allow(clippy::type_complexity)]
fn compute_components(
    nodes: &[String],
    edges: &[(String, String, String)],
) -> (
    Vec<Vec<String>>,
    HashMap<String, usize>,
    HashMap<String, usize>,
) {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for n in nodes {
        adj.entry(n.clone()).or_default();
    }
    for (a, b, _) in edges {
        if a.is_empty() || b.is_empty() {
            continue;
        }
        let entry = adj.entry(a.clone()).or_default();
        if !entry.contains(b) {
            entry.push(b.clone());
        }
        let back = adj.entry(b.clone()).or_default();
        if !back.contains(a) {
            back.push(a.clone());
        }
    }

    let degrees: HashMap<String, usize> = adj.iter().map(|(k, v)| (k.clone(), v.len())).collect();

    let mut visited: HashSet<String> = HashSet::new();
    let mut comps: Vec<Vec<String>> = Vec::new();
    for n in nodes {
        if visited.contains(n) {
            continue;
        }
        let mut stack = vec![n.clone()];
        let mut comp = Vec::new();
        visited.insert(n.clone());
        while let Some(cur) = stack.pop() {
            comp.push(cur.clone());
            if let Some(neigh) = adj.get(&cur) {
                for nb in neigh {
                    if visited.insert(nb.clone()) {
                        stack.push(nb.clone());
                    }
                }
            }
        }
        comps.push(comp);
    }

    comps.sort_by(|a, b| {
        b.len().cmp(&a.len()).then(
            a.first()
                .unwrap_or(&String::new())
                .cmp(b.first().unwrap_or(&String::new())),
        )
    });

    let mut node_to_component: HashMap<String, usize> = HashMap::new();
    for (idx, comp) in comps.iter().enumerate() {
        let cid = idx + 1;
        for node in comp {
            node_to_component.insert(node.clone(), cid);
        }
    }

    (comps, node_to_component, degrees)
}

pub fn build_graph_data(
    analyses: &[FileAnalysis],
    graph_edges: &[(String, String, String)],
    loc_map: &HashMap<String, usize>,
    fe_commands: &CommandUsage,
    be_commands: &CommandUsage,
) -> Option<GraphData> {
    let mut nodes: HashSet<String> = analyses.iter().map(|a| a.path.clone()).collect();
    for (a, b, _) in graph_edges {
        if !a.is_empty() {
            nodes.insert(a.clone());
        }
        if !b.is_empty() {
            nodes.insert(b.clone());
        }
    }

    if nodes.is_empty() {
        return None;
    }
    if nodes.len() > MAX_GRAPH_NODES || graph_edges.len() > MAX_GRAPH_EDGES {
        eprintln!(
            "[loctree][warn] graph skipped ({} nodes, {} edges > limits)",
            nodes.len(),
            graph_edges.len()
        );
        return None;
    }

    let mut nodes_vec: Vec<String> = nodes.into_iter().collect();
    nodes_vec.sort();
    let (component_nodes, node_to_component, degrees) = compute_components(&nodes_vec, graph_edges);
    let positions = layout_positions(&component_nodes);
    let main_component_id = if component_nodes.is_empty() { 0 } else { 1 };

    let mut component_meta: Vec<GraphComponent> = Vec::new();
    for (idx, comp_nodes) in component_nodes.iter().enumerate() {
        let mut sorted_nodes = comp_nodes.clone();
        sorted_nodes.sort();
        let cid = idx + 1;
        let comp_set: HashSet<String> = sorted_nodes.iter().cloned().collect();
        let edge_count = graph_edges
            .iter()
            .filter(|(a, b, _)| comp_set.contains(a) && comp_set.contains(b))
            .count();
        let isolated_count = sorted_nodes
            .iter()
            .filter(|n| degrees.get(*n).cloned().unwrap_or(0) == 0)
            .count();
        let loc_sum: usize = sorted_nodes
            .iter()
            .map(|n| loc_map.get(n).cloned().unwrap_or(0))
            .sum();
        let sample = sorted_nodes.first().cloned().unwrap_or_default();

        let tauri_frontend = fe_commands
            .values()
            .flat_map(|locs| locs.iter())
            .filter(|(path, _, _)| comp_set.contains(path))
            .count();
        let tauri_backend = be_commands
            .values()
            .flat_map(|locs| locs.iter())
            .filter(|(path, _, _)| comp_set.contains(path))
            .count();
        let detached = main_component_id != 0 && cid != main_component_id;

        component_meta.push(GraphComponent {
            id: cid,
            size: sorted_nodes.len(),
            edge_count,
            nodes: sorted_nodes,
            isolated_count,
            sample,
            loc_sum,
            detached,
            tauri_frontend,
            tauri_backend,
        });
    }

    let graph_nodes: Vec<GraphNode> = nodes_vec
        .iter()
        .filter_map(|id| {
            if id.is_empty() {
                return None;
            }
            let (x, y) = positions.get(id).cloned().unwrap_or((0.0, 0.0));
            let loc = loc_map.get(id).cloned().unwrap_or(0);
            let label = id.rsplit('/').next().unwrap_or(id.as_str()).to_string();
            let component = *node_to_component.get(id).unwrap_or(&0);
            let degree = *degrees.get(id).unwrap_or(&0);
            let detached = main_component_id != 0 && component != main_component_id;
            Some(GraphNode {
                id: id.clone(),
                label,
                loc,
                x,
                y,
                component,
                degree,
                detached,
            })
        })
        .collect();

    Some(GraphData {
        nodes: graph_nodes,
        edges: graph_edges.to_vec(),
        components: component_meta,
        main_component_id,
    })
}
