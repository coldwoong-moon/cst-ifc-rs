// Integration tests for ifc_to_mesh module

use cst_ifc::ifc_to_mesh::*;
use cst_ifc::ifc_reader::IfcFaceData;
use cst_math::{DVec3, Vector3};

const EPSILON: f64 = 1e-6;

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < EPSILON
}

fn vec3_approx_eq(a: Vector3, b: Vector3) -> bool {
    approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z)
}

fn simple_face(vertices: Vec<DVec3>) -> IfcFaceData {
    IfcFaceData {
        outer: vertices,
        holes: vec![],
    }
}

#[test]
fn integration_single_triangle() {
    let triangle = simple_face(vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ]);
    let faces = vec![triangle];

    let mesh = faces_to_trimesh("triangle", &faces);

    assert_eq!(mesh.positions.len(), 3);
    assert_eq!(mesh.normals.len(), 3);
    assert_eq!(mesh.indices.len(), 3);
    assert_eq!(mesh.triangle_count(), 1);
}

#[test]
fn integration_quad_to_triangles() {
    let quad = simple_face(vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(1.0, 1.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ]);
    let faces = vec![quad];

    let mesh = faces_to_trimesh("quad", &faces);

    assert_eq!(mesh.positions.len(), 4);
    assert_eq!(mesh.triangle_count(), 2);
    assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
}

#[test]
fn integration_normal_computation() {
    let triangle = simple_face(vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ]);
    let faces = vec![triangle];

    let mesh = faces_to_trimesh("tri", &faces);

    // All normals should point in +Z direction for XY plane triangle
    for normal in &mesh.normals {
        let expected = Vector3::new(0.0, 0.0, 1.0);
        assert!(vec3_approx_eq(*normal, expected),
                "Expected {:?}, got {:?}", expected, normal);
    }
}

#[test]
fn integration_merge_multiple_meshes() {
    let mesh1 = faces_to_trimesh("m1", &[simple_face(vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ])]);

    let mesh2 = faces_to_trimesh("m2", &[simple_face(vec![
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(1.0, 1.0, 0.0),
        DVec3::new(0.0, 1.0, 0.0),
    ])]);

    let merged = merge_trimeshes(&[mesh1.clone(), mesh2.clone()]);

    assert_eq!(merged.triangle_count(),
               mesh1.triangle_count() + mesh2.triangle_count());
    assert_eq!(merged.positions.len(), 7); // 3 + 4
}
