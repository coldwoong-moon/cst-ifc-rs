use cst_core::error::{CstError, Result};
use cst_math::Point3;
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

use super::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh {
    pub vertices: SlotMap<VertexId, Vertex>,
    pub halfedges: SlotMap<HalfEdgeId, HalfEdge>,
    pub edges: SlotMap<EdgeId, Edge>,
    pub loops: SlotMap<LoopId, Loop>,
    pub faces: SlotMap<FaceId, Face>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: SlotMap::with_key(),
            halfedges: SlotMap::with_key(),
            edges: SlotMap::with_key(),
            loops: SlotMap::with_key(),
            faces: SlotMap::with_key(),
        }
    }

    pub fn add_vertex(&mut self, position: Point3) -> VertexId {
        self.vertices.insert(Vertex {
            position,
            halfedge: None,
        })
    }

    /// Create an edge between two vertices, returning the EdgeId.
    /// Creates two half-edges (twins) and links them.
    pub fn make_edge(&mut self, v1: VertexId, v2: VertexId) -> Result<EdgeId> {
        if !self.vertices.contains_key(v1) || !self.vertices.contains_key(v2) {
            return Err(CstError::NotFound("Vertex not found".into()));
        }

        let he_a = self.halfedges.insert(HalfEdge {
            origin: v1,
            twin: None,
            next: None,
            prev: None,
            face: None,
            edge: None,
            loop_id: None,
        });

        let he_b = self.halfedges.insert(HalfEdge {
            origin: v2,
            twin: Some(he_a),
            next: None,
            prev: None,
            face: None,
            edge: None,
            loop_id: None,
        });

        self.halfedges[he_a].twin = Some(he_b);

        let edge_id = self.edges.insert(Edge {
            halfedge_a: he_a,
            halfedge_b: he_b,
        });

        self.halfedges[he_a].edge = Some(edge_id);
        self.halfedges[he_b].edge = Some(edge_id);

        // Set outgoing half-edge for vertices if not set
        if self.vertices[v1].halfedge.is_none() {
            self.vertices[v1].halfedge = Some(he_a);
        }
        if self.vertices[v2].halfedge.is_none() {
            self.vertices[v2].halfedge = Some(he_b);
        }

        Ok(edge_id)
    }

    /// Create a face from an ordered list of vertices (CCW winding).
    /// Reuses existing edges/half-edges where possible.
    pub fn make_face(&mut self, vertices: &[VertexId]) -> Result<FaceId> {
        let n = vertices.len();
        if n < 3 {
            return Err(CstError::Topology(
                "A face requires at least 3 vertices".into(),
            ));
        }

        for &v in vertices {
            if !self.vertices.contains_key(v) {
                return Err(CstError::NotFound("Vertex not found".into()));
            }
        }

        // Collect or create half-edges for each edge of the face
        let mut face_halfedges = Vec::with_capacity(n);

        for i in 0..n {
            let v_from = vertices[i];
            let v_to = vertices[(i + 1) % n];

            // Try to find an existing half-edge from v_from to v_to that has no face
            let existing_he = self.find_halfedge(v_from, v_to);

            match existing_he {
                Some(he_id) => {
                    if self.halfedges[he_id].face.is_some() {
                        return Err(CstError::Topology(
                            "Half-edge already belongs to a face (non-manifold)".into(),
                        ));
                    }
                    face_halfedges.push(he_id);
                }
                None => {
                    // Check if the reverse half-edge exists (twin direction)
                    let reverse_he = self.find_halfedge(v_to, v_from);

                    if let Some(rev_id) = reverse_he {
                        // The edge exists but we need the other direction
                        let twin_id = self.halfedges[rev_id].twin;
                        if let Some(twin) = twin_id {
                            if self.halfedges[twin].face.is_some() {
                                return Err(CstError::Topology(
                                    "Half-edge already belongs to a face (non-manifold)".into(),
                                ));
                            }
                            face_halfedges.push(twin);
                        } else {
                            return Err(CstError::Topology(
                                "Edge exists but twin is missing".into(),
                            ));
                        }
                    } else {
                        // Create a new edge
                        let edge_id = self.make_edge(v_from, v_to)?;
                        let edge = &self.edges[edge_id];
                        face_halfedges.push(edge.halfedge_a);
                    }
                }
            }
        }

        // Create the face and loop
        let loop_id = self.loops.insert(Loop {
            halfedge: face_halfedges[0],
            face: None,
        });

        let face_id = self.faces.insert(Face {
            outer_loop: loop_id,
            inner_loops: Vec::new(),
            surface_reversed: false,
        });

        self.loops[loop_id].face = Some(face_id);

        // Link half-edges: next/prev chain + face/loop assignment
        for i in 0..n {
            let he = face_halfedges[i];
            let next_he = face_halfedges[(i + 1) % n];
            let prev_he = face_halfedges[(n + i - 1) % n];

            self.halfedges[he].next = Some(next_he);
            self.halfedges[he].prev = Some(prev_he);
            self.halfedges[he].face = Some(face_id);
            self.halfedges[he].loop_id = Some(loop_id);
        }

        Ok(face_id)
    }

    /// Convenience: create a triangular face.
    pub fn make_triangle(&mut self, v1: VertexId, v2: VertexId, v3: VertexId) -> Result<FaceId> {
        self.make_face(&[v1, v2, v3])
    }

    /// Find a half-edge going from `origin` to `target`.
    fn find_halfedge(&self, origin: VertexId, target: VertexId) -> Option<HalfEdgeId> {
        for (he_id, he) in &self.halfedges {
            if he.origin == origin {
                if let Some(twin_id) = he.twin {
                    if self.halfedges[twin_id].origin == target {
                        return Some(he_id);
                    }
                }
            }
        }
        None
    }

    /// Get the target (destination) vertex of a half-edge.
    pub fn halfedge_target(&self, he_id: HalfEdgeId) -> Option<VertexId> {
        let he = self.halfedges.get(he_id)?;
        let twin_id = he.twin?;
        let twin = self.halfedges.get(twin_id)?;
        Some(twin.origin)
    }

    /// Get both faces adjacent to an edge.
    pub fn edge_faces(&self, edge_id: EdgeId) -> (Option<FaceId>, Option<FaceId>) {
        let edge = match self.edges.get(edge_id) {
            Some(e) => e,
            None => return (None, None),
        };
        let face_a = self
            .halfedges
            .get(edge.halfedge_a)
            .and_then(|he| he.face);
        let face_b = self
            .halfedges
            .get(edge.halfedge_b)
            .and_then(|he| he.face);
        (face_a, face_b)
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}
