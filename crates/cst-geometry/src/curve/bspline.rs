//! B-spline and NURBS curve implementations.

use cst_math::{Point3, Vector3};
use serde::{Deserialize, Serialize};

use super::Curve;
use crate::nurbs::deboor;

/// A B-spline curve defined by degree, knot vector, and control points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BSplineCurve {
    pub degree: usize,
    pub knots: Vec<f64>,
    pub control_points: Vec<Point3>,
}

impl BSplineCurve {
    pub fn new(degree: usize, knots: Vec<f64>, control_points: Vec<Point3>) -> Self {
        debug_assert!(
            knots.len() == control_points.len() + degree + 1,
            "Knot vector length must be n + p + 1, got {} knots for {} CPs with degree {}",
            knots.len(),
            control_points.len(),
            degree
        );
        Self {
            degree,
            knots,
            control_points,
        }
    }
}

impl Curve for BSplineCurve {
    fn point_at(&self, t: f64) -> Point3 {
        deboor::curve_point(self.degree, &self.knots, &self.control_points, t)
    }

    fn tangent_at(&self, t: f64) -> Vector3 {
        deboor::curve_tangent(self.degree, &self.knots, &self.control_points, t)
    }

    fn domain(&self) -> (f64, f64) {
        let p = self.degree;
        (self.knots[p], self.knots[self.knots.len() - p - 1])
    }
}

/// A NURBS (Non-Uniform Rational B-Spline) curve.
///
/// Extends `BSplineCurve` with weights for rational evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NurbsCurve {
    pub degree: usize,
    pub knots: Vec<f64>,
    pub control_points: Vec<Point3>,
    pub weights: Vec<f64>,
}

impl NurbsCurve {
    pub fn new(
        degree: usize,
        knots: Vec<f64>,
        control_points: Vec<Point3>,
        weights: Vec<f64>,
    ) -> Self {
        debug_assert!(
            knots.len() == control_points.len() + degree + 1,
            "Knot vector length must be n + p + 1"
        );
        debug_assert!(
            control_points.len() == weights.len(),
            "Must have same number of weights as control points"
        );
        debug_assert!(
            weights.iter().all(|&w| w > 0.0),
            "All weights must be positive"
        );
        Self {
            degree,
            knots,
            control_points,
            weights,
        }
    }
}

impl Curve for NurbsCurve {
    fn point_at(&self, t: f64) -> Point3 {
        deboor::nurbs_curve_point(
            self.degree,
            &self.knots,
            &self.control_points,
            &self.weights,
            t,
        )
    }

    fn tangent_at(&self, t: f64) -> Vector3 {
        deboor::nurbs_curve_tangent(
            self.degree,
            &self.knots,
            &self.control_points,
            &self.weights,
            t,
        )
    }

    fn domain(&self) -> (f64, f64) {
        let p = self.degree;
        (self.knots[p], self.knots[self.knots.len() - p - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cst_math::DVec3;

    #[test]
    fn test_bspline_quadratic() {
        // Quadratic Bezier curve (degree 2, 3 control points)
        let curve = BSplineCurve::new(
            2,
            vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(0.5, 1.0, 0.0),
                DVec3::new(1.0, 0.0, 0.0),
            ],
        );

        // Endpoints should interpolate
        let p0 = curve.point_at(0.0);
        assert!((p0 - DVec3::new(0.0, 0.0, 0.0)).length() < 1e-10);

        let p1 = curve.point_at(1.0);
        assert!((p1 - DVec3::new(1.0, 0.0, 0.0)).length() < 1e-10);

        // Midpoint of quadratic Bezier: (1-t)^2 P0 + 2t(1-t) P1 + t^2 P2
        // At t=0.5: 0.25*P0 + 0.5*P1 + 0.25*P2 = (0.5, 0.5, 0)
        let pm = curve.point_at(0.5);
        assert!((pm.x - 0.5).abs() < 1e-10);
        assert!((pm.y - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_bspline_domain() {
        let curve = BSplineCurve::new(
            2,
            vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0],
            vec![
                DVec3::ZERO,
                DVec3::X,
                DVec3::Y,
                DVec3::Z,
                DVec3::ONE,
            ],
        );
        assert_eq!(curve.domain(), (0.0, 3.0));
    }

    #[test]
    fn test_nurbs_circle() {
        // Represent a unit circle as a NURBS curve (degree 2, 9 control points)
        let w = 1.0_f64 / 2.0_f64.sqrt();
        let curve = NurbsCurve::new(
            2,
            vec![0.0, 0.0, 0.0, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1.0, 1.0, 1.0],
            vec![
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(1.0, 1.0, 0.0),
                DVec3::new(0.0, 1.0, 0.0),
                DVec3::new(-1.0, 1.0, 0.0),
                DVec3::new(-1.0, 0.0, 0.0),
                DVec3::new(-1.0, -1.0, 0.0),
                DVec3::new(0.0, -1.0, 0.0),
                DVec3::new(1.0, -1.0, 0.0),
                DVec3::new(1.0, 0.0, 0.0),
            ],
            vec![1.0, w, 1.0, w, 1.0, w, 1.0, w, 1.0],
        );

        // Check that all points lie on the unit circle
        let (t_min, t_max) = curve.domain();
        for i in 0..=20 {
            let t = t_min + (t_max - t_min) * i as f64 / 20.0;
            let p = curve.point_at(t);
            let r = (p.x * p.x + p.y * p.y).sqrt();
            assert!(
                (r - 1.0).abs() < 1e-8,
                "NURBS circle point at t={} has radius {}, expected 1.0",
                t,
                r
            );
            assert!(p.z.abs() < 1e-10);
        }
    }

    #[test]
    fn test_bspline_tangent_direction() {
        // Straight line as B-spline: tangent should point in line direction
        let curve = BSplineCurve::new(
            1,
            vec![0.0, 0.0, 1.0, 1.0],
            vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 0.0, 0.0),
            ],
        );
        let t = curve.tangent_at(0.5);
        assert!(t.x > 0.0);
        assert!(t.y.abs() < 1e-10);
    }
}
