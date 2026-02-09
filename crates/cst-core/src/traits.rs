use crate::error::Result;

/// Validate structural integrity of a geometric/topological entity.
pub trait Validate {
    fn validate(&self) -> Result<()>;
}

/// Compute an axis-aligned bounding box.
pub trait BoundingBox {
    type Point;
    fn bounding_box(&self) -> (Self::Point, Self::Point);
}
