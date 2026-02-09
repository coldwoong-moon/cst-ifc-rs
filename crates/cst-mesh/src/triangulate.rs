use cst_math::aabb::Aabb3;
use cst_math::{Point2, Point3, Vector3};

/// GPU-ready triangle mesh with interleaved vertex data.
#[derive(Debug, Clone, Default)]
pub struct TriangleMesh {
    pub positions: Vec<Point3>,
    pub normals: Vec<Vector3>,
    pub indices: Vec<u32>,
    pub uvs: Vec<Point2>,
}

impl TriangleMesh {
    /// Number of vertices in the mesh.
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Number of triangles in the mesh.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Merge another mesh into this one, offsetting indices appropriately.
    pub fn merge(&mut self, other: &TriangleMesh) {
        let offset = self.positions.len() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.uvs.extend_from_slice(&other.uvs);
        self.indices
            .extend(other.indices.iter().map(|&i| i + offset));
    }

    /// Compute flat (face) normals from triangle indices and assign to each vertex.
    ///
    /// For shared vertices this accumulates normals from all adjacent faces
    /// and normalizes the result (smooth shading approximation).
    pub fn compute_normals(&mut self) {
        let n = self.positions.len();
        self.normals.clear();
        self.normals.resize(n, Vector3::ZERO);

        for tri in self.indices.chunks_exact(3) {
            let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
            let p0 = self.positions[i0];
            let p1 = self.positions[i1];
            let p2 = self.positions[i2];
            let normal = (p1 - p0).cross(p2 - p0);
            self.normals[i0] += normal;
            self.normals[i1] += normal;
            self.normals[i2] += normal;
        }

        for n in &mut self.normals {
            let len = n.length();
            if len > 1e-12 {
                *n /= len;
            }
        }
    }

    /// Compute the axis-aligned bounding box of all positions.
    pub fn bounding_box(&self) -> Aabb3 {
        Aabb3::from_points(&self.positions).unwrap_or(Aabb3::new(Point3::ZERO, Point3::ZERO))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cst_math::DVec3;

    fn single_triangle() -> TriangleMesh {
        TriangleMesh {
            positions: vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![],
            indices: vec![0, 1, 2],
            uvs: vec![],
        }
    }

    #[test]
    fn test_vertex_and_triangle_count() {
        let mesh = single_triangle();
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn test_merge() {
        let mut a = single_triangle();
        let b = TriangleMesh {
            positions: vec![
                DVec3::new(2.0, 0.0, 0.0),
                DVec3::new(3.0, 0.0, 0.0),
                DVec3::new(2.0, 1.0, 0.0),
            ],
            normals: vec![],
            indices: vec![0, 1, 2],
            uvs: vec![],
        };
        a.merge(&b);
        assert_eq!(a.vertex_count(), 6);
        assert_eq!(a.triangle_count(), 2);
        // Second triangle indices should be offset by 3
        assert_eq!(a.indices[3], 3);
        assert_eq!(a.indices[4], 4);
        assert_eq!(a.indices[5], 5);
    }

    #[test]
    fn test_compute_normals() {
        let mut mesh = single_triangle();
        mesh.compute_normals();
        assert_eq!(mesh.normals.len(), 3);
        for n in &mesh.normals {
            // Normal should point in +Z direction for a CCW triangle on XY plane
            assert!((n.z - 1.0).abs() < 1e-10, "Expected +Z normal, got {:?}", n);
        }
    }

    #[test]
    fn test_bounding_box() {
        let mesh = single_triangle();
        let bb = mesh.bounding_box();
        assert_eq!(bb.min, DVec3::new(0.0, 0.0, 0.0));
        assert_eq!(bb.max, DVec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn test_empty_mesh() {
        let mesh = TriangleMesh::default();
        assert_eq!(mesh.vertex_count(), 0);
        assert_eq!(mesh.triangle_count(), 0);
        let bb = mesh.bounding_box();
        assert_eq!(bb.min, DVec3::ZERO);
        assert_eq!(bb.max, DVec3::ZERO);
    }
}
