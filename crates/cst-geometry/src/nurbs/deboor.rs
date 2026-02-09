//! De Boor algorithm for B-spline and NURBS evaluation.

use cst_math::{Point3, Vector3, DVec3};

use super::knot::{basis_functions, basis_functions_derivs, find_span};

/// Evaluate a B-spline curve point at parameter `t` using the De Boor algorithm.
pub fn curve_point(degree: usize, knots: &[f64], control_points: &[Point3], t: f64) -> Point3 {
    let n = control_points.len() - 1;
    let span = find_span(degree, knots, n, t);
    let basis = basis_functions(degree, knots, span, t);

    let mut point = DVec3::ZERO;
    for i in 0..=degree {
        point += basis[i] * control_points[span - degree + i];
    }

    point
}

/// Evaluate the tangent (first derivative) of a B-spline curve at parameter `t`.
pub fn curve_tangent(
    degree: usize,
    knots: &[f64],
    control_points: &[Point3],
    t: f64,
) -> Vector3 {
    let n = control_points.len() - 1;
    let span = find_span(degree, knots, n, t);
    let (_, dn) = basis_functions_derivs(degree, knots, span, t);

    let mut tangent = DVec3::ZERO;
    for i in 0..=degree {
        tangent += dn[i] * control_points[span - degree + i];
    }

    tangent
}

/// Evaluate a rational B-spline (NURBS) curve point at parameter `t`.
#[allow(clippy::needless_range_loop)]
pub fn nurbs_curve_point(
    degree: usize,
    knots: &[f64],
    control_points: &[Point3],
    weights: &[f64],
    t: f64,
) -> Point3 {
    let n = control_points.len() - 1;
    let span = find_span(degree, knots, n, t);
    let basis = basis_functions(degree, knots, span, t);

    let mut point = DVec3::ZERO;
    let mut w = 0.0;

    for i in 0..=degree {
        let idx = span - degree + i;
        let bw = basis[i] * weights[idx];
        point += bw * control_points[idx];
        w += bw;
    }

    if w.abs() < 1e-15 {
        point
    } else {
        point / w
    }
}

/// Evaluate the tangent of a NURBS curve at parameter `t`.
#[allow(clippy::needless_range_loop)]
pub fn nurbs_curve_tangent(
    degree: usize,
    knots: &[f64],
    control_points: &[Point3],
    weights: &[f64],
    t: f64,
) -> Vector3 {
    let n = control_points.len() - 1;
    let span = find_span(degree, knots, n, t);
    let (basis, dbasis) = basis_functions_derivs(degree, knots, span, t);

    let mut a = DVec3::ZERO;
    let mut da = DVec3::ZERO;
    let mut w = 0.0;
    let mut dw = 0.0;

    for i in 0..=degree {
        let idx = span - degree + i;
        let bw = basis[i] * weights[idx];
        let dbw = dbasis[i] * weights[idx];
        a += bw * control_points[idx];
        da += dbw * control_points[idx];
        w += bw;
        dw += dbw;
    }

    if w.abs() < 1e-15 {
        da
    } else {
        let c = a / w;
        (da - dw * c) / w
    }
}

/// Evaluate a B-spline surface point at parameters `(u, v)`.
#[allow(clippy::needless_range_loop)]
pub fn surface_point(
    degree_u: usize,
    degree_v: usize,
    knots_u: &[f64],
    knots_v: &[f64],
    control_points: &[Vec<Point3>],
    u: f64,
    v: f64,
) -> Point3 {
    let n_u = control_points.len() - 1;
    let span_u = find_span(degree_u, knots_u, n_u, u);
    let basis_u = basis_functions(degree_u, knots_u, span_u, u);

    let n_v = control_points[0].len() - 1;
    let span_v = find_span(degree_v, knots_v, n_v, v);
    let basis_v = basis_functions(degree_v, knots_v, span_v, v);

    let mut point = DVec3::ZERO;
    for i in 0..=degree_u {
        let u_idx = span_u - degree_u + i;
        for j in 0..=degree_v {
            let v_idx = span_v - degree_v + j;
            point += basis_u[i] * basis_v[j] * control_points[u_idx][v_idx];
        }
    }

    point
}

/// Evaluate partial derivatives of a B-spline surface at `(u, v)`.
#[allow(clippy::needless_range_loop)]
pub fn surface_derivs(
    degree_u: usize,
    degree_v: usize,
    knots_u: &[f64],
    knots_v: &[f64],
    control_points: &[Vec<Point3>],
    u: f64,
    v: f64,
) -> (Vector3, Vector3) {
    let n_u = control_points.len() - 1;
    let span_u = find_span(degree_u, knots_u, n_u, u);
    let (basis_u, dbasis_u) = basis_functions_derivs(degree_u, knots_u, span_u, u);

    let n_v = control_points[0].len() - 1;
    let span_v = find_span(degree_v, knots_v, n_v, v);
    let (basis_v, dbasis_v) = basis_functions_derivs(degree_v, knots_v, span_v, v);

    let mut du = DVec3::ZERO;
    let mut dv = DVec3::ZERO;

    for i in 0..=degree_u {
        let u_idx = span_u - degree_u + i;
        for j in 0..=degree_v {
            let v_idx = span_v - degree_v + j;
            let cp = control_points[u_idx][v_idx];
            du += dbasis_u[i] * basis_v[j] * cp;
            dv += basis_u[i] * dbasis_v[j] * cp;
        }
    }

    (du, dv)
}

