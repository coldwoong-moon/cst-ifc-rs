use cst_core::traits::BoundingBox;
use cst_math::Point3;

use super::mesh::Mesh;

impl BoundingBox for Mesh {
    type Point = Point3;

    fn bounding_box(&self) -> (Point3, Point3) {
        if self.vertices.is_empty() {
            return (Point3::ZERO, Point3::ZERO);
        }

        let mut min = Point3::splat(f64::INFINITY);
        let mut max = Point3::splat(f64::NEG_INFINITY);

        for (_, vertex) in &self.vertices {
            min = min.min(vertex.position);
            max = max.max(vertex.position);
        }

        (min, max)
    }
}
