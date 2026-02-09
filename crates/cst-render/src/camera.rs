use cst_math::{Aabb3, Point3, Vector3, DVec3};

/// A 3D perspective camera with look-at controls.
#[derive(Debug, Clone)]
pub struct Camera {
    pub eye: Point3,       // camera position
    pub target: Point3,    // look-at target
    pub up: Vector3,       // up vector
    pub fov_y: f64,        // vertical FOV in radians
    pub aspect: f64,       // width/height
    pub near: f64,         // near clip plane
    pub far: f64,          // far clip plane
}

impl Camera {
    /// Create a new camera with explicit parameters.
    pub fn new(
        eye: Point3,
        target: Point3,
        up: Vector3,
        fov_y: f64,
        aspect: f64,
        near: f64,
        far: f64,
    ) -> Self {
        Self {
            eye,
            target,
            up,
            fov_y,
            aspect,
            near,
            far,
        }
    }

    /// Create a camera with sensible defaults.
    /// Eye at (0, 0, 5), looking at origin, 45° FOV, 16:9 aspect.
    pub fn default() -> Self {
        Self {
            eye: Point3::new(0.0, 0.0, 5.0),
            target: Point3::ZERO,
            up: Vector3::Y,
            fov_y: std::f64::consts::FRAC_PI_4, // 45 degrees
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 100.0,
        }
    }

    /// Compute the view matrix (look-at matrix) in row-major format.
    pub fn view_matrix(&self) -> [[f64; 4]; 4] {
        let forward = (self.target - self.eye).normalize();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward);

        // View matrix transforms world space to camera space
        // Camera looks down -Z in view space
        let f = -forward; // Negate to look down -Z

        // Create rotation part
        let mut mat = [[0.0; 4]; 4];
        mat[0] = [right.x, right.y, right.z, 0.0];
        mat[1] = [up.x, up.y, up.z, 0.0];
        mat[2] = [f.x, f.y, f.z, 0.0];
        mat[3] = [0.0, 0.0, 0.0, 1.0];

        // Apply translation
        mat[0][3] = -right.dot(self.eye);
        mat[1][3] = -up.dot(self.eye);
        mat[2][3] = -f.dot(self.eye);

        mat
    }

    /// Compute the perspective projection matrix in row-major format.
    /// Uses OpenGL-style NDC (-1 to 1 for Z).
    pub fn projection_matrix(&self) -> [[f64; 4]; 4] {
        let tan_half_fov = (self.fov_y / 2.0).tan();
        let f = 1.0 / tan_half_fov;

        let mut mat = [[0.0; 4]; 4];
        mat[0][0] = f / self.aspect;
        mat[1][1] = f;
        mat[2][2] = (self.far + self.near) / (self.near - self.far);
        mat[2][3] = (2.0 * self.far * self.near) / (self.near - self.far);
        mat[3][2] = -1.0;

        mat
    }

    /// Compute combined view-projection matrix.
    pub fn view_projection(&self) -> [[f64; 4]; 4] {
        let view = self.view_matrix();
        let proj = self.projection_matrix();
        multiply_matrices(&proj, &view)
    }

    /// Orbit the camera around the target.
    /// delta_x and delta_y are in radians.
    pub fn orbit(&mut self, delta_x: f64, delta_y: f64) {
        let offset = self.eye - self.target;
        let radius = offset.length();

        // Convert to spherical coordinates
        let theta = offset.z.atan2(offset.x); // azimuth
        let phi = (offset.y / radius).acos(); // polar angle

        // Apply deltas
        let new_theta = theta + delta_x;
        let new_phi = (phi + delta_y).clamp(0.01, std::f64::consts::PI - 0.01);

        // Convert back to Cartesian
        let new_offset = DVec3::new(
            radius * new_phi.sin() * new_theta.cos(),
            radius * new_phi.cos(),
            radius * new_phi.sin() * new_theta.sin(),
        );

        self.eye = self.target + new_offset;
    }

    /// Zoom by moving the camera closer or farther from the target.
    /// Positive delta moves closer, negative moves farther.
    pub fn zoom(&mut self, delta: f64) {
        let direction = (self.target - self.eye).normalize();
        let new_eye = self.eye + direction * delta;

        // Prevent camera from crossing or getting too close to target
        let new_distance = (self.target - new_eye).length();
        if new_distance > 0.1 {
            self.eye = new_eye;
        }
    }

    /// Pan the camera and target together in the view plane.
    pub fn pan(&mut self, dx: f64, dy: f64) {
        let forward = (self.target - self.eye).normalize();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward);

        let offset = right * dx + up * dy;
        self.eye += offset;
        self.target += offset;
    }

    /// Adjust camera to fit an AABB in view.
    /// Positions camera to see entire bounding box.
    pub fn fit_to_aabb(&mut self, aabb: &Aabb3) {
        let center = aabb.center();
        let size = aabb.extents();
        let max_dim = size.x.max(size.y).max(size.z);

        // Calculate distance needed to fit the object
        let distance = max_dim / (2.0 * (self.fov_y / 2.0).tan());

        // Position camera along view direction
        let view_dir = (self.target - self.eye).normalize();
        self.target = center;
        self.eye = center - view_dir * distance * 1.5; // 1.5x for padding
    }
}

