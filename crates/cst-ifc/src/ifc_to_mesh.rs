//! IFC FacetedBrep to Triangle Mesh Conversion
//!
//! Converts IFC polygon face data into indexed triangle meshes with computed normals.
//! Supports concave polygons and faces with holes via earcutr ear-clipping triangulation.

use cst_math::{DVec3, Point3, Vector3};
use crate::ifc_reader::IfcFaceData;

/// Triangle mesh data converted from IFC geometry.
/// Compatible with cst_mesh::TriangleMesh fields.
#[derive(Debug, Clone)]
pub struct IfcTriMesh {
    pub name: String,
    pub positions: Vec<Point3>,
    pub normals: Vec<Vector3>,
    pub indices: Vec<u32>,
}

impl IfcTriMesh {
    /// Create an empty mesh with the given name.
    pub fn new(name: String) -> Self {
        Self {
            name,
            positions: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Get the number of triangles in the mesh.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Convert a list of face data (outer boundary + optional holes) into a triangle mesh.
///
/// Uses fan triangulation as a fast path for simple convex faces (3-4 vertices, no holes),
/// and earcutr ear-clipping for concave polygons and faces with holes.
///
/// # Arguments
/// * `name` - Name for the resulting mesh
/// * `faces` - List of face data with outer boundary and optional hole boundaries
///
/// # Returns
/// A triangle mesh with positions, normals, and indices. Degenerate faces are skipped.
pub fn faces_to_trimesh(name: &str, faces: &[IfcFaceData]) -> IfcTriMesh {
    let mut mesh = IfcTriMesh::new(name.to_string());
    let mut vertex_offset = 0u32;

    for face in faces {
        let outer = &face.outer;

        // Skip degenerate faces
        if outer.len() < 3 {
            continue;
        }

        // Compute face normal using Newell's method on the outer boundary
        let normal = compute_face_normal(outer);

        // Skip faces with zero-area (degenerate)
        if normal.length_squared() < 1e-10 {
            continue;
        }

        if face.holes.is_empty() && outer.len() <= 4 {
            // Fast path: simple convex polygon (triangle or quad) without holes
            // Use fan triangulation
            let triangles = fan_triangulate(outer);

            for vertex in outer {
                mesh.positions.push(Point3::new(vertex.x, vertex.y, vertex.z));
                mesh.normals.push(normal);
            }

            for [i0, i1, i2] in triangles {
                mesh.indices.push(vertex_offset + i0 as u32);
                mesh.indices.push(vertex_offset + i1 as u32);
                mesh.indices.push(vertex_offset + i2 as u32);
            }

            vertex_offset += outer.len() as u32;
        } else {
            // General path: use earcutr for concave polygons and/or holes
            // Collect all vertices: outer boundary first, then holes
            let mut all_vertices: Vec<DVec3> = outer.clone();
            let mut hole_indices: Vec<usize> = Vec::new();

            for hole in &face.holes {
                hole_indices.push(all_vertices.len());
                all_vertices.extend_from_slice(hole);
            }

            // Project 3D vertices to 2D for earcutr
            let coords_2d = project_to_2d(&all_vertices, &normal);

            // Run earcutr
            let tri_result = earcutr::earcut(&coords_2d, &hole_indices, 2);

            let tri_indices = match tri_result {
                Ok(indices) if !indices.is_empty() => Some(indices),
                _ => None,
            };

            if let Some(tri_indices) = tri_indices {
                // Add all vertices (outer + holes)
                for vertex in &all_vertices {
                    mesh.positions.push(Point3::new(vertex.x, vertex.y, vertex.z));
                    mesh.normals.push(normal);
                }

                // Add triangle indices with offset
                for idx in &tri_indices {
                    mesh.indices.push(vertex_offset + *idx as u32);
                }

                vertex_offset += all_vertices.len() as u32;
            } else {
                // Fallback to fan triangulation on the outer boundary only
                // (earcutr can fail on degenerate inputs)
                for vertex in outer {
                    mesh.positions.push(Point3::new(vertex.x, vertex.y, vertex.z));
                    mesh.normals.push(normal);
                }

                let triangles = fan_triangulate(outer);
                for [i0, i1, i2] in triangles {
                    mesh.indices.push(vertex_offset + i0 as u32);
                    mesh.indices.push(vertex_offset + i1 as u32);
                    mesh.indices.push(vertex_offset + i2 as u32);
                }

                vertex_offset += outer.len() as u32;
            }
        }
    }

    mesh
}

/// Project 3D points to 2D coordinates for earcutr.
///
/// Uses the face normal to determine the dominant axis, then projects
/// to the other two axes for a robust 2D representation.
///
/// Returns a flat array of [x0, y0, x1, y1, ...] coordinates.
fn project_to_2d(vertices: &[DVec3], normal: &Vector3) -> Vec<f64> {
    let abs_nx = normal.x.abs();
    let abs_ny = normal.y.abs();
    let abs_nz = normal.z.abs();

    let mut coords = Vec::with_capacity(vertices.len() * 2);

    if abs_nz >= abs_nx && abs_nz >= abs_ny {
        // Normal is mostly Z - project to XY plane
        for v in vertices {
            coords.push(v.x);
            coords.push(v.y);
        }
    } else if abs_ny >= abs_nx {
        // Normal is mostly Y - project to XZ plane
        for v in vertices {
            coords.push(v.x);
            coords.push(v.z);
        }
    } else {
        // Normal is mostly X - project to YZ plane
        for v in vertices {
            coords.push(v.y);
            coords.push(v.z);
        }
    }

    coords
}

/// Triangulate a single convex polygon using fan triangulation from vertex 0.
///
/// For a polygon [v0, v1, v2, v3, ...], generates triangles:
///   (v0, v1, v2), (v0, v2, v3), (v0, v3, v4), ...
///
/// # Arguments
/// * `vertices` - The polygon vertices (must have at least 3 vertices)
///
/// # Returns
/// List of triangle indices (triplets of vertex indices)
fn fan_triangulate(vertices: &[DVec3]) -> Vec<[usize; 3]> {
    let n = vertices.len();
    if n < 3 {
        return Vec::new();
    }

    let mut triangles = Vec::with_capacity(n - 2);
    for i in 1..n - 1 {
        triangles.push([0, i, i + 1]);
    }
    triangles
}

/// Compute the face normal from a polygon's vertices using Newell's method.
///
/// Newell's method is more robust than simple cross products for non-planar polygons:
/// ```text
/// nx = sum((y_i - y_j) * (z_i + z_j)) for each edge (i, j)
/// ny = sum((z_i - z_j) * (x_i + x_j))
/// nz = sum((x_i - x_j) * (y_i + y_j))
/// ```
///
/// # Arguments
/// * `vertices` - The polygon vertices
///
/// # Returns
/// The normalized face normal vector. Returns zero vector for degenerate polygons.
fn compute_face_normal(vertices: &[DVec3]) -> Vector3 {
    if vertices.len() < 3 {
        return Vector3::ZERO;
    }

    let mut nx = 0.0;
    let mut ny = 0.0;
    let mut nz = 0.0;

    let n = vertices.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let vi = vertices[i];
        let vj = vertices[j];

        nx += (vi.y - vj.y) * (vi.z + vj.z);
        ny += (vi.z - vj.z) * (vi.x + vj.x);
        nz += (vi.x - vj.x) * (vi.y + vj.y);
    }

    let normal = Vector3::new(nx, ny, nz);
    let len_sq = normal.length_squared();

    if len_sq < 1e-10 {
        Vector3::ZERO
    } else {
        normal.normalize()
    }
}

/// Merge multiple IfcTriMesh into one, offsetting indices appropriately.
///
/// # Arguments
/// * `meshes` - List of meshes to merge
///
/// # Returns
/// A single merged mesh containing all geometry from input meshes.
/// The name is taken from the first mesh, or "merged" if the list is empty.
pub fn merge_trimeshes(meshes: &[IfcTriMesh]) -> IfcTriMesh {
    if meshes.is_empty() {
        return IfcTriMesh::new("merged".to_string());
    }

    let name = meshes[0].name.clone();
    let mut result = IfcTriMesh::new(name);

    let mut vertex_offset = 0u32;

    for mesh in meshes {
        // Copy positions and normals
        result.positions.extend_from_slice(&mesh.positions);
        result.normals.extend_from_slice(&mesh.normals);

        // Copy indices with offset
        for &idx in &mesh.indices {
            result.indices.push(idx + vertex_offset);
        }

        vertex_offset += mesh.positions.len() as u32;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-6;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    fn vec3_approx_eq(a: Vector3, b: Vector3) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z)
    }

    /// Helper: create a simple face from a list of points (no holes)
    fn simple_face(vertices: Vec<DVec3>) -> IfcFaceData {
        IfcFaceData {
            outer: vertices,
            holes: vec![],
        }
    }

    #[test]
    fn test_single_triangle_face() {
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

        // Check indices form one triangle
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_quad_face_becomes_two_triangles() {
        let quad = simple_face(vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
        ]);
        let faces = vec![quad];

        let mesh = faces_to_trimesh("quad", &faces);

        assert_eq!(mesh.positions.len(), 4);
        assert_eq!(mesh.normals.len(), 4);
        assert_eq!(mesh.indices.len(), 6); // 2 triangles
        assert_eq!(mesh.triangle_count(), 2);

        // Fan triangulation: (0,1,2), (0,2,3)
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn test_pentagon_face_uses_earcutr() {
        let pentagon = simple_face(vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(1.5, 0.5, 0.0),
            DVec3::new(0.5, 1.0, 0.0),
            DVec3::new(-0.5, 0.5, 0.0),
        ]);
        let faces = vec![pentagon];

        let mesh = faces_to_trimesh("pentagon", &faces);

        assert_eq!(mesh.positions.len(), 5);
        assert_eq!(mesh.triangle_count(), 3);
        assert_eq!(mesh.indices.len(), 9); // 3 triangles
    }

    #[test]
    fn test_multiple_faces_merged() {
        let tri1 = simple_face(vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
        ]);
        let tri2 = simple_face(vec![
            DVec3::new(2.0, 0.0, 0.0),
            DVec3::new(3.0, 0.0, 0.0),
            DVec3::new(2.0, 1.0, 0.0),
        ]);
        let faces = vec![tri1, tri2];

        let mesh = faces_to_trimesh("multi", &faces);

        assert_eq!(mesh.positions.len(), 6); // 3 + 3
        assert_eq!(mesh.triangle_count(), 2);
        assert_eq!(mesh.indices.len(), 6);

        // First triangle: indices 0,1,2
        // Second triangle: indices 3,4,5 (offset by 3)
        assert_eq!(mesh.indices, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_degenerate_face_skipped() {
        let degenerate = simple_face(vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
        ]);
        let valid = simple_face(vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
        ]);
        let faces = vec![degenerate, valid];

        let mesh = faces_to_trimesh("skip_degen", &faces);

        // Only the valid triangle should be included
        assert_eq!(mesh.positions.len(), 3);
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn test_compute_face_normal_triangle() {
        // Right triangle in XY plane, normal should point in +Z direction
        let vertices = vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
        ];

        let normal = compute_face_normal(&vertices);

        // Expected normal for counter-clockwise winding in XY plane
        let expected = Vector3::new(0.0, 0.0, 1.0);
        assert!(vec3_approx_eq(normal, expected),
                "Expected {:?}, got {:?}", expected, normal);
    }

