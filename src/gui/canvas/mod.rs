// Topology canvas module

pub mod node;
pub mod edge;
pub mod layout;
pub mod renderer;

pub use node::{TopologyNode, NodeType, NodeData, NodeId};
pub use edge::TopologyEdge;
pub use renderer::TopologyCanvas;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CanvasMode {
    Select,
    AddEnvironment,
    AddSite,
    Connect,
}
