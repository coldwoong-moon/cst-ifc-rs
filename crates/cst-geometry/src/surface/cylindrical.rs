//! Cylindrical surface.

use std::f64::consts::PI;

use cst_math::{Point3, Vector3, DVec3};
use serde::{Deserialize, Serialize};

use super::Surface;

/// A cylindrical surface parameterized by angle `u` in `[0, 2*PI]` and height `v`.
///
/// Points are computed as:
/// `P(u, v) = origin + radius * (cos(u) * ref_dir + sin(u) * cross_dir) + v * axis`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CylindricalSurface {
    pub origin: Point3,
    pub axis: Vector3,
    pub radius: f64,
}

impl CylindricalSurface {
    pub fn new(origin: Point3, axis: Vector3, radius: f64) -> Self {
        Self {
            origin,
            axis: axis.normalize(),
            radius,
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

impl Surface for CylindricalSurface {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        let (ref_dir, cross_dir) = self.local_frame();
        self.origin
            + self.radius * (u.cos() * ref_dir + u.sin() * cross_dir)
            + v * self.axis
    }

    fn normal_at(&self, u: f64, _v: f64) -> Vector3 {
        let (ref_dir, cross_dir) = self.local_frame();
        let n = u.cos() * ref_dir + u.sin() * cross_dir;
        n.normalize()
    }

    fn domain_u(&self) -> (f64, f64) {
        (0.0, 2.0 * PI)
    }

    fn domain_v(&self) -> (f64, f64) {
        (-1e6, 1e6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cylindrical_point_on_cylinder() {
        let cyl = CylindricalSurface::new(DVec3::ZERO, DVec3::Z, 2.0);

        for i in 0..8 {
            let u = i as f64 * PI / 4.0;
            let p = cyl.point_at(u, 0.0);
            let r = (p.x * p.x + p.y * p.y).sqrt();
            assert!(
                (r - 2.0).abs() < 1e-10,
                "Point not on cylinder at u={}: r={}",
                u,
                r
            );
            assert!(p.z.abs() < 1e-10);
        }
    }

    #[test]
    fn test_cylindrical_height() {
        let cyl = CylindricalSurface::new(DVec3::ZERO, DVec3::Z, 1.0);
        let p = cyl.point_at(0.0, 5.0);
        assert!((p.z - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_cylindrical_normal_outward() {
        let cyl = CylindricalSurface::new(DVec3::ZERO, DVec3::Z, 1.0);

        for i in 0..8 {
            let u = i as f64 * PI / 4.0;
            let n = cyl.normal_at(u, 0.0);
            let p = cyl.point_at(u, 0.0);
            // Normal should point in same direction as position vector (outward)
            let radial = DVec3::new(p.x, p.y, 0.0).normalize();
            assert!(
                (n - radial).length() < 1e-10,
                "Normal not outward at u={}",
                u
            );
        }
    }
}
