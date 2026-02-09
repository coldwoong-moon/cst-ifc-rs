//! Adaptive tessellation of parametric surfaces.
//!
//! Recursively subdivides UV patches where the surface curvature exceeds a tolerance,
//! producing finer triangles in high-curvature regions and coarser triangles in flat areas.

use cst_geometry::Surface;
use cst_math::{Point2, Point3, Vector3};

use crate::TriangleMesh;

/// Maximum recursion depth to prevent infinite subdivision.
const MAX_DEPTH: u32 = 8;

/// Collects vertex/index data during recursive subdivision.
struct MeshBuilder<'a> {
    surface: &'a dyn Surface,
    tolerance: f64,
    u_domain: (f64, f64),
    v_domain: (f64, f64),
    positions: Vec<Point3>,
    normals: Vec<Vector3>,
    uvs: Vec<Point2>,
    indices: Vec<u32>,
}

impl<'a> MeshBuilder<'a> {
    fn new(surface: &'a dyn Surface, tolerance: f64) -> Self {
        Self {
            surface,
            tolerance,
            u_domain: surface.domain_u(),
            v_domain: surface.domain_v(),
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
        }
    }

    fn subdivide(&mut self, u0: f64, u1: f64, v0: f64, v1: f64, depth: u32) {
        let u_mid = (u0 + u1) * 0.5;
        let v_mid = (v0 + v1) * 0.5;

        let p00 = self.surface.point_at(u0, v0);
        let p10 = self.surface.point_at(u1, v0);
        let p01 = self.surface.point_at(u0, v1);
        let p11 = self.surface.point_at(u1, v1);
        let p_mid_true = self.surface.point_at(u_mid, v_mid);

        let p_mid_approx = (p00 + p10 + p01 + p11) * 0.25;
        let deviation = (p_mid_true - p_mid_approx).length();

        if deviation > self.tolerance && depth < MAX_DEPTH {
            self.subdivide(u0, u_mid, v0, v_mid, depth + 1);
            self.subdivide(u_mid, u1, v0, v_mid, depth + 1);
            self.subdivide(u0, u_mid, v_mid, v1, depth + 1);
            self.subdivide(u_mid, u1, v_mid, v1, depth + 1);
        } else {
            self.emit_quad(u0, u1, v0, v1);
        }
    }

    fn emit_quad(&mut self, u0: f64, u1: f64, v0: f64, v1: f64) {
        let base = self.positions.len() as u32;
        let u_range = self.u_domain.1 - self.u_domain.0;
        let v_range = self.v_domain.1 - self.v_domain.0;

        for &(u, v) in &[(u0, v0), (u1, v0), (u1, v1), (u0, v1)] {
            self.positions.push(self.surface.point_at(u, v));
            self.normals.push(self.surface.normal_at(u, v));
            self.uvs.push(Point2::new(
                (u - self.u_domain.0) / u_range,
                (v - self.v_domain.0) / v_range,
            ));
        }

        // Triangle 1: [0, 1, 2]
        self.indices.push(base);
        self.indices.push(base + 1);
        self.indices.push(base + 2);
        // Triangle 2: [0, 2, 3]
        self.indices.push(base);
        self.indices.push(base + 2);
        self.indices.push(base + 3);
    }

    fn into_mesh(self) -> TriangleMesh {
        TriangleMesh {
            positions: self.positions,
            normals: self.normals,
            indices: self.indices,
            uvs: self.uvs,
        }
    }
}

/// Adaptively tessellate a parametric surface based on a distance tolerance.
///
/// The algorithm starts with a coarse UV grid and recursively subdivides each quad
/// where the surface midpoint deviates from the bilinear interpolation by more than
/// `tolerance`.
///
/// # Arguments
/// * `surface` - The parametric surface to tessellate
/// * `tolerance` - Maximum allowed deviation (in 3D space) from the true surface
///
/// # Returns
/// A `TriangleMesh` with positions, normals, and UV coordinates.
pub fn adaptive_tessellate_surface(surface: &dyn Surface, tolerance: f64) -> TriangleMesh {
    let (u_min, u_max) = surface.domain_u();
    let (v_min, v_max) = surface.domain_v();

    let mut builder = MeshBuilder::new(surface, tolerance);

    // Start with a 4x4 initial grid
    let init_divs = 4usize;
    for i in 0..init_divs {
        for j in 0..init_divs {
            let u0 = u_min + (u_max - u_min) * i as f64 / init_divs as f64;
            let u1 = u_min + (u_max - u_min) * (i + 1) as f64 / init_divs as f64;
            let v0 = v_min + (v_max - v_min) * j as f64 / init_divs as f64;
            let v1 = v_min + (v_max - v_min) * (j + 1) as f64 / init_divs as f64;

            builder.subdivide(u0, u1, v0, v1, 0);
        }
    }

    builder.into_mesh()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cst_geometry::surface::{PlanarSurface, SphericalSurface};
    use cst_math::DVec3;

    #[test]
    fn test_adaptive_planar_surface() {
        let plane = PlanarSurface::new(DVec3::ZERO, DVec3::X, DVec3::Y);
        let mesh = adaptive_tessellate_surface(&plane, 0.01);

        // Should have exactly 4x4 = 16 quads = 32 triangles (no subdivision needed)
        assert_eq!(mesh.triangle_count(), 32);

        for p in &mesh.positions {
            assert!(p.z.abs() < 1e-10, "Point not on plane: z={}", p.z);
        }
    }

    #[test]
    fn test_adaptive_sphere_more_triangles_than_plane() {
        let sphere = SphericalSurface::new(DVec3::ZERO, 1.0);
        let mesh_tight = adaptive_tessellate_surface(&sphere, 0.001);
        let mesh_loose = adaptive_tessellate_surface(&sphere, 0.1);

        assert!(
            mesh_tight.triangle_count() > mesh_loose.triangle_count(),
            "Tight tolerance ({}) should produce more triangles than loose ({})",
            mesh_tight.triangle_count(),
            mesh_loose.triangle_count()
        );
    }

    #[test]
    fn test_adaptive_sphere_points_on_surface() {
        let radius = 3.0;
        let sphere = SphericalSurface::new(DVec3::ZERO, radius);
        let mesh = adaptive_tessellate_surface(&sphere, 0.01);

        for p in &mesh.positions {
            let dist = p.length();
            assert!(
                (dist - radius).abs() < 1e-10,
                "Point not on sphere: dist={}, expected={}",
                dist,
                radius
            );
        }
    }

    #[test]
    fn test_adaptive_indices_valid() {
        let sphere = SphericalSurface::new(DVec3::ZERO, 1.0);
        let mesh = adaptive_tessellate_surface(&sphere, 0.05);
        let n = mesh.vertex_count() as u32;
        for &idx in &mesh.indices {
            assert!(idx < n, "Index {} out of bounds (n={})", idx, n);
        }
    }

    #[test]
    fn test_adaptive_has_uvs() {
        let sphere = SphericalSurface::new(DVec3::ZERO, 1.0);
        let mesh = adaptive_tessellate_surface(&sphere, 0.05);
        assert_eq!(mesh.uvs.len(), mesh.vertex_count());

        for uv in &mesh.uvs {
            assert!(
                uv.x >= -1e-10 && uv.x <= 1.0 + 1e-10,
                "UV.x out of range: {}",
                uv.x
            );
            assert!(
                uv.y >= -1e-10 && uv.y <= 1.0 + 1e-10,
                "UV.y out of range: {}",
                uv.y
            );
        }
    }
}
