//! Convert a half-edge topology Mesh to a TriangleMesh.

use cst_math::Point3;
use cst_topology::Mesh;

use crate::face_tessellator::tessellate_planar_face;
use crate::TriangleMesh;

/// Convert a `cst_topology::Mesh` to a `TriangleMesh`.
///
/// Each face in the topology mesh is tessellated independently using fan triangulation
/// on its vertex positions, then all face meshes are merged into a single result.
pub fn topology_mesh_to_triangles(mesh: &Mesh) -> TriangleMesh {
    let mut result = TriangleMesh::default();

    for (face_id, _face) in &mesh.faces {
        let vertex_iter = match mesh.face_vertices(face_id) {
            Some(iter) => iter,
            None => continue,
        };

        let positions: Vec<Point3> = vertex_iter
            .map(|vid| mesh.vertices[vid].position)
            .collect();

        if positions.len() < 3 {
            continue;
        }

        let face_mesh = tessellate_planar_face(&positions);
        result.merge(&face_mesh);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use cst_math::DVec3;

    #[test]
    fn test_single_triangle_topology_to_mesh() {
        let mut topo = Mesh::new();
        let v0 = topo.add_vertex(DVec3::new(0.0, 0.0, 0.0));
        let v1 = topo.add_vertex(DVec3::new(1.0, 0.0, 0.0));
        let v2 = topo.add_vertex(DVec3::new(0.0, 1.0, 0.0));
        topo.make_face(&[v0, v1, v2]).unwrap();

        let mesh = topology_mesh_to_triangles(&topo);
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn test_quad_topology_to_mesh() {
        let mut topo = Mesh::new();
        let v0 = topo.add_vertex(DVec3::new(0.0, 0.0, 0.0));
        let v1 = topo.add_vertex(DVec3::new(1.0, 0.0, 0.0));
        let v2 = topo.add_vertex(DVec3::new(1.0, 1.0, 0.0));
        let v3 = topo.add_vertex(DVec3::new(0.0, 1.0, 0.0));
        topo.make_face(&[v0, v1, v2, v3]).unwrap();

        let mesh = topology_mesh_to_triangles(&topo);
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.triangle_count(), 2);
    }

    #[test]
    fn test_two_triangles_topology_to_mesh() {
        let mut topo = Mesh::new();
        let v0 = topo.add_vertex(DVec3::new(0.0, 0.0, 0.0));
        let v1 = topo.add_vertex(DVec3::new(1.0, 0.0, 0.0));
        let v2 = topo.add_vertex(DVec3::new(0.5, 1.0, 0.0));
        let v3 = topo.add_vertex(DVec3::new(1.5, 1.0, 0.0));
        topo.make_face(&[v0, v1, v2]).unwrap();
        topo.make_face(&[v1, v3, v2]).unwrap();

        let mesh = topology_mesh_to_triangles(&topo);
        // Each face produces its own copy of vertices (no sharing across faces)
        assert_eq!(mesh.vertex_count(), 6);
        assert_eq!(mesh.triangle_count(), 2);
    }

    #[test]
    fn test_empty_topology() {
        let topo = Mesh::new();
        let mesh = topology_mesh_to_triangles(&topo);
        assert_eq!(mesh.vertex_count(), 0);
        assert_eq!(mesh.triangle_count(), 0);
    }
}
