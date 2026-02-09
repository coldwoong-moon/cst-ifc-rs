//! Face tessellation: convert planar polygons and parametric surfaces to triangle meshes.

use cst_geometry::Surface;
use cst_math::{Point2, Point3};

use crate::TriangleMesh;

/// Tessellate a planar face defined by an ordered list of vertices using fan triangulation.
///
/// The first vertex is used as the fan center. This works correctly for convex polygons
/// and is a reasonable approximation for mildly non-convex ones.
///
/// # Panics
/// Panics if `vertices` has fewer than 3 elements.
pub fn tessellate_planar_face(vertices: &[Point3]) -> TriangleMesh {
    assert!(
        vertices.len() >= 3,
        "Need at least 3 vertices for tessellation"
    );

    let n = vertices.len();
    let positions = vertices.to_vec();

    // Fan triangulation from vertex 0
    let mut indices = Vec::with_capacity((n - 2) * 3);
    for i in 1..n - 1 {
        indices.push(0u32);
        indices.push(i as u32);
        indices.push((i + 1) as u32);
    }

    let mut mesh = TriangleMesh {
        positions,
        normals: vec![],
        indices,
        uvs: vec![],
    };
    mesh.compute_normals();
    mesh
}

/// Tessellate a parametric surface by uniform subdivision in the UV domain.
///
/// Generates a `(u_divs+1) * (v_divs+1)` grid of vertices with positions, normals,
/// and UV coordinates, connected by `u_divs * v_divs * 2` triangles.
pub fn tessellate_surface(
    surface: &dyn Surface,
    u_divs: usize,
    v_divs: usize,
) -> TriangleMesh {
    let (u_min, u_max) = surface.domain_u();
    let (v_min, v_max) = surface.domain_v();

    let u_count = u_divs + 1;
    let v_count = v_divs + 1;
    let total_verts = u_count * v_count;

    let mut positions = Vec::with_capacity(total_verts);
    let mut normals = Vec::with_capacity(total_verts);
    let mut uvs = Vec::with_capacity(total_verts);

    for i in 0..u_count {
        let u = u_min + (u_max - u_min) * i as f64 / u_divs as f64;
        for j in 0..v_count {
            let v = v_min + (v_max - v_min) * j as f64 / v_divs as f64;
            positions.push(surface.point_at(u, v));
            normals.push(surface.normal_at(u, v));
            uvs.push(Point2::new(
                i as f64 / u_divs as f64,
                j as f64 / v_divs as f64,
            ));
        }
    }

    let mut indices = Vec::with_capacity(u_divs * v_divs * 6);
    for i in 0..u_divs {
        for j in 0..v_divs {
            let idx = |ii: usize, jj: usize| -> u32 { (ii * v_count + jj) as u32 };
            // First triangle
            indices.push(idx(i, j));
            indices.push(idx(i + 1, j));
            indices.push(idx(i + 1, j + 1));
            // Second triangle
            indices.push(idx(i, j));
            indices.push(idx(i + 1, j + 1));
            indices.push(idx(i, j + 1));
        }
    }

    TriangleMesh {
        positions,
        normals,
        indices,
        uvs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cst_geometry::surface::SphericalSurface;
    use cst_math::DVec3;

    #[test]
    fn test_tessellate_triangle() {
        let verts = vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
        ];
        let mesh = tessellate_planar_face(&verts);
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_tessellate_quad() {
        let verts = vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
        ];
        let mesh = tessellate_planar_face(&verts);
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.triangle_count(), 2);
        // Fan from vertex 0: [0,1,2] and [0,2,3]
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn test_tessellate_pentagon() {
        let verts: Vec<Point3> = (0..5)
            .map(|i| {
                let angle = std::f64::consts::TAU * i as f64 / 5.0;
                DVec3::new(angle.cos(), angle.sin(), 0.0)
            })
            .collect();
        let mesh = tessellate_planar_face(&verts);
        assert_eq!(mesh.vertex_count(), 5);
        assert_eq!(mesh.triangle_count(), 3);
    }

    #[test]
    fn test_tessellate_surface_sphere() {
        let sphere = SphericalSurface::new(DVec3::ZERO, 2.0);
        let mesh = tessellate_surface(&sphere, 16, 8);

        assert_eq!(mesh.vertex_count(), 17 * 9);
        assert_eq!(mesh.triangle_count(), 16 * 8 * 2);

        // All positions should be on the sphere (distance from center == radius)
        for p in &mesh.positions {
            let dist = p.length();
            assert!(
                (dist - 2.0).abs() < 1e-10,
                "Point not on sphere: dist={}",
                dist
            );
        }

        // All normals should be unit length
        for n in &mesh.normals {
            let len = n.length();
            assert!(
                (len - 1.0).abs() < 1e-10,
                "Normal not unit length: {}",
                len
            );
        }

        // UVs should be present
        assert_eq!(mesh.uvs.len(), mesh.vertex_count());
    }

    #[test]
    fn test_tessellate_surface_indices_valid() {
        let sphere = SphericalSurface::new(DVec3::ZERO, 1.0);
        let mesh = tessellate_surface(&sphere, 8, 4);
        let n = mesh.vertex_count() as u32;
        for &idx in &mesh.indices {
            assert!(idx < n, "Index {} out of bounds (n={})", idx, n);
        }
    }

    #[test]
    #[should_panic(expected = "Need at least 3 vertices")]
    fn test_tessellate_planar_face_too_few_vertices() {
        tessellate_planar_face(&[DVec3::ZERO, DVec3::X]);
    }
}
