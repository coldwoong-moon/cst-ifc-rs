//! B-spline and NURBS surface implementations.

use cst_math::{Point3, Vector3, DVec3};
use serde::{Deserialize, Serialize};

use super::Surface;
use crate::nurbs::deboor;

/// A B-spline surface defined by degrees, knot vectors, and a 2D grid of control points.
///
/// `control_points[i][j]` is the control point at row `i` (u-direction) and column `j` (v-direction).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BSplineSurface {
    pub degree_u: usize,
    pub degree_v: usize,
    pub knots_u: Vec<f64>,
    pub knots_v: Vec<f64>,
    pub control_points: Vec<Vec<Point3>>,
}

impl BSplineSurface {
    pub fn new(
        degree_u: usize,
        degree_v: usize,
        knots_u: Vec<f64>,
        knots_v: Vec<f64>,
        control_points: Vec<Vec<Point3>>,
    ) -> Self {
        let n_u = control_points.len();
        let n_v = control_points[0].len();
        debug_assert!(
            knots_u.len() == n_u + degree_u + 1,
            "knots_u length mismatch: {} != {} + {} + 1",
            knots_u.len(),
            n_u,
            degree_u
        );
        debug_assert!(
            knots_v.len() == n_v + degree_v + 1,
            "knots_v length mismatch: {} != {} + {} + 1",
            knots_v.len(),
            n_v,
            degree_v
        );
        Self {
            degree_u,
            degree_v,
            knots_u,
            knots_v,
            control_points,
        }
    }
}

impl Surface for BSplineSurface {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        deboor::surface_point(
            self.degree_u,
            self.degree_v,
            &self.knots_u,
            &self.knots_v,
            &self.control_points,
            u,
            v,
        )
    }

    fn normal_at(&self, u: f64, v: f64) -> Vector3 {
        let (du, dv) = deboor::surface_derivs(
            self.degree_u,
            self.degree_v,
            &self.knots_u,
            &self.knots_v,
            &self.control_points,
            u,
            v,
        );
        let n = du.cross(dv);
        let len = n.length();
        if len < 1e-15 {
            DVec3::Z
        } else {
            n / len
        }
    }

    fn domain_u(&self) -> (f64, f64) {
        let p = self.degree_u;
        (self.knots_u[p], self.knots_u[self.knots_u.len() - p - 1])
    }

    fn domain_v(&self) -> (f64, f64) {
        let p = self.degree_v;
        (self.knots_v[p], self.knots_v[self.knots_v.len() - p - 1])
    }
}

/// A NURBS surface (rational B-spline surface).
///
/// Extends `BSplineSurface` with a 2D grid of weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NurbsSurface {
    pub degree_u: usize,
    pub degree_v: usize,
    pub knots_u: Vec<f64>,
    pub knots_v: Vec<f64>,
    pub control_points: Vec<Vec<Point3>>,
    pub weights: Vec<Vec<f64>>,
}

impl NurbsSurface {
    pub fn new(
        degree_u: usize,
        degree_v: usize,
        knots_u: Vec<f64>,
        knots_v: Vec<f64>,
        control_points: Vec<Vec<Point3>>,
        weights: Vec<Vec<f64>>,
    ) -> Self {
        let n_u = control_points.len();
        let n_v = control_points[0].len();
        debug_assert!(knots_u.len() == n_u + degree_u + 1);
        debug_assert!(knots_v.len() == n_v + degree_v + 1);
        debug_assert!(weights.len() == n_u);
        debug_assert!(weights[0].len() == n_v);
        Self {
            degree_u,
            degree_v,
            knots_u,
            knots_v,
            control_points,
            weights,
        }
    }
}

impl Surface for NurbsSurface {
    fn point_at(&self, u: f64, v: f64) -> Point3 {
        deboor::nurbs_surface_point(
            self.degree_u,
            self.degree_v,
            &self.knots_u,
            &self.knots_v,
            &self.control_points,
            &self.weights,
            u,
            v,
        )
    }

    fn normal_at(&self, u: f64, v: f64) -> Vector3 {
        deboor::nurbs_surface_normal(
            self.degree_u,
            self.degree_v,
            &self.knots_u,
            &self.knots_v,
            &self.control_points,
            &self.weights,
            u,
            v,
        )
    }

    fn domain_u(&self) -> (f64, f64) {
        let p = self.degree_u;
        (self.knots_u[p], self.knots_u[self.knots_u.len() - p - 1])
    }

    fn domain_v(&self) -> (f64, f64) {
        let p = self.degree_v;
        (self.knots_v[p], self.knots_v[self.knots_v.len() - p - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bilinear_surface() -> BSplineSurface {
        BSplineSurface::new(
            1,
            1,
            vec![0.0, 0.0, 1.0, 1.0],
            vec![0.0, 0.0, 1.0, 1.0],
            vec![
                vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(1.0, 0.0, 0.0)],
                vec![DVec3::new(0.0, 1.0, 0.0), DVec3::new(1.0, 1.0, 0.0)],
            ],
        )
    }

    #[test]
    fn test_bspline_surface_corners() {
        let surf = bilinear_surface();
        let p00 = surf.point_at(0.0, 0.0);
        assert!((p00 - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);

        let p10 = surf.point_at(1.0, 0.0);
        assert!((p10 - DVec3::new(0.0, 1.0, 0.0)).length() < 1e-10);

        let p01 = surf.point_at(0.0, 1.0);
        assert!((p01 - DVec3::new(1.0, 0.0, 0.0)).length() < 1e-10);

        let p11 = surf.point_at(1.0, 1.0);
        assert!((p11 - DVec3::new(1.0, 1.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_bspline_surface_center() {
        let surf = bilinear_surface();
        let p = surf.point_at(0.5, 0.5);
        assert!((p - DVec3::new(0.5, 0.5, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_bspline_surface_normal_flat() {
        let surf = bilinear_surface();
        let n = surf.normal_at(0.5, 0.5);
        // For a flat surface in XY, normal should be +Z or -Z
        assert!(
            (n - DVec3::Z).length() < 1e-10 || (n + DVec3::Z).length() < 1e-10,
            "Normal of flat surface should be +/-Z, got {:?}",
            n
        );
    }

    #[test]
    fn test_nurbs_surface_uniform_weights() {
        // NURBS surface with uniform weights should match B-spline
        let surf = NurbsSurface::new(
            1,
            1,
            vec![0.0, 0.0, 1.0, 1.0],
            vec![0.0, 0.0, 1.0, 1.0],
            vec![
                vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(1.0, 0.0, 0.0)],
                vec![DVec3::new(0.0, 1.0, 0.0), DVec3::new(1.0, 1.0, 0.0)],
            ],
            vec![vec![1.0, 1.0], vec![1.0, 1.0]],
        );

        let p = surf.point_at(0.5, 0.5);
        assert!((p - DVec3::new(0.5, 0.5, 0.0)).length() < 1e-10);
    }
}