/// Multiply two 4x4 matrices (row-major).
fn multiply_matrices(a: &[[f64; 4]; 4], b: &[[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut result = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            result[i][j] = a[i][0] * b[0][j]
                + a[i][1] * b[1][j]
                + a[i][2] * b[2][j]
                + a[i][3] * b[3][j];
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_camera() {
        let cam = Camera::default();
        assert_eq!(cam.eye, Point3::new(0.0, 0.0, 5.0));
        assert_eq!(cam.target, Point3::ZERO);
        assert_eq!(cam.up, Vector3::Y);
    }

    #[test]
    fn test_view_matrix() {
        let cam = Camera::default();
        let view = cam.view_matrix();

        // Check it's a valid transformation matrix
        // Last row should be [0, 0, 0, 1] for affine transform
        assert!((view[3][0]).abs() < 1e-10);
        assert!((view[3][1]).abs() < 1e-10);
        assert!((view[3][2]).abs() < 1e-10);
        assert!((view[3][3] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_projection_matrix() {
        let cam = Camera::default();
        let proj = cam.projection_matrix();

        // Basic validity checks
        assert!(proj[0][0] > 0.0); // aspect-corrected focal length
        assert!(proj[1][1] > 0.0); // vertical focal length
        assert!(proj[2][2] < 0.0); // depth mapping (negative for OpenGL)
        assert!((proj[3][2] + 1.0).abs() < 1e-10); // perspective divide
    }

    #[test]
    fn test_view_projection() {
        let cam = Camera::default();
        let vp = cam.view_projection();

        // Should be non-zero
        let sum: f64 = vp.iter().flat_map(|row| row.iter()).sum();
        assert!(sum.abs() > 0.1);
    }

    #[test]
    fn test_orbit() {
        let mut cam = Camera::default();
        let original_distance = (cam.eye - cam.target).length();

        cam.orbit(0.1, 0.0);

        let new_distance = (cam.eye - cam.target).length();
        // Distance should remain approximately the same
        assert!((original_distance - new_distance).abs() < 1e-10);
        // Eye position should have changed
        assert!((cam.eye - Point3::new(0.0, 0.0, 5.0)).length() > 0.1);
    }

    #[test]
    fn test_zoom() {
        let mut cam = Camera::default();
        let original_distance = (cam.eye - cam.target).length();

        cam.zoom(1.0); // Move 1 unit closer

        let new_distance = (cam.eye - cam.target).length();
        assert!(new_distance < original_distance);
        assert!((new_distance - (original_distance - 1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_pan() {
        let mut cam = Camera::default();
        let original_eye = cam.eye;
        let original_target = cam.target;

        cam.pan(1.0, 0.5);

        // Both eye and target should move
        assert!((cam.eye - original_eye).length() > 0.1);
        assert!((cam.target - original_target).length() > 0.1);

        // Distance between them should remain constant
        let original_dist = (original_eye - original_target).length();
        let new_dist = (cam.eye - cam.target).length();
        assert!((original_dist - new_dist).abs() < 1e-10);
    }

    #[test]
    fn test_fit_to_aabb() {
        let mut cam = Camera::default();
        let aabb = Aabb3::new(
            Point3::new(-2.0, -2.0, -2.0),
            Point3::new(2.0, 2.0, 2.0),
        );

        cam.fit_to_aabb(&aabb);

        // Target should be at center of AABB
        assert_eq!(cam.target, Point3::ZERO);

        // Camera should be positioned away from center
        let distance = (cam.eye - cam.target).length();
        assert!(distance > 4.0); // Should be farther than box radius
    }
}