/// Evaluate a NURBS surface point at parameters `(u, v)`.
#[allow(clippy::needless_range_loop, clippy::too_many_arguments)]
pub fn nurbs_surface_point(
    degree_u: usize,
    degree_v: usize,
    knots_u: &[f64],
    knots_v: &[f64],
    control_points: &[Vec<Point3>],
    weights: &[Vec<f64>],
    u: f64,
    v: f64,
) -> Point3 {
    let n_u = control_points.len() - 1;
    let span_u = find_span(degree_u, knots_u, n_u, u);
    let basis_u = basis_functions(degree_u, knots_u, span_u, u);

    let n_v = control_points[0].len() - 1;
    let span_v = find_span(degree_v, knots_v, n_v, v);
    let basis_v = basis_functions(degree_v, knots_v, span_v, v);

    let mut point = DVec3::ZERO;
    let mut w = 0.0;

    for i in 0..=degree_u {
        let u_idx = span_u - degree_u + i;
        for j in 0..=degree_v {
            let v_idx = span_v - degree_v + j;
            let bw = basis_u[i] * basis_v[j] * weights[u_idx][v_idx];
            point += bw * control_points[u_idx][v_idx];
            w += bw;
        }
    }

    if w.abs() < 1e-15 {
        point
    } else {
        point / w
    }
}

/// Evaluate the normal of a NURBS surface at `(u, v)`.
#[allow(clippy::needless_range_loop, clippy::too_many_arguments)]
pub fn nurbs_surface_normal(
    degree_u: usize,
    degree_v: usize,
    knots_u: &[f64],
    knots_v: &[f64],
    control_points: &[Vec<Point3>],
    weights: &[Vec<f64>],
    u: f64,
    v: f64,
) -> Vector3 {
    let n_u = control_points.len() - 1;
    let span_u = find_span(degree_u, knots_u, n_u, u);
    let (basis_u, dbasis_u) = basis_functions_derivs(degree_u, knots_u, span_u, u);

    let n_v = control_points[0].len() - 1;
    let span_v = find_span(degree_v, knots_v, n_v, v);
    let (basis_v, dbasis_v) = basis_functions_derivs(degree_v, knots_v, span_v, v);

    let mut a = DVec3::ZERO;
    let mut da_u = DVec3::ZERO;
    let mut da_v = DVec3::ZERO;
    let mut w = 0.0;
    let mut dw_u = 0.0;
    let mut dw_v = 0.0;

    for i in 0..=degree_u {
        let u_idx = span_u - degree_u + i;
        for j in 0..=degree_v {
            let v_idx = span_v - degree_v + j;
            let cp = control_points[u_idx][v_idx];
            let wt = weights[u_idx][v_idx];

            let buv = basis_u[i] * basis_v[j] * wt;
            let dbu = dbasis_u[i] * basis_v[j] * wt;
            let dbv = basis_u[i] * dbasis_v[j] * wt;

            a += buv * cp;
            da_u += dbu * cp;
            da_v += dbv * cp;
            w += buv;
            dw_u += dbu;
            dw_v += dbv;
        }
    }

    if w.abs() < 1e-15 {
        return DVec3::Z;
    }

    let c = a / w;
    let du = (da_u - dw_u * c) / w;
    let dv = (da_v - dw_v * c) / w;

    let normal = du.cross(dv);
    let len = normal.length();
    if len < 1e-15 {
        DVec3::Z
    } else {
        normal / len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curve_point_linear() {
        let degree = 1;
        let knots = vec![0.0, 0.0, 1.0, 2.0, 2.0];
        let cps = vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
        ];

        let p = curve_point(degree, &knots, &cps, 0.5);
        assert!((p.x - 0.5).abs() < 1e-10);
        assert!(p.y.abs() < 1e-10);

        let p = curve_point(degree, &knots, &cps, 1.5);
        assert!((p.x - 1.0).abs() < 1e-10);
        assert!((p.y - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_curve_point_quadratic() {
        let degree = 2;
        let knots = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let cps = vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(0.5, 1.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
        ];

        let p = curve_point(degree, &knots, &cps, 0.0);
        assert!((p.x - 0.0).abs() < 1e-10);

        let p = curve_point(degree, &knots, &cps, 1.0);
        assert!((p.x - 1.0).abs() < 1e-10);

        let p = curve_point(degree, &knots, &cps, 0.5);
        assert!((p.x - 0.5).abs() < 1e-10);
        assert!((p.y - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_surface_point_bilinear() {
        let degree_u = 1;
        let degree_v = 1;
        let knots_u = vec![0.0, 0.0, 1.0, 1.0];
        let knots_v = vec![0.0, 0.0, 1.0, 1.0];
        let cps = vec![
            vec![DVec3::new(0.0, 0.0, 0.0), DVec3::new(1.0, 0.0, 0.0)],
            vec![DVec3::new(0.0, 1.0, 0.0), DVec3::new(1.0, 1.0, 0.0)],
        ];

        let p = surface_point(degree_u, degree_v, &knots_u, &knots_v, &cps, 0.5, 0.5);
        assert!((p.x - 0.5).abs() < 1e-10);
        assert!((p.y - 0.5).abs() < 1e-10);
        assert!(p.z.abs() < 1e-10);
    }
}
