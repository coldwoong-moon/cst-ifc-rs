//! Spherical surface.

use std::f64::consts::PI;

use cst_math::{Point3, Vector3, DVec3};
use serde::{Deserialize, Serialize};

use super::Surface;

/// A spherical surface parameterized by longitude `u` in `[0, 2*PI]` and
/// latitude `v` in `[-PI/2, PI/2]`.
///
/// Points are computed as:
/// `P(u, v) = center + radius * (cos(v)*cos(u), cos(v)*sin(u), sin(v))`
/// (in the local frame aligned with the sphere)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SphericalSurface {
    pub center: Point3,
    pub radius: f64,
}

impl SphericalSurface {
    pub fn new(center: Point3, radius: f64) -> Self {
        Self { center, radius }
    }
}

impl Surface for SphericalSurface {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        let x = self.radius * v.cos() * u.cos();
        let y = self.radius * v.cos() * u.sin();
        let z = self.radius * v.sin();
        self.center + DVec3::new(x, y, z)
    }

    fn normal_at(&self, u: f64, v: f64) -> Vector3 {
        let x = v.cos() * u.cos();
        let y = v.cos() * u.sin();
        let z = v.sin();
        DVec3::new(x, y, z).normalize()
    }

    fn domain_u(&self) -> (f64, f64) {
        (0.0, 2.0 * PI)
    }

    fn domain_v(&self) -> (f64, f64) {
        (-PI / 2.0, PI / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spherical_points_on_sphere() {
        let sphere = SphericalSurface::new(DVec3::ZERO, 3.0);

        for i in 0..8 {
            for j in 0..4 {
                let u = i as f64 * PI / 4.0;
                let v = -PI / 2.0 + j as f64 * PI / 3.0;
                let p = sphere.point_at(u, v);
                let dist = p.length();
                assert!(
                    (dist - 3.0).abs() < 1e-10,
                    "Point at u={}, v={} not on sphere: dist={}",
                    u,
                    v,
                    dist
                );
            }
        }
    }

    #[test]
    fn test_spherical_north_pole() {
        let sphere = SphericalSurface::new(DVec3::ZERO, 1.0);
        let p = sphere.point_at(0.0, PI / 2.0);
        assert!((p - DVec3::new(0.0, 0.0, 1.0)).length() < 1e-10);
    }

    #[test]
    fn test_spherical_south_pole() {
        let sphere = SphericalSurface::new(DVec3::ZERO, 1.0);
        let p = sphere.point_at(0.0, -PI / 2.0);
        assert!((p - DVec3::new(0.0, 0.0, -1.0)).length() < 1e-10);
    }

    #[test]
    fn test_spherical_normal_outward() {
        let sphere = SphericalSurface::new(DVec3::ZERO, 2.0);

        for i in 0..8 {
            let u = i as f64 * PI / 4.0;
            let v = 0.0;
            let p = sphere.point_at(u, v);
            let n = sphere.normal_at(u, v);
            // Normal should be parallel to position vector (outward)
            let expected = p.normalize();
            assert!(
                (n - expected).length() < 1e-10,
                "Normal not outward at u={}, v={}",
                u,
                v
            );
        }
    }

    #[test]
    fn test_spherical_with_center() {
        let center = DVec3::new(1.0, 2.0, 3.0);
        let sphere = SphericalSurface::new(center, 1.0);
        let p = sphere.point_at(0.0, 0.0);
        let dist = (p - center).length();
        assert!((dist - 1.0).abs() < 1e-10);
    }
}
