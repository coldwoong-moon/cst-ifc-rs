//! IFC entity type definitions.
//!
//! Phase 1 covers the most common geometry and profile types, targeting
//! 60-70% coverage of typical BIM models.

use cst_math::{DVec2, DVec3};
use cst_math::transform::Transform;

/// IFC geometry representations.
#[derive(Debug, Clone)]
pub enum IfcGeometry {
    /// Extruded area solid - sweep a profile along a direction.
    ExtrudedAreaSolid {
        profile: IfcProfile,
        position: Transform,
        direction: DVec3,
        depth: f64,
    },
    /// Faceted B-rep - a closed shell of planar faces.
    FacetedBrep {
        faces: Vec<Vec<DVec3>>,
    },
    /// Mapped item - an instance of another geometry with a transform.
    MappedItem {
        source: Box<IfcGeometry>,
        transform: Transform,
    },
    /// Boolean clipping result (first operand minus second).
    BooleanClippingResult {
        first: Box<IfcGeometry>,
        second: Box<IfcGeometry>,
    },
}

/// IFC profile (cross-section) definitions.
#[derive(Debug, Clone)]
pub enum IfcProfile {
    /// Rectangle defined by X and Y dimensions (centered at origin).
    RectangleProfile {
        x_dim: f64,
        y_dim: f64,
    },
    /// Circle defined by radius.
    CircleProfile {
        radius: f64,
    },
    /// Arbitrary closed profile from a polyline.
    ArbitraryClosedProfile {
        points: Vec<DVec2>,
    },
}

/// Placement / axis definition used in IFC.
#[derive(Debug, Clone)]
pub struct IfcAxis2Placement3D {
    pub location: DVec3,
    pub axis: DVec3,      // Z-direction (default: 0,0,1)
    pub ref_direction: DVec3, // X-direction (default: 1,0,0)
}

impl Default for IfcAxis2Placement3D {
    fn default() -> Self {
        Self {
            location: DVec3::ZERO,
            axis: DVec3::Z,
            ref_direction: DVec3::X,
        }
    }
}

impl IfcAxis2Placement3D {
    /// Convert this axis placement to a rigid body Transform.
    pub fn to_transform(&self) -> Transform {
        let z = self.axis.normalize_or_zero();
        let x = self.ref_direction.normalize_or_zero();
        let y = z.cross(x).normalize_or_zero();
        // Recompute x to ensure orthogonality
        let x = y.cross(z).normalize_or_zero();

        let mat = cst_math::DMat4::from_cols(
            cst_math::DVec4::new(x.x, x.y, x.z, 0.0),
            cst_math::DVec4::new(y.x, y.y, y.z, 0.0),
            cst_math::DVec4::new(z.x, z.y, z.z, 0.0),
            cst_math::DVec4::new(self.location.x, self.location.y, self.location.z, 1.0),
        );
        Transform::from_mat4(mat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_placement() {
        let p = IfcAxis2Placement3D::default();
        assert_eq!(p.location, DVec3::ZERO);
        assert_eq!(p.axis, DVec3::Z);
        assert_eq!(p.ref_direction, DVec3::X);
    }

    #[test]
    fn test_placement_to_transform_identity() {
        let p = IfcAxis2Placement3D::default();
        let t = p.to_transform();
        let pt = DVec3::new(1.0, 2.0, 3.0);
        let result = t.transform_point(pt);
        assert!((result - pt).length() < 1e-10);
    }

    #[test]
    fn test_placement_to_transform_translated() {
        let p = IfcAxis2Placement3D {
            location: DVec3::new(10.0, 20.0, 30.0),
            ..Default::default()
        };
        let t = p.to_transform();
        let pt = DVec3::ZERO;
        let result = t.transform_point(pt);
        assert!((result - DVec3::new(10.0, 20.0, 30.0)).length() < 1e-10);
    }
}
