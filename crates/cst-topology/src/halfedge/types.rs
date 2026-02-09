use cst_math::Point3;
use serde::{Deserialize, Serialize};
use slotmap::new_key_type;

// --- SlotMap key types ---

new_key_type! {
    pub struct VertexId;
    pub struct HalfEdgeId;
    pub struct EdgeId;
    pub struct LoopId;
    pub struct FaceId;
    pub struct ShellId;
    pub struct SolidId;
}

// --- Entity structs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vertex {
    pub position: Point3,
    pub halfedge: Option<HalfEdgeId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HalfEdge {
    pub origin: VertexId,
    pub twin: Option<HalfEdgeId>,
    pub next: Option<HalfEdgeId>,
    pub prev: Option<HalfEdgeId>,
    pub face: Option<FaceId>,
    pub edge: Option<EdgeId>,
    pub loop_id: Option<LoopId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Edge {
    pub halfedge_a: HalfEdgeId,
    pub halfedge_b: HalfEdgeId,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Loop {
    pub halfedge: HalfEdgeId,
    pub face: Option<FaceId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Face {
    pub outer_loop: LoopId,
    pub inner_loops: Vec<LoopId>,
    pub surface_reversed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shell {
    pub faces: Vec<FaceId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solid {
    pub outer_shell: ShellId,
    pub inner_shells: Vec<ShellId>,
}
