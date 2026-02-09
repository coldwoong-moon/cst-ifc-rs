use super::types::*;
use super::mesh::Mesh;

/// Iterator over half-edges around a face (follows `next` pointers).
pub struct FaceHalfEdgeIter<'a> {
    mesh: &'a Mesh,
    start: HalfEdgeId,
    current: Option<HalfEdgeId>,
    started: bool,
}

impl<'a> FaceHalfEdgeIter<'a> {
    pub fn new(mesh: &'a Mesh, start: HalfEdgeId) -> Self {
        Self {
            mesh,
            start,
            current: Some(start),
            started: false,
        }
    }
}

impl<'a> Iterator for FaceHalfEdgeIter<'a> {
    type Item = HalfEdgeId;

    fn next(&mut self) -> Option<HalfEdgeId> {
        let cur = self.current?;

        if self.started && cur == self.start {
            return None;
        }
        self.started = true;

        let he = self.mesh.halfedges.get(cur)?;
        self.current = he.next;
        Some(cur)
    }
}

/// Iterator over vertices around a face.
pub struct FaceVertexIter<'a> {
    inner: FaceHalfEdgeIter<'a>,
}

impl<'a> FaceVertexIter<'a> {
    pub fn new(mesh: &'a Mesh, start: HalfEdgeId) -> Self {
        Self {
            inner: FaceHalfEdgeIter::new(mesh, start),
        }
    }
}

impl<'a> Iterator for FaceVertexIter<'a> {
    type Item = VertexId;

    fn next(&mut self) -> Option<VertexId> {
        let he_id = self.inner.next()?;
        let he = self.inner.mesh.halfedges.get(he_id)?;
        Some(he.origin)
    }
}

/// Iterator over outgoing half-edges from a vertex (one-ring traversal).
/// Uses twin->next to circulate around the vertex.
pub struct VertexOutgoingIter<'a> {
    mesh: &'a Mesh,
    start: HalfEdgeId,
    current: Option<HalfEdgeId>,
    started: bool,
}

impl<'a> VertexOutgoingIter<'a> {
    pub fn new(mesh: &'a Mesh, start: HalfEdgeId) -> Self {
        Self {
            mesh,
            start,
            current: Some(start),
            started: false,
        }
    }
}

impl<'a> Iterator for VertexOutgoingIter<'a> {
    type Item = HalfEdgeId;

    fn next(&mut self) -> Option<HalfEdgeId> {
        let cur = self.current?;

        if self.started && cur == self.start {
            return None;
        }
        self.started = true;

        // Move to next outgoing half-edge: twin -> next
        let he = self.mesh.halfedges.get(cur)?;
        let twin_id = he.twin?;
        let twin = self.mesh.halfedges.get(twin_id)?;
        self.current = twin.next;

        // If twin.next is None (boundary), stop iteration
        if self.current.is_none() {
            self.current = None;
        }

        Some(cur)
    }
}

// --- Mesh iterator methods ---

impl Mesh {
    /// Iterate over half-edges around a face.
    pub fn face_halfedges(&self, face_id: FaceId) -> Option<FaceHalfEdgeIter<'_>> {
        let face = self.faces.get(face_id)?;
        let lp = self.loops.get(face.outer_loop)?;
        Some(FaceHalfEdgeIter::new(self, lp.halfedge))
    }

    /// Iterate over vertices around a face.
    pub fn face_vertices(&self, face_id: FaceId) -> Option<FaceVertexIter<'_>> {
        let face = self.faces.get(face_id)?;
        let lp = self.loops.get(face.outer_loop)?;
        Some(FaceVertexIter::new(self, lp.halfedge))
    }

    /// Iterate over outgoing half-edges from a vertex.
    pub fn vertex_outgoing(&self, vertex_id: VertexId) -> Option<VertexOutgoingIter<'_>> {
        let vertex = self.vertices.get(vertex_id)?;
        let he_id = vertex.halfedge?;
        Some(VertexOutgoingIter::new(self, he_id))
    }
}
