//! Knot vector utilities for B-spline/NURBS evaluation.

/// Find the knot span index for parameter `t` in the knot vector.
///
/// Returns the index `i` such that `knots[i] <= t < knots[i+1]`,
/// with special handling for the upper boundary.
///
/// # Arguments
/// * `degree` - Degree of the B-spline
/// * `knots` - The knot vector
/// * `n` - Number of control points minus 1
/// * `t` - Parameter value
pub fn find_span(degree: usize, knots: &[f64], n: usize, t: f64) -> usize {
    // Special case: t at upper boundary
    if t >= knots[n + 1] {
        return n;
    }
    if t <= knots[degree] {
        return degree;
    }

    // Binary search
    let mut low = degree;
    let mut high = n + 1;
    let mut mid = (low + high) / 2;

    while t < knots[mid] || t >= knots[mid + 1] {
        if t < knots[mid] {
            high = mid;
        } else {
            low = mid;
        }
        mid = (low + high) / 2;
    }

    mid
}

/// Compute the non-vanishing basis functions at parameter `t`.
///
/// Returns a vector of `degree + 1` basis function values N_{span-degree,degree}(t)
/// through N_{span,degree}(t).
///
/// # Arguments
/// * `degree` - Degree of the B-spline
/// * `knots` - The knot vector
/// * `span` - The knot span index (from `find_span`)
/// * `t` - Parameter value
pub fn basis_functions(degree: usize, knots: &[f64], span: usize, t: f64) -> Vec<f64> {
    let mut n = vec![0.0; degree + 1];
    let mut left = vec![0.0; degree + 1];
    let mut right = vec![0.0; degree + 1];

    n[0] = 1.0;

    for j in 1..=degree {
        left[j] = t - knots[span + 1 - j];
        right[j] = knots[span + j] - t;
        let mut saved = 0.0;

        for r in 0..j {
            let temp = n[r] / (right[r + 1] + left[j - r]);
            n[r] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }

        n[j] = saved;
    }

    n
}

/// Compute basis functions and their first derivatives at parameter `t`.
///
/// Returns `(N, dN)` where `N` contains basis function values and `dN` contains
/// their first derivatives.
pub fn basis_functions_derivs(
    degree: usize,
    knots: &[f64],
    span: usize,
    t: f64,
) -> (Vec<f64>, Vec<f64>) {
    let p = degree;

    // Compute basis functions (same as basis_functions but we also need the triangular table)
    let mut ndu = vec![vec![0.0; p + 1]; p + 1];
    let mut left = vec![0.0; p + 1];
    let mut right = vec![0.0; p + 1];

    ndu[0][0] = 1.0;

    for j in 1..=p {
        left[j] = t - knots[span + 1 - j];
        right[j] = knots[span + j] - t;
        let mut saved = 0.0;

        for r in 0..j {
            // Lower triangle
            ndu[j][r] = right[r + 1] + left[j - r];
            let temp = ndu[r][j - 1] / ndu[j][r];

            // Upper triangle
            ndu[r][j] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }
        ndu[j][j] = saved;
    }

    // Basis function values
    let mut n_vals = vec![0.0; p + 1];
    for j in 0..=p {
        n_vals[j] = ndu[j][p];
    }

    // Derivatives
    let mut dn_vals = vec![0.0; p + 1];
    let mut a = vec![vec![0.0; p + 1]; 2];

    for r in 0..=p {
        let mut s1 = 0usize;
        let mut s2 = 1usize;
        a[0][0] = 1.0;

        // Compute first derivative only (k=1)
        let mut d = 0.0;
        let rk = r as isize - 1;
        let pk = p as isize - 1;

        if r >= 1 {
            a[s2][0] = a[s1][0] / ndu[p][r - 1];
            d = a[s2][0] * ndu[r - 1][p - 1];
        }

        let j1 = if rk >= 0 { 1 } else { (-rk) as usize };
        let j2 = if (r as isize - 1) <= pk {
            p - r
        } else {
            (p as isize - rk) as usize
        };

        for j in j1..=j2 {
            a[s2][j] = (a[s1][j] - a[s1][j - 1]) / ndu[p][r + j - 1];
            d += a[s2][j] * ndu[r + j - 1][p - 1];
        }

        dn_vals[r] = d;

        // Swap rows
        std::mem::swap(&mut s1, &mut s2);
    }

    // Multiply through by the correct factors
    let factor = p as f64;
    for val in &mut dn_vals {
        *val *= factor;
    }

    (n_vals, dn_vals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_span_uniform() {
        // Degree 2, 5 control points, uniform knot vector
        let knots = vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0];
        let n = 4; // 5 control points - 1
        let degree = 2;

        assert_eq!(find_span(degree, &knots, n, 0.0), 2);
        assert_eq!(find_span(degree, &knots, n, 0.5), 2);
        assert_eq!(find_span(degree, &knots, n, 1.0), 3);
        assert_eq!(find_span(degree, &knots, n, 1.5), 3);
        assert_eq!(find_span(degree, &knots, n, 2.5), 4);
        assert_eq!(find_span(degree, &knots, n, 3.0), 4);
    }

    #[test]
    fn test_basis_functions_partition_of_unity() {
        let knots = vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0];
        let degree = 2;
        let n = 4;

        // Basis functions should sum to 1 (partition of unity)
        for &t in &[0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0] {
            let span = find_span(degree, &knots, n, t);
            let basis = basis_functions(degree, &knots, span, t);
            let sum: f64 = basis.iter().sum();
            assert!(
                (sum - 1.0).abs() < 1e-12,
                "Partition of unity failed at t={}: sum={}",
                t,
                sum
            );
        }
    }

    #[test]
    fn test_basis_functions_non_negative() {
        let knots = vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
        let degree = 3;
        let n = 3;

        for i in 0..=20 {
            let t = i as f64 / 20.0;
            let span = find_span(degree, &knots, n, t);
            let basis = basis_functions(degree, &knots, span, t);
            for (j, &val) in basis.iter().enumerate() {
                assert!(
                    val >= -1e-15,
                    "Negative basis at t={}, j={}: {}",
                    t,
                    j,
                    val
                );
            }
        }
    }
}
