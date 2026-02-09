use crate::{DMat4, Point3, Vector3};
use serde::{Deserialize, Serialize};

/// Rigid body transform (rotation + translation, no shear/scale).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
    pub matrix: [f64; 16],
}

impl Transform {
    pub fn identity() -> Self {
        Self::from_mat4(DMat4::IDENTITY)
    }

    pub fn from_translation(t: Vector3) -> Self {
        Self::from_mat4(DMat4::from_translation(t))
    }

    pub fn from_mat4(m: DMat4) -> Self {
        Self {
            matrix: m.to_cols_array(),
        }
    }

    pub fn to_mat4(&self) -> DMat4 {
        DMat4::from_cols_array(&self.matrix)
    }

    pub fn transform_point(&self, p: Point3) -> Point3 {
        self.to_mat4().transform_point3(p)
    }

    pub fn transform_vector(&self, v: Vector3) -> Vector3 {
        self.to_mat4().transform_vector3(v)
    }

    pub fn then(&self, other: &Transform) -> Transform {
        Self::from_mat4(other.to_mat4() * self.to_mat4())
    }

    pub fn inverse(&self) -> Option<Transform> {
        let m = self.to_mat4();
        let inv = m.inverse();
        // Check if inverse is valid (determinant != 0)
        if m.determinant().abs() < 1e-15 {
            None
        } else {
            Some(Self::from_mat4(inv))
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::dvec3;

    #[test]
    fn test_identity() {
        let t = Transform::identity();
        let p = dvec3(1.0, 2.0, 3.0);
        let result = t.transform_point(p);
        assert!((result - p).length() < 1e-10);
    }

    #[test]
    fn test_translation() {
        let t = Transform::from_translation(dvec3(10.0, 20.0, 30.0));
        let p = dvec3(1.0, 2.0, 3.0);
        let result = t.transform_point(p);
        assert!((result - dvec3(11.0, 22.0, 33.0)).length() < 1e-10);
    }

    #[test]
    fn test_inverse() {
        let t = Transform::from_translation(dvec3(10.0, 20.0, 30.0));
        let inv = t.inverse().unwrap();
        let p = dvec3(1.0, 2.0, 3.0);
        let result = inv.transform_point(t.transform_point(p));
        assert!((result - p).length() < 1e-10);
    }
}
