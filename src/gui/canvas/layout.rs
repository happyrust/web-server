// Layout algorithms for topology canvas

use super::{TopologyNode, NodeType};
use std::collections::HashMap;

pub fn auto_layout(nodes: &mut HashMap<usize, TopologyNode>) {
    let mut env_nodes = Vec::new();
    let mut site_nodes = Vec::new();
    
    for (id, node) in nodes.iter() {
        match node.node_type {
            NodeType::Environment => env_nodes.push(*id),
            NodeType::Site => site_nodes.push(*id),
        }
    }
    
    // Environment nodes in top layer
    let env_y = 100.0;
    let env_spacing = 200.0;
    for (i, node_id) in env_nodes.iter().enumerate() {
        if let Some(node) = nodes.get_mut(node_id) {
            node.position = egui::pos2(100.0 + i as f32 * env_spacing, env_y);
        }
    }
    
    // Site nodes in bottom layer
    let site_y = 300.0;
    let site_spacing = 150.0;
    for (i, node_id) in site_nodes.iter().enumerate() {
        if let Some(node) = nodes.get_mut(node_id) {
            node.position = egui::pos2(100.0 + i as f32 * site_spacing, site_y);
        }
    }
}
