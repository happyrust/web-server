// Topology edge definitions

use super::NodeId;

#[derive(Debug, Clone)]
pub struct TopologyEdge {
    pub source: NodeId,
    pub target: NodeId,
}
