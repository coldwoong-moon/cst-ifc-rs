//! Tessellation utilities for converting curves and surfaces to discrete representations.

use cst_math::Point3;

use crate::curve::Curve;
use crate::surface::Surface;

/// Convert a curve to a polyline using adaptive subdivision.
///
/// The algorithm recursively subdivides segments where the midpoint deviation
/// from the chord exceeds the given `tolerance`.
///
/// # Arguments
/// * `curve` - The curve to tessellate
/// * `tolerance` - Maximum allowed deviation from the true curve
///
/// # Returns
/// A vector of points approximating the curve.
pub fn curve_to_polyline(curve: &dyn Curve, tolerance: f64) -> Vec<Point3> {
    let (t_min, t_max) = curve.domain();
    let mut points = Vec::new();
    points.push(curve.point_at(t_min));
    subdivide_curve(curve, t_min, t_max, tolerance, &mut points, 0);
    points
}

/// Maximum recursion depth for adaptive subdivision.
const MAX_DEPTH: u32 = 12;

fn subdivide_curve(
    curve: &dyn Curve,
    t0: f64,
    t1: f64,
    tolerance: f64,
    points: &mut Vec<Point3>,
    depth: u32,
) {
    if depth >= MAX_DEPTH {
        points.push(curve.point_at(t1));
        return;
    }

    let t_mid = (t0 + t1) * 0.5;
    let p0 = curve.point_at(t0);
    let p1 = curve.point_at(t1);
    let p_mid = curve.point_at(t_mid);

    // Chord midpoint
    let chord_mid = (p0 + p1) * 0.5;
    let deviation = (p_mid - chord_mid).length();

    if deviation > tolerance {
        subdivide_curve(curve, t0, t_mid, tolerance, points, depth + 1);
        subdivide_curve(curve, t_mid, t1, tolerance, points, depth + 1);
    } else {
        points.push(curve.point_at(t1));
    }
}

/// Convert a surface to a triangle mesh using uniform parameter subdivision.
///
/// # Arguments
/// * `surface` - The surface to tessellate
/// * `u_divs` - Number of divisions in the u direction
/// * `v_divs` - Number of divisions in the v direction
///
/// # Returns
/// A tuple of `(vertices, triangles)` where each triangle is an array of 3 vertex indices.
pub fn surface_to_triangles(
    surface: &dyn Surface,
    u_divs: usize,
    v_divs: usize,
) -> (Vec<Point3>, Vec<[u32; 3]>) {
    let (u_min, u_max) = surface.domain_u();
    let (v_min, v_max) = surface.domain_v();

    let u_count = u_divs + 1;
    let v_count = v_divs + 1;

    // Generate vertices
    let mut vertices = Vec::with_capacity(u_count * v_count);
    for i in 0..u_count {
        let u = u_min + (u_max - u_min) * i as f64 / u_divs as f64;
        for j in 0..v_count {
            let v = v_min + (v_max - v_min) * j as f64 / v_divs as f64;
            vertices.push(surface.point_at(u, v));
        }
    }

    // Generate triangles (two triangles per quad)
    let mut triangles = Vec::with_capacity(u_divs * v_divs * 2);
    for i in 0..u_divs {
        for j in 0..v_divs {
            let idx = |ii: usize, jj: usize| -> u32 { (ii * v_count + jj) as u32 };

            // First triangle
            triangles.push([idx(i, j), idx(i + 1, j), idx(i + 1, j + 1)]);
            // Second triangle
            triangles.push([idx(i, j), idx(i + 1, j + 1), idx(i, j + 1)]);
        }
    }

    (vertices, triangles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Line;
    use crate::surface::PlanarSurface;
    use cst_math::DVec3;

    #[test]
    fn test_curve_to_polyline_line() {
        let line = Line::new(DVec3::ZERO, DVec3::new(10.0, 0.0, 0.0));
        let points = curve_to_polyline(&line, 0.01);
        // A line should produce exactly 2 points (no subdivision needed)
        assert_eq!(points.len(), 2);
        assert!((points[0] - DVec3::ZERO).length() < 1e-10);
        assert!((points[1] - DVec3::new(10.0, 0.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_curve_to_polyline_circle() {
        use crate::curve::Circle;

        let circle = Circle::new(DVec3::ZERO, DVec3::Z, 1.0);
        let points = curve_to_polyline(&circle, 0.01);
        // Circle should produce many points due to curvature
        assert!(
            points.len() > 10,
            "Circle should produce many points, got {}",
            points.len()
        );

        // All points should be on the circle
        for p in &points {
            let r = (p.x * p.x + p.y * p.y).sqrt();
            assert!(
                (r - 1.0).abs() < 0.02,
                "Point not on circle: r={}",
                r
            );
        }
    }

    #[test]
    fn test_surface_to_triangles_counts() {
        let plane = PlanarSurface::new(DVec3::ZERO, DVec3::X, DVec3::Y);
        let (vertices, triangles) = surface_to_triangles(&plane, 4, 3);

        assert_eq!(vertices.len(), 5 * 4); // (4+1) * (3+1) = 20
        assert_eq!(triangles.len(), 4 * 3 * 2); // 4 * 3 * 2 = 24
    }

    #[test]
    fn test_surface_to_triangles_indices_valid() {
        let plane = PlanarSurface::new(DVec3::ZERO, DVec3::X, DVec3::Y);
        let (vertices, triangles) = surface_to_triangles(&plane, 3, 3);

        let n = vertices.len() as u32;
        for tri in &triangles {
            for &idx in tri {
                assert!(idx < n, "Triangle index {} out of bounds (n={})", idx, n);
            }
        }
    }
}
