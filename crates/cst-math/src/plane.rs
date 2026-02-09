use crate::{Point3, Vector3};
use serde::{Deserialize, Serialize};

/// A plane in 3D space defined by a point and normal.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Plane {
    pub origin: Point3,
    pub normal: Vector3,
}

impl Plane {
    pub fn new(origin: Point3, normal: Vector3) -> Self {
        Self {
            origin,
            normal: normal.normalize(),
        }
    }

    pub fn xy() -> Self {
        Self::new(Point3::ZERO, Vector3::Z)
    }

    pub fn xz() -> Self {
        Self::new(Point3::ZERO, Vector3::Y)
    }

    pub fn yz() -> Self {
        Self::new(Point3::ZERO, Vector3::X)
    }

    /// Signed distance from a point to this plane.
    pub fn signed_distance(&self, point: Point3) -> f64 {
        (point - self.origin).dot(self.normal)
    }

    /// Project a point onto this plane.
    pub fn project_point(&self, point: Point3) -> Point3 {
        point - self.normal * self.signed_distance(point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::dvec3;

    #[test]
    fn test_signed_distance() {
        let plane = Plane::xy();
        assert!((plane.signed_distance(dvec3(0.0, 0.0, 5.0)) - 5.0).abs() < 1e-10);
        assert!((plane.signed_distance(dvec3(0.0, 0.0, -3.0)) + 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_project_point() {
        let plane = Plane::xy();
        let projected = plane.project_point(dvec3(1.0, 2.0, 5.0));
        assert!((projected - dvec3(1.0, 2.0, 0.0)).length() < 1e-10);
    }
}
