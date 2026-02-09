//! Planar surface.

use cst_math::{Point3, Vector3, DVec3};
use serde::{Deserialize, Serialize};

use super::Surface;

/// An infinite planar surface parameterized by `origin + u * u_axis + v * v_axis`.
///
/// The domain defaults to `[-1e6, 1e6]` in both u and v (effectively infinite).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanarSurface {
    pub origin: Point3,
    pub u_axis: Vector3,
    pub v_axis: Vector3,
}

impl PlanarSurface {
    pub fn new(origin: Point3, u_axis: Vector3, v_axis: Vector3) -> Self {
        Self {
            origin,
            u_axis,
            v_axis,
        }
    }

    /// XY plane centered at origin.
    pub fn xy() -> Self {
        Self::new(DVec3::ZERO, DVec3::X, DVec3::Y)
    }
}

impl Surface for PlanarSurface {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        self.origin + u * self.u_axis + v * self.v_axis
    }

    fn normal_at(&self, _u: f64, _v: f64) -> Vector3 {
        let n = self.u_axis.cross(self.v_axis);
        let len = n.length();
        if len < 1e-15 {
            DVec3::Z
        } else {
            n / len
        }
    }

    fn domain_u(&self) -> (f64, f64) {
        (-1e6, 1e6)
    }

    fn domain_v(&self) -> (f64, f64) {
        (-1e6, 1e6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planar_point() {
        let plane = PlanarSurface::xy();
        let p = plane.point_at(1.0, 2.0);
        assert!((p.x - 1.0).abs() < 1e-10);
        assert!((p.y - 2.0).abs() < 1e-10);
        assert!(p.z.abs() < 1e-10);
    }

    #[test]
    fn test_planar_normal() {
        let plane = PlanarSurface::xy();
        let n = plane.normal_at(0.0, 0.0);
        assert!((n - DVec3::Z).length() < 1e-10);
    }

    #[test]
    fn test_planar_normal_constant() {
        let plane = PlanarSurface::xy();
        let n1 = plane.normal_at(0.0, 0.0);
        let n2 = plane.normal_at(100.0, -50.0);
        assert!((n1 - n2).length() < 1e-10);
    }
}
