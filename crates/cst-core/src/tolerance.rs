/// Global and local tolerance management for geometric computations.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Tolerance {
    /// Linear tolerance for distance comparisons (in model units)
    pub linear: f64,
    /// Angular tolerance (in radians)
    pub angular: f64,
}

impl Tolerance {
    pub const DEFAULT_LINEAR: f64 = 1e-7;
    pub const DEFAULT_ANGULAR: f64 = 1e-10;

    pub fn new(linear: f64, angular: f64) -> Self {
        Self { linear, angular }
    }

    pub fn default_precision() -> Self {
        Self {
            linear: Self::DEFAULT_LINEAR,
            angular: Self::DEFAULT_ANGULAR,
        }
    }

    pub fn loose() -> Self {
        Self {
            linear: 1e-4,
            angular: 1e-6,
        }
    }

    pub fn tight() -> Self {
        Self {
            linear: 1e-10,
            angular: 1e-12,
        }
    }

    /// Check if two values are equal within linear tolerance
    pub fn linear_eq(self, a: f64, b: f64) -> bool {
        (a - b).abs() < self.linear
    }

    /// Check if a value is zero within linear tolerance
    pub fn is_zero(self, v: f64) -> bool {
        v.abs() < self.linear
    }

    /// Check if two angles are equal within angular tolerance
    pub fn angular_eq(self, a: f64, b: f64) -> bool {
        (a - b).abs() < self.angular
    }
}

impl Default for Tolerance {
    fn default() -> Self {
        Self::default_precision()
    }
}
