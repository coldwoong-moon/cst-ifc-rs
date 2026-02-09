use cst_core::error::{CstError, Result};
use cst_core::traits::Validate;

use super::mesh::Mesh;

impl Validate for Mesh {
    fn validate(&self) -> Result<()> {
        // 1. Validate twin symmetry
        for (he_id, he) in &self.halfedges {
            if let Some(twin_id) = he.twin {
                let twin = self.halfedges.get(twin_id).ok_or_else(|| {
                    CstError::Topology(format!(
                        "HalfEdge {:?} has twin {:?} that does not exist",
                        he_id, twin_id
                    ))
                })?;

                if twin.twin != Some(he_id) {
                    return Err(CstError::Topology(format!(
                        "Twin symmetry violated: {:?}.twin = {:?}, but {:?}.twin = {:?}",
                        he_id, twin_id, twin_id, twin.twin
                    )));
                }

                // Twin should have opposite direction
                if he.origin == twin.origin {
                    return Err(CstError::Topology(format!(
                        "HalfEdge {:?} and its twin {:?} have the same origin",
                        he_id, twin_id
                    )));
                }
            }
        }

        // 2. Validate next/prev chain forms closed loops for each face
        for (face_id, face) in &self.faces {
            let lp = self.loops.get(face.outer_loop).ok_or_else(|| {
                CstError::Topology(format!("Face {:?} references non-existent loop", face_id))
            })?;

            let start = lp.halfedge;
            let mut current = start;
            let mut count = 0;
            let max_iter = self.halfedges.len() + 1;

            loop {
                let he = self.halfedges.get(current).ok_or_else(|| {
                    CstError::Topology(format!(
                        "HalfEdge {:?} in face {:?} loop does not exist",
                        current, face_id
                    ))
                })?;

                // Check face assignment
                if he.face != Some(face_id) {
                    return Err(CstError::Topology(format!(
                        "HalfEdge {:?} in face {:?} loop has wrong face assignment: {:?}",
                        current, face_id, he.face
                    )));
                }

                // Check prev/next consistency
                if let Some(next_id) = he.next {
                    let next_he = self.halfedges.get(next_id).ok_or_else(|| {
                        CstError::Topology(format!(
                            "HalfEdge {:?}.next = {:?} does not exist",
                            current, next_id
                        ))
                    })?;
                    if next_he.prev != Some(current) {
                        return Err(CstError::Topology(format!(
                            "next/prev mismatch: {:?}.next = {:?}, but {:?}.prev = {:?}",
                            current, next_id, next_id, next_he.prev
                        )));
                    }
                }

                let next = he.next.ok_or_else(|| {
                    CstError::Topology(format!(
                        "HalfEdge {:?} in face {:?} has no next pointer",
                        current, face_id
                    ))
                })?;

                count += 1;
                if count > max_iter {
                    return Err(CstError::Topology(format!(
                        "Face {:?} loop does not close (infinite chain detected)",
                        face_id
                    )));
                }

                current = next;
                if current == start {
                    break;
                }
            }

            if count < 3 {
                return Err(CstError::Topology(format!(
                    "Face {:?} loop has fewer than 3 half-edges ({})",
                    face_id, count
                )));
            }
        }

        // 3. Validate edge consistency
        for (edge_id, edge) in &self.edges {
            let he_a = self.halfedges.get(edge.halfedge_a).ok_or_else(|| {
                CstError::Topology(format!(
                    "Edge {:?} references non-existent halfedge_a {:?}",
                    edge_id, edge.halfedge_a
                ))
            })?;
            let he_b = self.halfedges.get(edge.halfedge_b).ok_or_else(|| {
                CstError::Topology(format!(
                    "Edge {:?} references non-existent halfedge_b {:?}",
                    edge_id, edge.halfedge_b
                ))
            })?;

            if he_a.twin != Some(edge.halfedge_b) || he_b.twin != Some(edge.halfedge_a) {
                return Err(CstError::Topology(format!(
                    "Edge {:?} half-edges are not twins of each other",
                    edge_id
                )));
            }
        }

        Ok(())
    }
}
