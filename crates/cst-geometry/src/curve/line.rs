//! Line segment curve.

use cst_math::{Point3, Vector3};
use serde::{Deserialize, Serialize};

use super::Curve;

/// A line segment from `start` to `end`, parameterized over `[0, 1]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub start: Point3,
    pub end: Point3,
}

impl Line {
    pub fn new(start: Point3, end: Point3) -> Self {
        Self { start, end }
    }
}

impl Curve for Line {
    fn point_at(&self, t: f64) -> Point3 {
        self.start + t * (self.end - self.start)
    }

    fn tangent_at(&self, _t: f64) -> Vector3 {
        self.end - self.start
    }

    fn domain(&self) -> (f64, f64) {
        (0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cst_math::DVec3;

    #[test]
    fn test_line_point_at() {
        let line = Line::new(DVec3::new(0.0, 0.0, 0.0), DVec3::new(2.0, 4.0, 6.0));
        let p = line.point_at(0.5);
        assert!((p.x - 1.0).abs() < 1e-10);
        assert!((p.y - 2.0).abs() < 1e-10);
        assert!((p.z - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_line_endpoints() {
        let line = Line::new(DVec3::new(1.0, 2.0, 3.0), DVec3::new(4.0, 5.0, 6.0));
        let p0 = line.point_at(0.0);
        let p1 = line.point_at(1.0);
        assert!((p0 - line.start).length() < 1e-10);
        assert!((p1 - line.end).length() < 1e-10);
    }

    #[test]
    fn test_line_tangent() {
        let line = Line::new(DVec3::new(0.0, 0.0, 0.0), DVec3::new(1.0, 0.0, 0.0));
        let t = line.tangent_at(0.5);
        assert!((t.x - 1.0).abs() < 1e-10);
        assert!(t.y.abs() < 1e-10);
        assert!(t.z.abs() < 1e-10);
    }

    #[test]
    fn test_line_domain() {
        let line = Line::new(DVec3::ZERO, DVec3::X);
        assert_eq!(line.domain(), (0.0, 1.0));
    }
}
