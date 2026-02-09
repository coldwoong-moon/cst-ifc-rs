//! Ellipse curve.

use std::f64::consts::PI;

use cst_math::{Point3, Vector3, DVec3};
use serde::{Deserialize, Serialize};

use super::Curve;

/// An ellipse in 3D space, parameterized over `[0, 2*PI]`.
///
/// Defined by center, normal, major axis direction, and minor radius.
/// The major radius is the length of `major_axis`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ellipse {
    pub center: Point3,
    pub normal: Vector3,
    pub major_axis: Vector3,
    pub minor_radius: f64,
}

impl Ellipse {
    pub fn new(center: Point3, normal: Vector3, major_axis: Vector3, minor_radius: f64) -> Self {
        Self {
            center,
            normal: normal.normalize(),
            major_axis,
            minor_radius,
        }
    }

    /// Major radius (length of major_axis).
    pub fn major_radius(&self) -> f64 {
        self.major_axis.length()
    }

    /// Compute the minor axis direction (perpendicular to both normal and major axis).
    fn minor_axis(&self) -> DVec3 {
        self.normal.cross(self.major_axis).normalize()
    }
}

impl Curve for Ellipse {
    fn point_at(&self, t: f64) -> Point3 {
        let u = self.major_axis;
        let v = self.minor_axis() * self.minor_radius;
        self.center + t.cos() * u + t.sin() * v
    }

    fn tangent_at(&self, t: f64) -> Vector3 {
        let u = self.major_axis;
        let v = self.minor_axis() * self.minor_radius;
        -t.sin() * u + t.cos() * v
    }

    fn domain(&self) -> (f64, f64) {
        (0.0, 2.0 * PI)
    }

    fn is_closed(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ellipse_endpoints() {
        let ellipse = Ellipse::new(
            DVec3::ZERO,
            DVec3::Z,
            DVec3::new(2.0, 0.0, 0.0),
            1.0,
        );

        // t=0: at major axis end
        let p0 = ellipse.point_at(0.0);
        assert!((p0.x - 2.0).abs() < 1e-10);
        assert!(p0.y.abs() < 1e-10);

        // t=PI/2: at minor axis end
        let p1 = ellipse.point_at(PI / 2.0);
        assert!(p1.x.abs() < 1e-10);
        // minor axis direction depends on cross(Z, X) = -Y, so y should be -1.0
        assert!((p1.y.abs() - 1.0).abs() < 1e-10);

        // t=PI: at negative major axis
        let p2 = ellipse.point_at(PI);
        assert!((p2.x + 2.0).abs() < 1e-10);
        assert!(p2.y.abs() < 1e-10);
    }

    #[test]
    fn test_ellipse_on_plane() {
        let ellipse = Ellipse::new(
            DVec3::new(1.0, 2.0, 3.0),
            DVec3::Z,
            DVec3::new(2.0, 0.0, 0.0),
            1.0,
        );

        for i in 0..16 {
            let t = i as f64 * PI / 8.0;
            let p = ellipse.point_at(t);
            assert!(
                (p.z - 3.0).abs() < 1e-10,
                "Ellipse point not on plane at t={}",
                t
            );
        }
    }

    #[test]
    fn test_ellipse_is_closed() {
        let ellipse = Ellipse::new(DVec3::ZERO, DVec3::Z, DVec3::X, 1.0);
        assert!(ellipse.is_closed());
    }
}
