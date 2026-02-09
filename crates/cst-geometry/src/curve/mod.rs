//! Curve traits and implementations.

mod line;
mod circle;
mod ellipse;
mod bspline;

use cst_math::{Point3, Vector3};

pub use line::Line;
pub use circle::Circle;
pub use ellipse::Ellipse;
pub use bspline::{BSplineCurve, NurbsCurve};

/// Trait for parametric curves in 3D space.
pub trait Curve: Send + Sync {
    /// Evaluate the curve at parameter `t`.
    fn point_at(&self, t: f64) -> Point3;

    /// Evaluate the tangent vector at parameter `t`.
    fn tangent_at(&self, t: f64) -> Vector3;

    /// Return the parameter domain `(t_min, t_max)`.
    fn domain(&self) -> (f64, f64);

    /// Whether the curve is closed (start == end).
    fn is_closed(&self) -> bool {
        false
    }
}
