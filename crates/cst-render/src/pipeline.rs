use cst_mesh::TriangleMesh;
use cst_math::{Point2, Point3, Vector3};

/// Vertex with f32 data packed for GPU.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl GpuVertex {
    /// Create a GPU vertex from mesh vertex data.
    pub fn from_mesh_vertex(pos: Point3, normal: Vector3, uv: Point2) -> Self {
        Self {
            position: [pos.x as f32, pos.y as f32, pos.z as f32],
            normal: [normal.x as f32, normal.y as f32, normal.z as f32],
            uv: [uv.x as f32, uv.y as f32],
        }
    }

    /// Convert vertex array to raw bytes for GPU upload.
    pub fn as_bytes(vertices: &[GpuVertex]) -> Vec<u8> {
        let size = std::mem::size_of::<GpuVertex>() * vertices.len();
        let mut bytes = Vec::with_capacity(size);
        unsafe {
            let ptr = vertices.as_ptr() as *const u8;
            bytes.extend_from_slice(std::slice::from_raw_parts(ptr, size));
        }
        bytes
    }
}

/// Prepared render data ready for GPU upload.
#[derive(Debug, Clone)]
pub struct RenderMesh {
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u32>,
    pub vertex_buffer_bytes: Vec<u8>,
    pub index_buffer_bytes: Vec<u8>,
}

/// Convert a TriangleMesh to GPU-ready buffers.
pub fn prepare_mesh(mesh: &TriangleMesh) -> RenderMesh {
    let vertex_count = mesh.positions.len();
    let mut vertices = Vec::with_capacity(vertex_count);

    // Convert each vertex to GPU format
    for i in 0..vertex_count {
        let pos = mesh.positions[i];
        let normal = mesh.normals.get(i).copied().unwrap_or(Vector3::Y);
        let uv = mesh.uvs.get(i).copied().unwrap_or(Point2::ZERO);

        vertices.push(GpuVertex::from_mesh_vertex(pos, normal, uv));
    }

    // Convert to byte buffers
    let vertex_buffer_bytes = GpuVertex::as_bytes(&vertices);
    let index_buffer_bytes = indices_to_bytes(&mesh.indices);

    RenderMesh {
        vertices,
        indices: mesh.indices.clone(),
        vertex_buffer_bytes,
        index_buffer_bytes,
    }
}

/// Convert index array to raw bytes.
fn indices_to_bytes(indices: &[u32]) -> Vec<u8> {
    let size = std::mem::size_of::<u32>() * indices.len();
    let mut bytes = Vec::with_capacity(size);
    unsafe {
        let ptr = indices.as_ptr() as *const u8;
        bytes.extend_from_slice(std::slice::from_raw_parts(ptr, size));
    }
    bytes
}

/// Uniform buffer for camera matrices.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CameraUniforms {
    pub view: [[f32; 4]; 4],
    pub projection: [[f32; 4]; 4],
    pub view_projection: [[f32; 4]; 4],
    pub eye_position: [f32; 4],
}

impl CameraUniforms {
    /// Create camera uniforms from a Camera.
    pub fn from_camera(camera: &crate::camera::Camera) -> Self {
        let view = convert_matrix_to_f32(camera.view_matrix());
        let projection = convert_matrix_to_f32(camera.projection_matrix());
        let view_projection = convert_matrix_to_f32(camera.view_projection());
        let eye_position = [
            camera.eye.x as f32,
            camera.eye.y as f32,
            camera.eye.z as f32,
            1.0,
        ];

        Self {
            view,
            projection,
            view_projection,
            eye_position,
        }
    }
}

