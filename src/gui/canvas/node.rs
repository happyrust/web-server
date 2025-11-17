// Topology node definitions

use crate::gui::state::{RemoteSyncEnv, RemoteSyncSite};

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
