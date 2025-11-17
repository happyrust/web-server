// Topology canvas for visualizing environment and site connections

use crate::gui::state::{RemoteSyncEnv, RemoteSyncSite, TopologyData, TopologyConnection};
use std::collections::HashMap;

pub type NodeId = usize;

#[derive(Debug, Clone)]
pub struct TopologyNode {
    pub id: NodeId,
    pub node_type: NodeType,
    pub position: egui::Pos2,
    pub data: NodeData,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Environment,
    Site,
}

#[derive(Debug, Clone)]
pub enum NodeData {
    Environment(RemoteSyncEnv),
    Site(RemoteSyncSite),
}

#[derive(Debug, Clone)]
pub struct TopologyEdge {
    pub source: NodeId,
    pub target: NodeId,
}

pub struct TopologyCanvas {
    nodes: HashMap<NodeId, TopologyNode>,
    edges: Vec<TopologyEdge>,
    next_node_id: usize,
    zoom: f32,
    pan: egui::Vec2,
    dragging_node: Option<NodeId>,
}

impl TopologyCanvas {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            next_node_id: 0,
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            dragging_node: None,
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );
        
        let to_screen = |pos: egui::Pos2| -> egui::Pos2 {
            response.rect.min + (pos.to_vec2() * self.zoom + self.pan)
        };
        
        // Draw grid background
        self.draw_grid(&painter, &response.rect);
        
        // Draw edges
        for edge in &self.edges {
            if let (Some(source), Some(target)) = (self.nodes.get(&edge.source), self.nodes.get(&edge.target)) {
                let start = to_screen(source.position);
                let end = to_screen(target.position);
                
                painter.arrow(
                    start,
                    end - start,
                    egui::Stroke::new(2.0, egui::Color32::GRAY),
                );
            }
        }
        
        // Draw nodes
        for (_node_id, node) in &self.nodes {
            let screen_pos = to_screen(node.position);
            
            match node.node_type {
                NodeType::Environment => {
                    self.draw_environment_node(&painter, screen_pos, node);
                }
                NodeType::Site => {
                    self.draw_site_node(&painter, screen_pos, node);
                }
            }
        }
        
        // Handle interactions
        if response.dragged() {
            self.pan += response.drag_delta();
        }
        
        // Handle zoom
        if let Some(hover_pos) = response.hover_pos() {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                let zoom_delta = 1.0 + scroll_delta * 0.001;
                self.zoom = (self.zoom * zoom_delta).clamp(0.1, 5.0);
            }
        }
    }
    
    fn draw_grid(&self, painter: &egui::Painter, rect: &egui::Rect) {
        let grid_size = 50.0 * self.zoom;
        let color = egui::Color32::from_gray(230);
        
        // Draw vertical lines
        let mut x = rect.min.x + (self.pan.x % grid_size);
        while x < rect.max.x {
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(1.0, color),
            );
            x += grid_size;
        }
        
        // Draw horizontal lines
        let mut y = rect.min.y + (self.pan.y % grid_size);
        while y < rect.max.y {
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                egui::Stroke::new(1.0, color),
            );
            y += grid_size;
        }
    }
    
    fn draw_environment_node(&self, painter: &egui::Painter, pos: egui::Pos2, node: &TopologyNode) {
        let size = egui::vec2(150.0, 80.0) * self.zoom;
        let rect = egui::Rect::from_center_size(pos, size);
        
        // Draw rectangle
        painter.rect(
            rect,
            5.0,
            egui::Color32::from_rgb(200, 220, 255),
            egui::Stroke::new(2.0, egui::Color32::BLUE),
        );
        
        // Draw text
        if let NodeData::Environment(env) = &node.data {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &env.name,
                egui::FontId::proportional(14.0 * self.zoom),
                egui::Color32::BLACK,
            );
        }
    }
    
    fn draw_site_node(&self, painter: &egui::Painter, pos: egui::Pos2, node: &TopologyNode) {
        let radius = 40.0 * self.zoom;
        
        // Draw circle
        painter.circle(
            pos,
            radius,
            egui::Color32::from_rgb(200, 255, 200),
            egui::Stroke::new(2.0, egui::Color32::GREEN),
        );
        
        // Draw text
        if let NodeData::Site(site) = &node.data {
            painter.text(
                pos,
                egui::Align2::CENTER_CENTER,
                &site.name,
                egui::FontId::proportional(14.0 * self.zoom),
                egui::Color32::BLACK,
            );
        }
    }
    
    pub fn add_environment_node(&mut self, position: egui::Pos2) {
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        
        let env = RemoteSyncEnv {
            id: format!("env_{}", node_id),
            name: format!("环境 {}", node_id),
            mqtt_host: None,
            mqtt_port: None,
            file_server_host: None,
            location: String::new(),
            location_dbs: None,
            reconnect_initial_ms: Some(1000),
            reconnect_max_ms: Some(30000),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        
        self.nodes.insert(node_id, TopologyNode {
            id: node_id,
            node_type: NodeType::Environment,
            position,
            data: NodeData::Environment(env),
        });
    }
    
    pub fn add_site_node(&mut self, position: egui::Pos2) {
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        
        let site = RemoteSyncSite {
            id: format!("site_{}", node_id),
            env_id: String::new(),
            name: format!("站点 {}", node_id),
            location: String::new(),
            http_host: None,
            dbnums: None,
            notes: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        
        self.nodes.insert(node_id, TopologyNode {
            id: node_id,
            node_type: NodeType::Site,
            position,
            data: NodeData::Site(site),
        });
    }
    
    pub fn auto_layout(&mut self) {
        // Simple hierarchical layout algorithm
        let mut env_nodes = Vec::new();
        let mut site_nodes = Vec::new();
        
        for (id, node) in &self.nodes {
            match node.node_type {
                NodeType::Environment => env_nodes.push(*id),
                NodeType::Site => site_nodes.push(*id),
            }
        }
        
        // Arrange environment nodes in top layer
        let env_y = 100.0;
        let env_spacing = 200.0;
        for (i, node_id) in env_nodes.iter().enumerate() {
            if let Some(node) = self.nodes.get_mut(node_id) {
                node.position = egui::pos2(100.0 + i as f32 * env_spacing, env_y);
            }
        }
        
        // Arrange site nodes in bottom layer
        let site_y = 300.0;
        let site_spacing = 150.0;
        for (i, node_id) in site_nodes.iter().enumerate() {
            if let Some(node) = self.nodes.get_mut(node_id) {
                node.position = egui::pos2(100.0 + i as f32 * site_spacing, site_y);
            }
        }
    }
    
    pub fn to_topology_data(&self) -> TopologyData {
        let mut environments = Vec::new();
        let mut sites = Vec::new();
        let mut connections = Vec::new();
        
        for node in self.nodes.values() {
            match &node.data {
                NodeData::Environment(env) => environments.push(env.clone()),
                NodeData::Site(site) => sites.push(site.clone()),
            }
        }
        
        for edge in &self.edges {
            if let (Some(source), Some(target)) = (self.nodes.get(&edge.source), self.nodes.get(&edge.target)) {
                if let (NodeData::Environment(env), NodeData::Site(site)) = (&source.data, &target.data) {
                    connections.push(TopologyConnection {
                        env_id: env.id.clone(),
                        site_id: site.id.clone(),
                    });
                }
            }
        }
        
        TopologyData {
            environments,
            sites,
            connections,
        }
    }
    
    pub fn load_from_topology(&mut self, topology: &TopologyData) {
        self.nodes.clear();
        self.edges.clear();
        self.next_node_id = 0;
        
        // Load environment nodes
        for env in &topology.environments {
            let node_id = self.next_node_id;
            self.next_node_id += 1;
            
            self.nodes.insert(node_id, TopologyNode {
                id: node_id,
                node_type: NodeType::Environment,
                position: egui::pos2(100.0, 100.0),
                data: NodeData::Environment(env.clone()),
            });
        }
        
        // Load site nodes
        for site in &topology.sites {
            let node_id = self.next_node_id;
            self.next_node_id += 1;
            
            self.nodes.insert(node_id, TopologyNode {
                id: node_id,
                node_type: NodeType::Site,
                position: egui::pos2(100.0, 300.0),
                data: NodeData::Site(site.clone()),
            });
        }
        
        // Apply auto layout
        self.auto_layout();
    }
}

impl Default for TopologyCanvas {
    fn default() -> Self {
        Self::new()
    }
}
