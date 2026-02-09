//! Conical surface.

use std::f64::consts::PI;

use cst_math::{Point3, Vector3, DVec3};
use serde::{Deserialize, Serialize};

use super::Surface;

/// A conical surface parameterized by angle `u` in `[0, 2*PI]` and distance `v` from apex.
///
/// Points are computed as:
/// `P(u, v) = apex + v * (sin(half_angle) * radial(u) + cos(half_angle) * axis)`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConicalSurface {
    pub apex: Point3,
    pub axis: Vector3,
    pub half_angle: f64,
}

impl ConicalSurface {
    pub fn new(apex: Point3, axis: Vector3, half_angle: f64) -> Self {
        Self {
            apex,
            axis: axis.normalize(),
            half_angle,
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

impl Surface for ConicalSurface {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        let (ref_dir, cross_dir) = self.local_frame();
        let radial = u.cos() * ref_dir + u.sin() * cross_dir;
        self.apex + v * (self.half_angle.sin() * radial + self.half_angle.cos() * self.axis)
    }

    fn normal_at(&self, u: f64, _v: f64) -> Vector3 {
        let (ref_dir, cross_dir) = self.local_frame();
        let radial = u.cos() * ref_dir + u.sin() * cross_dir;
        // Normal is perpendicular to the cone surface:
        // n = cos(half_angle) * radial - sin(half_angle) * axis
        let n = self.half_angle.cos() * radial - self.half_angle.sin() * self.axis;
        let len = n.length();
        if len < 1e-15 {
            radial
        } else {
            n / len
        }
    }

    fn domain_u(&self) -> (f64, f64) {
        (0.0, 2.0 * PI)
    }

    fn domain_v(&self) -> (f64, f64) {
        (0.0, 1e6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conical_apex() {
        let cone = ConicalSurface::new(DVec3::ZERO, DVec3::Z, PI / 4.0);
        let p = cone.point_at(0.0, 0.0);
        assert!(p.length() < 1e-10, "v=0 should be at apex");
    }

    #[test]
    fn test_conical_radius_grows() {
        let cone = ConicalSurface::new(DVec3::ZERO, DVec3::Z, PI / 4.0);
        let p1 = cone.point_at(0.0, 1.0);
        let p2 = cone.point_at(0.0, 2.0);
        let r1 = (p1.x * p1.x + p1.y * p1.y).sqrt();
        let r2 = (p2.x * p2.x + p2.y * p2.y).sqrt();
        assert!(r2 > r1, "Radius should grow with v");
    }

    #[test]
    fn test_conical_half_angle() {
        let half_angle = PI / 6.0; // 30 degrees
        let cone = ConicalSurface::new(DVec3::ZERO, DVec3::Z, half_angle);
        let v = 2.0;
        let p = cone.point_at(0.0, v);
        let r = (p.x * p.x + p.y * p.y).sqrt();
        let expected_r = v * half_angle.sin();
        assert!(
            (r - expected_r).abs() < 1e-10,
            "Radius at v={}: got {}, expected {}",
            v,
            r,
            expected_r
        );
    }
}
