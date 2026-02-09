use crate::{Point3, Vector3};
use serde::{Deserialize, Serialize};

/// A ray in 3D space defined by origin and direction.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Ray {
    pub origin: Point3,
    pub direction: Vector3,
}

impl Ray {
    pub fn new(origin: Point3, direction: Vector3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Get a point along the ray at parameter t.
    pub fn at(&self, t: f64) -> Point3 {
        self.origin + self.direction * t
    }

    /// Find the closest point on the ray to a given point.
    pub fn closest_point(&self, point: Point3) -> Point3 {
        let t = (point - self.origin).dot(self.direction).max(0.0);
        self.at(t)
    }

    /// Distance from a point to the ray.
    pub fn distance_to_point(&self, point: Point3) -> f64 {
        (point - self.closest_point(point)).length()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::dvec3;

    #[test]
    fn test_at() {
        let ray = Ray::new(dvec3(0.0, 0.0, 0.0), dvec3(1.0, 0.0, 0.0));
        let p = ray.at(5.0);
        assert!((p - dvec3(5.0, 0.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_distance_to_point() {
        let ray = Ray::new(dvec3(0.0, 0.0, 0.0), dvec3(1.0, 0.0, 0.0));
        let dist = ray.distance_to_point(dvec3(5.0, 3.0, 0.0));
        assert!((dist - 3.0).abs() < 1e-10);
    }
}
