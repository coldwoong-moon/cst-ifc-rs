//! Circle curve.

use std::f64::consts::PI;

use cst_math::{Point3, Vector3, DVec3};
use serde::{Deserialize, Serialize};

use super::Curve;

/// A circle in 3D space, parameterized over `[0, 2*PI]`.
///
/// The circle lies in the plane defined by `center` and `normal`,
/// with the reference direction for `t=0` computed from the normal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circle {
    pub center: Point3,
    pub normal: Vector3,
    pub radius: f64,
}

impl Circle {
    pub fn new(center: Point3, normal: Vector3, radius: f64) -> Self {
        Self {
            center,
            normal: normal.normalize(),
            radius,
        }
    }

    /// Compute an orthonormal frame (u_axis, v_axis) in the circle plane.
    fn local_frame(&self) -> (DVec3, DVec3) {
        let n = self.normal;
        // Choose a vector not parallel to normal to build the frame
        let ref_vec = if n.x.abs() < 0.9 {
            DVec3::X
        } else {
            DVec3::Y
        };
        let u = n.cross(ref_vec).normalize();
        let v = n.cross(u).normalize();
        (u, v)
    }
}

impl Curve for Circle {
    fn point_at(&self, t: f64) -> Point3 {
        let (u, v) = self.local_frame();
        self.center + self.radius * (t.cos() * u + t.sin() * v)
    }

    fn tangent_at(&self, t: f64) -> Vector3 {
        let (u, v) = self.local_frame();
        self.radius * (-t.sin() * u + t.cos() * v)
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
    fn test_circle_points_on_circle() {
        let circle = Circle::new(DVec3::ZERO, DVec3::Z, 1.0);
        for i in 0..8 {
            let t = i as f64 * PI / 4.0;
            let p = circle.point_at(t);
            let dist = p.length();
            assert!(
                (dist - 1.0).abs() < 1e-10,
                "Point at t={} not on circle: dist={}",
                t,
                dist
            );
            assert!(p.z.abs() < 1e-10, "Point not in XY plane");
        }
    }

    #[test]
    fn test_circle_cardinal_points() {
        let circle = Circle::new(DVec3::ZERO, DVec3::Z, 2.0);
        let (u, v) = circle.local_frame();

        // t=0: should be at center + radius * u
        let p0 = circle.point_at(0.0);
        let expected = 2.0 * u;
        assert!((p0 - expected).length() < 1e-10);

        // t=PI/2: should be at center + radius * v
        let p1 = circle.point_at(PI / 2.0);
        let expected = 2.0 * v;
        assert!((p1 - expected).length() < 1e-10);

        // t=PI: should be at center - radius * u
        let p2 = circle.point_at(PI);
        let expected = -2.0 * u;
        assert!((p2 - expected).length() < 1e-10);

        // t=3PI/2: should be at center - radius * v
        let p3 = circle.point_at(3.0 * PI / 2.0);
        let expected = -2.0 * v;
        assert!((p3 - expected).length() < 1e-10);
    }

    #[test]
    fn test_circle_tangent_perpendicular() {
        let circle = Circle::new(DVec3::ZERO, DVec3::Z, 1.0);
        for i in 0..8 {
            let t = i as f64 * PI / 4.0;
            let p = circle.point_at(t);
            let tang = circle.tangent_at(t);
            // Tangent should be perpendicular to radius vector
            let dot = p.dot(tang);
            assert!(
                dot.abs() < 1e-10,
                "Tangent not perpendicular at t={}: dot={}",
                t,
                dot
            );
        }
    }

    #[test]
    fn test_circle_is_closed() {
        let circle = Circle::new(DVec3::ZERO, DVec3::Z, 1.0);
        assert!(circle.is_closed());
    }

    #[test]
    fn test_circle_domain() {
        let circle = Circle::new(DVec3::ZERO, DVec3::Z, 1.0);
        let (a, b) = circle.domain();
        assert!((a - 0.0).abs() < 1e-10);
        assert!((b - 2.0 * PI).abs() < 1e-10);
    }
}