/// Convert f64 matrix to f32 matrix.
fn convert_matrix_to_f32(mat: [[f64; 4]; 4]) -> [[f32; 4]; 4] {
    [
        [mat[0][0] as f32, mat[0][1] as f32, mat[0][2] as f32, mat[0][3] as f32],
        [mat[1][0] as f32, mat[1][1] as f32, mat[1][2] as f32, mat[1][3] as f32],
        [mat[2][0] as f32, mat[2][1] as f32, mat[2][2] as f32, mat[2][3] as f32],
        [mat[3][0] as f32, mat[3][1] as f32, mat[3][2] as f32, mat[3][3] as f32],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mesh() -> TriangleMesh {
        TriangleMesh {
            positions: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![
                Vector3::new(0.0, 0.0, 1.0),
                Vector3::new(0.0, 0.0, 1.0),
                Vector3::new(0.0, 0.0, 1.0),
            ],
            indices: vec![0, 1, 2],
            uvs: vec![
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(0.0, 1.0),
            ],
        }
    }

    #[test]
    fn test_gpu_vertex_size() {
        // 3 floats (position) + 3 floats (normal) + 2 floats (uv) = 8 floats = 32 bytes
        assert_eq!(std::mem::size_of::<GpuVertex>(), 32);
    }

    #[test]
    fn test_prepare_mesh_vertex_count() {
        let mesh = create_test_mesh();
        let render_mesh = prepare_mesh(&mesh);
        assert_eq!(render_mesh.vertices.len(), 3);
    }

    #[test]
    fn test_prepare_mesh_index_count() {
        let mesh = create_test_mesh();
        let render_mesh = prepare_mesh(&mesh);
        assert_eq!(render_mesh.indices.len(), 3);
    }

    #[test]
    fn test_buffer_byte_sizes() {
        let mesh = create_test_mesh();
        let render_mesh = prepare_mesh(&mesh);

        // 3 vertices * 32 bytes each
        assert_eq!(render_mesh.vertex_buffer_bytes.len(), 3 * 32);

        // 3 indices * 4 bytes each
        assert_eq!(render_mesh.index_buffer_bytes.len(), 3 * 4);
    }

    #[test]
    fn test_camera_uniforms_from_camera() {
        let camera = crate::camera::Camera::default();
        let uniforms = CameraUniforms::from_camera(&camera);

        // Check eye position is converted correctly
        assert!((uniforms.eye_position[0] - 0.0).abs() < 1e-6);
        assert!((uniforms.eye_position[1] - 0.0).abs() < 1e-6);
        assert!((uniforms.eye_position[2] - 5.0).abs() < 1e-6);
        assert!((uniforms.eye_position[3] - 1.0).abs() < 1e-6);

        // Check matrices are non-zero
        let view_sum: f32 = uniforms.view.iter().flat_map(|row| row.iter()).sum();
        let proj_sum: f32 = uniforms.projection.iter().flat_map(|row| row.iter()).sum();
        assert!(view_sum.abs() > 0.1);
        assert!(proj_sum.abs() > 0.1);
    }

    #[test]
    fn test_gpu_vertex_from_mesh_vertex() {
        let pos = Point3::new(1.0, 2.0, 3.0);
        let normal = Vector3::new(0.0, 1.0, 0.0);
        let uv = Point2::new(0.5, 0.5);

        let vertex = GpuVertex::from_mesh_vertex(pos, normal, uv);

        assert_eq!(vertex.position, [1.0, 2.0, 3.0]);
        assert_eq!(vertex.normal, [0.0, 1.0, 0.0]);
        assert_eq!(vertex.uv, [0.5, 0.5]);
    }

    #[test]
    fn test_mesh_with_missing_data() {
        // Mesh with positions but no normals or UVs
        let mesh = TriangleMesh {
            positions: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
            ],
            normals: vec![],
            indices: vec![0, 1],
            uvs: vec![],
        };

        let render_mesh = prepare_mesh(&mesh);

        // Should use default values
        assert_eq!(render_mesh.vertices.len(), 2);
        assert_eq!(render_mesh.vertices[0].normal, [0.0, 1.0, 0.0]); // Default Y up
        assert_eq!(render_mesh.vertices[0].uv, [0.0, 0.0]); // Default zero
    }
}
