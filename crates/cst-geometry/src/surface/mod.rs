//! Surface traits and implementations.

mod planar;
mod cylindrical;
mod conical;
mod spherical;
mod toroidal;
mod bspline;

use cst_math::{Point3, Vector3};

pub use planar::PlanarSurface;
pub use cylindrical::CylindricalSurface;
pub use conical::ConicalSurface;
pub use spherical::SphericalSurface;
pub use toroidal::ToroidalSurface;
pub use bspline::{BSplineSurface, NurbsSurface};

/// Trait for parametric surfaces in 3D space.
pub trait Surface: Send + Sync {
    /// Evaluate the surface at parameters `(u, v)`.
    fn point_at(&self, u: f64, v: f64) -> Point3;

    /// Evaluate the surface normal at parameters `(u, v)`.
    fn normal_at(&self, u: f64, v: f64) -> Vector3;

    /// Return the u-parameter domain `(u_min, u_max)`.
    fn domain_u(&self) -> (f64, f64);

    /// Return the v-parameter domain `(v_min, v_max)`.
    fn domain_v(&self) -> (f64, f64);
}