    #[test]
    fn test_merge_preserves_triangle_count() {
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
        assert_eq!(merged.positions.len(),
                   mesh1.positions.len() + mesh2.positions.len());
    }

    #[test]
    fn test_empty_faces_returns_empty_mesh() {
        let mesh = faces_to_trimesh("empty", &[]);

        assert_eq!(mesh.positions.len(), 0);
        assert_eq!(mesh.normals.len(), 0);
        assert_eq!(mesh.indices.len(), 0);
        assert_eq!(mesh.triangle_count(), 0);
    }

    #[test]
    fn test_merge_empty_meshes() {
        let merged = merge_trimeshes(&[]);

        assert_eq!(merged.name, "merged");
        assert_eq!(merged.positions.len(), 0);
        assert_eq!(merged.triangle_count(), 0);
    }

    #[test]
    fn test_fan_triangulate_quad() {
        let vertices = vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
        ];

        let tris = fan_triangulate(&vertices);

        assert_eq!(tris.len(), 2);
        assert_eq!(tris[0], [0, 1, 2]);
        assert_eq!(tris[1], [0, 2, 3]);
    }

    #[test]
    fn test_concave_polygon_triangulation() {
        // L-shaped concave polygon in XY plane
        let l_shape = simple_face(vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(2.0, 0.0, 0.0),
            DVec3::new(2.0, 1.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
            DVec3::new(1.0, 2.0, 0.0),
            DVec3::new(0.0, 2.0, 0.0),
        ]);
        let faces = vec![l_shape];

        let mesh = faces_to_trimesh("concave", &faces);

        assert_eq!(mesh.positions.len(), 6);
        assert_eq!(mesh.triangle_count(), 4); // L-shape = 6 vertices = 4 triangles
        assert_eq!(mesh.indices.len(), 12);
    }

    #[test]
    fn test_face_with_hole() {
        // Outer square: (0,0) -> (10,0) -> (10,10) -> (0,10)
        // Inner hole: (3,3) -> (7,3) -> (7,7) -> (3,7)
        let face = IfcFaceData {
            outer: vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(10.0, 0.0, 0.0),
                DVec3::new(10.0, 10.0, 0.0),
                DVec3::new(0.0, 10.0, 0.0),
            ],
            holes: vec![vec![
                DVec3::new(3.0, 3.0, 0.0),
                DVec3::new(7.0, 3.0, 0.0),
                DVec3::new(7.0, 7.0, 0.0),
                DVec3::new(3.0, 7.0, 0.0),
            ]],
        };
        let faces = vec![face];

        let mesh = faces_to_trimesh("with_hole", &faces);

        // 8 vertices total (4 outer + 4 hole)
        assert_eq!(mesh.positions.len(), 8);
        // Should have triangles (the ring between outer and hole)
        assert!(mesh.triangle_count() >= 4, "Expected at least 4 triangles for ring, got {}", mesh.triangle_count());
    }

    #[test]
    fn test_project_to_2d_xy() {
        // Normal pointing in Z - should project to XY
        let normal = Vector3::new(0.0, 0.0, 1.0);
        let vertices = vec![
            DVec3::new(1.0, 2.0, 5.0),
            DVec3::new(3.0, 4.0, 5.0),
        ];

        let coords = project_to_2d(&vertices, &normal);
        assert_eq!(coords, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_project_to_2d_xz() {
        // Normal pointing in Y - should project to XZ
        let normal = Vector3::new(0.0, 1.0, 0.0);
        let vertices = vec![
            DVec3::new(1.0, 5.0, 2.0),
            DVec3::new(3.0, 5.0, 4.0),
        ];

        let coords = project_to_2d(&vertices, &normal);
        assert_eq!(coords, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_project_to_2d_yz() {
        // Normal pointing in X - should project to YZ
        let normal = Vector3::new(1.0, 0.0, 0.0);
        let vertices = vec![
            DVec3::new(5.0, 1.0, 2.0),
            DVec3::new(5.0, 3.0, 4.0),
        ];

        let coords = project_to_2d(&vertices, &normal);
        assert_eq!(coords, vec![1.0, 2.0, 3.0, 4.0]);
    }
}
