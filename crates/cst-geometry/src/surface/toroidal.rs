//! Toroidal surface.

use std::f64::consts::PI;

use cst_math::{Point3, Vector3, DVec3};
use serde::{Deserialize, Serialize};

use super::Surface;

/// A toroidal surface (torus) parameterized by `u` (major angle) and `v` (minor angle),
/// both in `[0, 2*PI]`.
///
/// The torus is centered at `center` with the axis of symmetry along `axis`.
/// `major_radius` is the distance from center to the tube center.
/// `minor_radius` is the radius of the tube.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToroidalSurface {
    pub center: Point3,
    pub axis: Vector3,
    pub major_radius: f64,
    pub minor_radius: f64,
}

impl ToroidalSurface {
    pub fn new(center: Point3, axis: Vector3, major_radius: f64, minor_radius: f64) -> Self {
        Self {
            center,
            axis: axis.normalize(),
            major_radius,
            minor_radius,
        }
    }

    fn local_frame(&self) -> (DVec3, DVec3) {
        let n = self.axis;
        let ref_vec = if n.x.abs() < 0.9 { DVec3::X } else { DVec3::Y };
        let u = n.cross(ref_vec).normalize();
        let v = n.cross(u).normalize();
        (u, v)
    }
}

impl Surface for ToroidalSurface {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        let (x_dir, y_dir) = self.local_frame();
        let radial = u.cos() * x_dir + u.sin() * y_dir;
        let tube_center = self.center + self.major_radius * radial;
        tube_center + self.minor_radius * (v.cos() * radial + v.sin() * self.axis)
    }

    fn normal_at(&self, u: f64, v: f64) -> Vector3 {
        let (x_dir, y_dir) = self.local_frame();
        let radial = u.cos() * x_dir + u.sin() * y_dir;
        let n = v.cos() * radial + v.sin() * self.axis;
        n.normalize()
    }

    fn domain_u(&self) -> (f64, f64) {
        (0.0, 2.0 * PI)
    }

    fn domain_v(&self) -> (f64, f64) {
        (0.0, 2.0 * PI)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toroidal_points_valid() {
        let torus = ToroidalSurface::new(DVec3::ZERO, DVec3::Z, 3.0, 1.0);

        // At v=0 (outer equator), distance from center should be major + minor
        let p = torus.point_at(0.0, 0.0);
        let dist_xy = (p.x * p.x + p.y * p.y).sqrt();
        assert!(
            (dist_xy - 4.0).abs() < 1e-10,
            "Outer equator distance: expected 4.0, got {}",
            dist_xy
        );

        // At v=PI (inner equator), distance from center should be major - minor
        let p = torus.point_at(0.0, PI);
        let dist_xy = (p.x * p.x + p.y * p.y).sqrt();
        assert!(
            (dist_xy - 2.0).abs() < 1e-10,
            "Inner equator distance: expected 2.0, got {}",
            dist_xy
        );
    }

    #[test]
    fn test_toroidal_top() {
        let torus = ToroidalSurface::new(DVec3::ZERO, DVec3::Z, 3.0, 1.0);
        // At v=PI/2, point should be at z=minor_radius, dist_xy=major_radius
        let p = torus.point_at(0.0, PI / 2.0);
        let dist_xy = (p.x * p.x + p.y * p.y).sqrt();
        assert!((dist_xy - 3.0).abs() < 1e-10);
        assert!((p.z - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_toroidal_symmetry() {
        let torus = ToroidalSurface::new(DVec3::ZERO, DVec3::Z, 3.0, 1.0);
        // Points at u=0 and u=PI should be symmetric about YZ plane
        let p1 = torus.point_at(0.0, 0.0);
        let p2 = torus.point_at(PI, 0.0);
        let dist1 = (p1.x * p1.x + p1.y * p1.y).sqrt();
        let dist2 = (p2.x * p2.x + p2.y * p2.y).sqrt();
        assert!((dist1 - dist2).abs() < 1e-10);
    }
}
