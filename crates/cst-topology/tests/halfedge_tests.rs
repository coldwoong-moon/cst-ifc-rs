use cst_core::traits::{BoundingBox, Validate};
use cst_math::Point3;
use cst_topology::{Mesh, VertexId};
use cst_math::DVec3;

fn dvec3(x: f64, y: f64, z: f64) -> cst_math::Point3 {
    DVec3::new(x, y, z)
}

fn make_triangle_mesh() -> (Mesh, VertexId, VertexId, VertexId) {
    let mut mesh = Mesh::new();
    let v0 = mesh.add_vertex(dvec3(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(dvec3(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(dvec3(0.0, 1.0, 0.0));
    (mesh, v0, v1, v2)
}

#[test]
fn test_single_triangle_creation() {
    let (mut mesh, v0, v1, v2) = make_triangle_mesh();
    let face_id = mesh.make_triangle(v0, v1, v2).unwrap();

    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.faces.len(), 1);
    assert_eq!(mesh.edges.len(), 3);
    assert_eq!(mesh.halfedges.len(), 6); // 3 edges * 2 half-edges each

    // Validate should pass
    mesh.validate().unwrap();
}

#[test]
fn test_triangle_face_halfedge_traversal() {
    let (mut mesh, v0, v1, v2) = make_triangle_mesh();
    let face_id = mesh.make_triangle(v0, v1, v2).unwrap();

    // Traverse half-edges around the face
    let halfedges: Vec<_> = mesh.face_halfedges(face_id).unwrap().collect();
    assert_eq!(halfedges.len(), 3);

    // Each half-edge should belong to the face
    for &he_id in &halfedges {
        assert_eq!(mesh.halfedges[he_id].face, Some(face_id));
    }
}

#[test]
fn test_triangle_face_vertex_traversal() {
    let (mut mesh, v0, v1, v2) = make_triangle_mesh();
    let face_id = mesh.make_triangle(v0, v1, v2).unwrap();

    let vertices: Vec<_> = mesh.face_vertices(face_id).unwrap().collect();
    assert_eq!(vertices.len(), 3);

    // All three original vertices should be present
    assert!(vertices.contains(&v0));
    assert!(vertices.contains(&v1));
    assert!(vertices.contains(&v2));
}

#[test]
fn test_quad_face_creation() {
    let mut mesh = Mesh::new();
    let v0 = mesh.add_vertex(dvec3(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(dvec3(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(dvec3(1.0, 1.0, 0.0));
    let v3 = mesh.add_vertex(dvec3(0.0, 1.0, 0.0));

    let face_id = mesh.make_face(&[v0, v1, v2, v3]).unwrap();

    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.faces.len(), 1);
    assert_eq!(mesh.edges.len(), 4);
    assert_eq!(mesh.halfedges.len(), 8);

    let vertices: Vec<_> = mesh.face_vertices(face_id).unwrap().collect();
    assert_eq!(vertices.len(), 4);

    mesh.validate().unwrap();
}

#[test]
fn test_two_adjacent_triangles_shared_edge() {
    let mut mesh = Mesh::new();
    let v0 = mesh.add_vertex(dvec3(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(dvec3(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(dvec3(0.5, 1.0, 0.0));
    let v3 = mesh.add_vertex(dvec3(0.5, -1.0, 0.0));

    // Triangle 1: v0-v1-v2 (CCW)
    let f1 = mesh.make_face(&[v0, v1, v2]).unwrap();
    // Triangle 2: v1-v0-v3 (CCW, shares edge v0-v1 with triangle 1)
    let f2 = mesh.make_face(&[v1, v0, v3]).unwrap();

    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.faces.len(), 2);
    // 3 edges for first triangle + 2 new edges for second = 5 total
    assert_eq!(mesh.edges.len(), 5);
    assert_eq!(mesh.halfedges.len(), 10);

    // Find the shared edge (v0-v1)
    // Both faces should reference it
    let mut shared_edge = None;
    for (edge_id, _) in &mesh.edges {
        let (fa, fb) = mesh.edge_faces(edge_id);
        if fa.is_some() && fb.is_some() {
            shared_edge = Some(edge_id);
            break;
        }
    }
    assert!(shared_edge.is_some(), "Should have a shared edge");

    let (fa, fb) = mesh.edge_faces(shared_edge.unwrap());
    let faces = vec![fa.unwrap(), fb.unwrap()];
    assert!(faces.contains(&f1));
    assert!(faces.contains(&f2));

    mesh.validate().unwrap();
}

#[test]
fn test_vertex_outgoing_iteration() {
    let (mut mesh, v0, v1, v2) = make_triangle_mesh();
    let _face_id = mesh.make_triangle(v0, v1, v2).unwrap();

    // v0 should have outgoing half-edges
    let outgoing: Vec<_> = mesh.vertex_outgoing(v0).unwrap().collect();
    assert!(!outgoing.is_empty());
    // In a single triangle, each vertex has at least one outgoing half-edge
    assert!(outgoing.len() >= 1);
}

#[test]
fn test_halfedge_target() {
    let (mut mesh, v0, v1, v2) = make_triangle_mesh();
    let _face_id = mesh.make_triangle(v0, v1, v2).unwrap();

    // Find a half-edge from v0
    let he_id = mesh.vertices[v0].halfedge.unwrap();
    let origin = mesh.halfedges[he_id].origin;
    assert_eq!(origin, v0);

    let target = mesh.halfedge_target(he_id).unwrap();
    assert!(target == v1 || target == v2);
}

#[test]
fn test_validate_passes_for_valid_mesh() {
    let (mut mesh, v0, v1, v2) = make_triangle_mesh();
    mesh.make_triangle(v0, v1, v2).unwrap();
    assert!(mesh.validate().is_ok());
}

#[test]
fn test_validate_fails_for_broken_twin() {
    let (mut mesh, v0, v1, v2) = make_triangle_mesh();
    mesh.make_triangle(v0, v1, v2).unwrap();

    // Break twin symmetry by corrupting a half-edge
    let first_he = mesh.halfedges.keys().next().unwrap();
    mesh.halfedges[first_he].twin = None;

    // Validation might still pass since twin=None is allowed (boundary),
    // but edge consistency check should fail since the edge still references
    // both half-edges as twins
    // The actual result depends on which invariant is checked
    // At minimum, the edge consistency check should catch this
    let result = mesh.validate();
    assert!(result.is_err());
}

#[test]
fn test_bounding_box() {
    let mut mesh = Mesh::new();
    mesh.add_vertex(dvec3(1.0, 2.0, 3.0));
    mesh.add_vertex(dvec3(-1.0, -2.0, -3.0));
    mesh.add_vertex(dvec3(5.0, 0.0, 1.0));

    let (min, max) = mesh.bounding_box();
    assert_eq!(min, dvec3(-1.0, -2.0, -3.0));
    assert_eq!(max, dvec3(5.0, 2.0, 3.0));
}

#[test]
fn test_bounding_box_empty_mesh() {
    let mesh = Mesh::new();
    let (min, max) = mesh.bounding_box();
    assert_eq!(min, Point3::ZERO);
    assert_eq!(max, Point3::ZERO);
}

#[test]
fn test_make_face_too_few_vertices() {
    let mut mesh = Mesh::new();
    let v0 = mesh.add_vertex(dvec3(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(dvec3(1.0, 0.0, 0.0));

    let result = mesh.make_face(&[v0, v1]);
    assert!(result.is_err());
}

#[test]
fn test_edge_faces_boundary_edge() {
    let (mut mesh, v0, v1, v2) = make_triangle_mesh();
    mesh.make_triangle(v0, v1, v2).unwrap();

    // In a single triangle, all edges are boundary edges
    // One half-edge has a face, the twin does not
    for (edge_id, _) in &mesh.edges {
        let (fa, fb) = mesh.edge_faces(edge_id);
        // Exactly one face should be assigned
        assert!(
            (fa.is_some() && fb.is_none()) || (fa.is_none() && fb.is_some()),
            "Boundary edge should have exactly one face"
        );
    }
}

#[test]
fn test_make_edge_standalone() {
    let mut mesh = Mesh::new();
    let v0 = mesh.add_vertex(dvec3(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(dvec3(1.0, 0.0, 0.0));

    let edge_id = mesh.make_edge(v0, v1).unwrap();
    assert_eq!(mesh.edges.len(), 1);
    assert_eq!(mesh.halfedges.len(), 2);

    let edge = &mesh.edges[edge_id];
    let he_a = &mesh.halfedges[edge.halfedge_a];
    let he_b = &mesh.halfedges[edge.halfedge_b];

    assert_eq!(he_a.origin, v0);
    assert_eq!(he_b.origin, v1);
    assert_eq!(he_a.twin, Some(edge.halfedge_b));
    assert_eq!(he_b.twin, Some(edge.halfedge_a));
}
