pub mod pipeline;
pub mod camera;
pub mod scene;

// Re-export main types
pub use camera::Camera;
pub use pipeline::{GpuVertex, RenderMesh, CameraUniforms, prepare_mesh};
pub use scene::{Scene, SceneMesh};
