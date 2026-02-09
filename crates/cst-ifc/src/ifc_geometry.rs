//! IFC geometry resolution - converts IFC geometry descriptions to point data.

use cst_math::DVec3;

use crate::ifc_entities::{IfcGeometry, IfcProfile};
use cst_core::Result;

/// Generate 2D profile points (in the XY plane, Z=0).
pub fn profile_points(profile: &IfcProfile) -> Vec<DVec3> {
    match profile {
        IfcProfile::RectangleProfile { x_dim, y_dim } => {
            let hx = x_dim / 2.0;
            let hy = y_dim / 2.0;
            vec![
                DVec3::new(-hx, -hy, 0.0),
                DVec3::new(hx, -hy, 0.0),
                DVec3::new(hx, hy, 0.0),
                DVec3::new(-hx, hy, 0.0),
            ]
        }
        IfcProfile::CircleProfile { radius } => {
            // Approximate circle with 32 segments
            let n = 32;
            (0..n)
                .map(|i| {
                    let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
                    DVec3::new(radius * angle.cos(), radius * angle.sin(), 0.0)
                })
                .collect()
        }
        IfcProfile::ArbitraryClosedProfile { points } => {
            points
                .iter()
                .map(|p| DVec3::new(p.x, p.y, 0.0))
                .collect()
        }
    }
}

/// Extrude a profile along a direction by the given depth.
///
/// Returns the vertices of the extruded solid: bottom face followed by top face.
pub fn extrude_profile(profile: &IfcProfile, direction: DVec3, depth: f64) -> Vec<DVec3> {
    let base = profile_points(profile);
    let offset = direction.normalize_or_zero() * depth;

    let mut points = Vec::with_capacity(base.len() * 2);
    // Bottom face
    for p in &base {
        points.push(*p);
    }
    // Top face
    for p in &base {
        points.push(*p + offset);
    }
    points
}

/// Resolve an IFC geometry description into a set of points.
///
/// This is a simplified resolution that produces representative vertices,
/// not full triangulated meshes.
pub fn resolve_geometry(geom: &IfcGeometry) -> Result<Vec<DVec3>> {
    match geom {
        IfcGeometry::ExtrudedAreaSolid {
            profile,
            position,
            direction,
            depth,
        } => {
            let local_pts = extrude_profile(profile, *direction, *depth);
            let world_pts = local_pts
                .into_iter()
                .map(|p| position.transform_point(p))
                .collect();
            Ok(world_pts)
        }
        IfcGeometry::FacetedBrep { faces } => {
            let pts: Vec<DVec3> = faces.iter().flat_map(|f| f.iter().copied()).collect();
            Ok(pts)
        }
        IfcGeometry::MappedItem { source, transform } => {
            let source_pts = resolve_geometry(source)?;
            let transformed = source_pts
                .into_iter()
                .map(|p| transform.transform_point(p))
                .collect();
            Ok(transformed)
        }
        IfcGeometry::BooleanClippingResult { first, .. } => {
            // Simplified: just return the first operand's geometry.
            // Full boolean operations require CSG, which is in cst-topology.
            resolve_geometry(first)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ifc_entities::IfcProfile;

    #[test]
    fn test_rectangle_profile_points() {
        let profile = IfcProfile::RectangleProfile {
            x_dim: 4.0,
            y_dim: 2.0,
        };
        let pts = profile_points(&profile);
        assert_eq!(pts.len(), 4);
        // Check corners
        assert!((pts[0] - DVec3::new(-2.0, -1.0, 0.0)).length() < 1e-10);
        assert!((pts[1] - DVec3::new(2.0, -1.0, 0.0)).length() < 1e-10);
        assert!((pts[2] - DVec3::new(2.0, 1.0, 0.0)).length() < 1e-10);
        assert!((pts[3] - DVec3::new(-2.0, 1.0, 0.0)).length() < 1e-10);
    }

    #[test]
    fn test_circle_profile_points() {
        let profile = IfcProfile::CircleProfile { radius: 5.0 };
        let pts = profile_points(&profile);
        assert_eq!(pts.len(), 32);
        // All points should be at distance 5 from origin
        for p in &pts {
            let dist = (p.x * p.x + p.y * p.y).sqrt();
            assert!((dist - 5.0).abs() < 1e-10);
            assert!(p.z.abs() < 1e-10);
        }
    }

    #[test]
    fn test_extrude_rectangle() {
        let profile = IfcProfile::RectangleProfile {
            x_dim: 2.0,
            y_dim: 2.0,
        };
        let pts = extrude_profile(&profile, DVec3::Z, 10.0);
        assert_eq!(pts.len(), 8); // 4 bottom + 4 top

        // Bottom face at z=0
        for p in &pts[..4] {
            assert!(p.z.abs() < 1e-10);
        }
        // Top face at z=10
        for p in &pts[4..] {
            assert!((p.z - 10.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_resolve_extruded_solid() {
        use cst_math::transform::Transform;

        let geom = IfcGeometry::ExtrudedAreaSolid {
            profile: IfcProfile::RectangleProfile {
                x_dim: 1.0,
                y_dim: 1.0,
            },
            position: Transform::identity(),
            direction: DVec3::Z,
            depth: 5.0,
        };
        let pts = resolve_geometry(&geom).unwrap();
        assert_eq!(pts.len(), 8);
    }

    #[test]
    fn test_resolve_faceted_brep() {
        let geom = IfcGeometry::FacetedBrep {
            faces: vec![
                vec![
                    DVec3::new(0.0, 0.0, 0.0),
                    DVec3::new(1.0, 0.0, 0.0),
                    DVec3::new(1.0, 1.0, 0.0),
                ],
                vec![
                    DVec3::new(0.0, 0.0, 1.0),
                    DVec3::new(1.0, 0.0, 1.0),
                    DVec3::new(1.0, 1.0, 1.0),
                ],
            ],
        };
        let pts = resolve_geometry(&geom).unwrap();
        assert_eq!(pts.len(), 6);
    }
}
