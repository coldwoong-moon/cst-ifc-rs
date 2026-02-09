use crate::{Point3, Vector3};
use serde::{Deserialize, Serialize};

/// Axis-Aligned Bounding Box in 3D space.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Aabb3 {
    pub min: Point3,
    pub max: Point3,
}

impl Aabb3 {
    pub fn new(min: Point3, max: Point3) -> Self {
        Self { min, max }
    }

    pub fn from_points(points: &[Point3]) -> Option<Self> {
        if points.is_empty() {
            return None;
        }
        let mut min = points[0];
        let mut max = points[0];
        for &p in &points[1..] {
            min = min.min(p);
            max = max.max(p);
        }
        Some(Self { min, max })
    }

    pub fn center(&self) -> Point3 {
        (self.min + self.max) * 0.5
    }

    pub fn extents(&self) -> Vector3 {
        self.max - self.min
    }

    pub fn contains_point(&self, p: Point3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn expand(&self, amount: f64) -> Self {
        let offset = Vector3::splat(amount);
        Self {
            min: self.min - offset,
            max: self.max + offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::dvec3;

    #[test]
    fn test_from_points() {
        let pts = vec![dvec3(1.0, 2.0, 3.0), dvec3(-1.0, 5.0, 0.0), dvec3(3.0, -1.0, 2.0)];
        let aabb = Aabb3::from_points(&pts).unwrap();
        assert_eq!(aabb.min, dvec3(-1.0, -1.0, 0.0));
        assert_eq!(aabb.max, dvec3(3.0, 5.0, 3.0));
    }

    #[test]
    fn test_contains_point() {
        let aabb = Aabb3::new(dvec3(0.0, 0.0, 0.0), dvec3(1.0, 1.0, 1.0));
        assert!(aabb.contains_point(dvec3(0.5, 0.5, 0.5)));
        assert!(!aabb.contains_point(dvec3(1.5, 0.5, 0.5)));
    }

    #[test]
    fn test_intersects() {
        let a = Aabb3::new(dvec3(0.0, 0.0, 0.0), dvec3(2.0, 2.0, 2.0));
        let b = Aabb3::new(dvec3(1.0, 1.0, 1.0), dvec3(3.0, 3.0, 3.0));
        let c = Aabb3::new(dvec3(5.0, 5.0, 5.0), dvec3(6.0, 6.0, 6.0));
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
    }
}
