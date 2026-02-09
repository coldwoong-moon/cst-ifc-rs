use cst_mesh::TriangleMesh;
use cst_math::Aabb3;
use std::path::Path;
use std::io::Write;

/// A named mesh in the scene
#[derive(Clone)]
pub struct SceneMesh {
    pub name: String,
    pub mesh: TriangleMesh,
    pub color: [f32; 3],
}

/// An instanced mesh group - one base geometry with multiple transform matrices
#[derive(Clone)]
pub struct InstancedGroup {
    pub name: String,
    pub mesh: TriangleMesh,
    pub color: [f32; 3],
    /// Each transform is a 4x4 matrix stored as [f32; 16] in column-major order
    pub transforms: Vec<[f32; 16]>,
}

/// A 3D scene for visualization
pub struct Scene {
    pub meshes: Vec<SceneMesh>,
    pub instanced_groups: Vec<InstancedGroup>,
}

impl Scene {
    /// Create a new empty scene
    pub fn new() -> Self {
        Self {
            meshes: Vec::new(),
            instanced_groups: Vec::new(),
        }
    }

    /// Add a mesh with a name and color
    pub fn add_mesh(&mut self, name: &str, mesh: TriangleMesh, color: [f32; 3]) {
        self.meshes.push(SceneMesh {
            name: name.to_string(),
            mesh,
            color,
        });
    }

    /// Add a mesh with auto-assigned color
    pub fn add_mesh_auto_color(&mut self, name: &str, mesh: TriangleMesh) {
        const PALETTE: [[f32; 3]; 10] = [
            [0.7, 0.8, 0.9],  // Light blue
            [0.9, 0.7, 0.7],  // Light red
            [0.7, 0.9, 0.7],  // Light green
            [0.9, 0.9, 0.7],  // Yellow
            [0.9, 0.7, 0.9],  // Pink
            [0.7, 0.9, 0.9],  // Cyan
            [0.8, 0.8, 0.8],  // Gray
            [0.9, 0.8, 0.7],  // Orange
            [0.8, 0.7, 0.9],  // Purple
            [0.7, 0.9, 0.8],  // Teal
        ];

        let color = PALETTE[self.meshes.len() % PALETTE.len()];
        self.add_mesh(name, mesh, color);
    }

    /// Add an instanced group (one base geometry with multiple placements)
    pub fn add_instanced_group(&mut self, name: &str, mesh: TriangleMesh, color: [f32; 3], transforms: Vec<[f32; 16]>) {
        self.instanced_groups.push(InstancedGroup {
            name: name.to_string(),
            mesh,
            color,
            transforms,
        });
    }

    /// Compute scene bounding box
    pub fn bounds(&self) -> Option<Aabb3> {
        if self.meshes.is_empty() && self.instanced_groups.is_empty() {
            return None;
        }

        let mut all_points = Vec::new();
        for scene_mesh in &self.meshes {
            all_points.extend_from_slice(&scene_mesh.mesh.positions);
        }
        for ig in &self.instanced_groups {
            for p in &ig.mesh.positions {
                all_points.push(*p);
            }
        }
        Aabb3::from_points(&all_points)
    }

    /// Total triangle count across all meshes
    pub fn total_triangles(&self) -> usize {
        self.meshes.iter().map(|m| m.mesh.indices.len() / 3).sum()
    }

    /// Export scene as a standalone HTML file with embedded Three.js viewer
    pub fn export_html(&self, path: &Path) -> std::io::Result<()> {
        let bounds = self.bounds().unwrap_or_else(|| {
            use cst_math::{Point3, DVec3};
            Aabb3::new(Point3::ZERO, DVec3::splat(1.0))
        });
        let center = bounds.center();
        let size = bounds.extents();
        let camera_distance = size.length() * 1.5;

        let mut file = std::fs::File::create(path)?;

        write!(file, r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CSTEngine Scene Viewer</title>
    <style>
        body {{
            margin: 0;
            overflow: hidden;
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: #1a1a1a;
        }}
        #container {{
            width: 100vw;
            height: 100vh;
        }}
        #info {{
            position: absolute;
            top: 10px;
            left: 10px;
            background: rgba(0, 0, 0, 0.7);
            color: white;
            padding: 15px;
            border-radius: 5px;
            font-size: 14px;
            max-width: 300px;
            max-height: calc(100vh - 40px);
            overflow-y: auto;
        }}
        #info h3 {{
            margin: 0 0 10px 0;
            font-size: 16px;
            border-bottom: 1px solid #666;
            padding-bottom: 5px;
        }}
        #info .mesh-item {{
            margin: 5px 0;
            padding: 5px;
            background: rgba(255, 255, 255, 0.1);
            border-radius: 3px;
        }}
        #info .mesh-name {{
            font-weight: bold;
            color: #4fc3f7;
        }}
        #info .mesh-stats {{
            font-size: 12px;
            color: #aaa;
        }}
        #error {{
            position: absolute;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            background: rgba(200, 0, 0, 0.9);
            color: white;
            padding: 20px;
            border-radius: 5px;
            display: none;
        }}
    </style>
</head>
<body>
    <div id="container"></div>
    <div id="info">
        <h3>CSTEngine Scene</h3>
        <div>Meshes: {}</div>
        <div>Triangles: {}</div>
        <hr style="border: 1px solid #666; margin: 10px 0;">
"#, self.meshes.len(), self.total_triangles())?;

        // Write mesh list
        for scene_mesh in &self.meshes {
            let tri_count = scene_mesh.mesh.indices.len() / 3;
            write!(file, r#"        <div class="mesh-item">
            <div class="mesh-name">{}</div>
            <div class="mesh-stats">{} triangles</div>
        </div>
"#, scene_mesh.name, tri_count)?;
        }

        write!(file, r#"    </div>
    <div id="error">Failed to load Three.js from CDN. Please check your internet connection.</div>

    <script src="https://cdnjs.cloudflare.com/ajax/libs/three.js/r128/three.min.js"></script>
    <script>
        if (typeof THREE === 'undefined') document.getElementById('error').style.display='block';
"#)?;

        // Embed mesh data
        write!(file, "        const meshData = [\n")?;
        for (i, scene_mesh) in self.meshes.iter().enumerate() {
            write!(file, "            {{\n")?;
            write!(file, "                name: \"{}\",\n", scene_mesh.name)?;
            write!(file, "                color: [{}, {}, {}],\n",
                scene_mesh.color[0], scene_mesh.color[1], scene_mesh.color[2])?;

            // Write positions (convert to f32 and truncate to 2 decimals)
            write!(file, "                positions: [")?;
            for (j, pos) in scene_mesh.mesh.positions.iter().enumerate() {
                if j > 0 { write!(file, ",")?; }
                write!(file, "{:.2},{:.2},{:.2}", pos.x as f32, pos.y as f32, pos.z as f32)?;
            }
            write!(file, "],\n")?;

            // Write normals
            write!(file, "                normals: [")?;
            for (j, norm) in scene_mesh.mesh.normals.iter().enumerate() {
                if j > 0 { write!(file, ",")?; }
                write!(file, "{:.2},{:.2},{:.2}", norm.x as f32, norm.y as f32, norm.z as f32)?;
            }
            write!(file, "],\n")?;

            // Write indices
            write!(file, "                indices: [")?;
            for (j, idx) in scene_mesh.mesh.indices.iter().enumerate() {
                if j > 0 { write!(file, ",")?; }
                write!(file, "{}", idx)?;
            }
            write!(file, "]\n")?;

            write!(file, "            }}")?;
            if i < self.meshes.len() - 1 {
                write!(file, ",")?;
            }
            write!(file, "\n")?;
        }
        write!(file, "        ];\n\n")?;

        // Three.js scene setup
        write!(file, r#"        function initScene() {{
            const scene = new THREE.Scene();
            scene.background = new THREE.Color(0x1a1a1a);

            const camera = new THREE.PerspectiveCamera(
                60,
                window.innerWidth / window.innerHeight,
                0.1,
                10000
            );

            const renderer = new THREE.WebGLRenderer({{ antialias: true }});
            renderer.setSize(window.innerWidth, window.innerHeight);
            document.getElementById('container').appendChild(renderer.domElement);

            // Add lighting
            const ambientLight = new THREE.AmbientLight(0x404040, 2);
            scene.add(ambientLight);

            const dirLight1 = new THREE.DirectionalLight(0xffffff, 1);
            dirLight1.position.set(1, 1, 1);
            scene.add(dirLight1);

            const dirLight2 = new THREE.DirectionalLight(0xffffff, 0.5);
            dirLight2.position.set(-1, -1, -1);
            scene.add(dirLight2);

            // Add meshes
            meshData.forEach(data => {{
                const geometry = new THREE.BufferGeometry();
                geometry.setAttribute('position', new THREE.Float32BufferAttribute(data.positions, 3));
                geometry.setAttribute('normal', new THREE.Float32BufferAttribute(data.normals, 3));
                geometry.setIndex(data.indices);

                const material = new THREE.MeshPhongMaterial({{
                    color: new THREE.Color(data.color[0], data.color[1], data.color[2]),
                    shininess: 30,
                    side: THREE.DoubleSide
                }});

                const mesh = new THREE.Mesh(geometry, material);
                scene.add(mesh);
            }});

            // Add grid and axes
            const gridSize = {:.2};
            const grid = new THREE.GridHelper(gridSize * 2, 20, 0x444444, 0x222222);
            grid.position.y = {:.2};
            scene.add(grid);

            const axes = new THREE.AxesHelper(gridSize * 0.5);
            scene.add(axes);

            // Position camera
            const center = new THREE.Vector3({:.2}, {:.2}, {:.2});
            const distance = {:.2};
            camera.position.set(
                center.x + distance * 0.7,
                center.y + distance * 0.7,
                center.z + distance * 0.7
            );
            camera.lookAt(center);

            // Simple orbit controls (mouse drag)
            let isDragging = false;
            let previousMousePosition = {{ x: 0, y: 0 }};
            let theta = Math.PI / 4;
            let phi = Math.PI / 4;
            let radius = distance;

            renderer.domElement.addEventListener('mousedown', (e) => {{
                isDragging = true;
                previousMousePosition = {{ x: e.clientX, y: e.clientY }};
            }});

            renderer.domElement.addEventListener('mousemove', (e) => {{
                if (isDragging) {{
                    const deltaX = e.clientX - previousMousePosition.x;
                    const deltaY = e.clientY - previousMousePosition.y;

                    theta -= deltaX * 0.01;
                    phi = Math.max(0.1, Math.min(Math.PI - 0.1, phi + deltaY * 0.01));

                    previousMousePosition = {{ x: e.clientX, y: e.clientY }};
                    updateCameraPosition();
                }}
            }});

            renderer.domElement.addEventListener('mouseup', () => {{
                isDragging = false;
            }});

            renderer.domElement.addEventListener('wheel', (e) => {{
                e.preventDefault();
                radius = Math.max(1, radius + e.deltaY * 0.01);
                updateCameraPosition();
            }});

            function updateCameraPosition() {{
                camera.position.x = center.x + radius * Math.sin(phi) * Math.cos(theta);
                camera.position.y = center.y + radius * Math.cos(phi);
                camera.position.z = center.z + radius * Math.sin(phi) * Math.sin(theta);
                camera.lookAt(center);
            }}

            // Handle window resize
            window.addEventListener('resize', () => {{
                camera.aspect = window.innerWidth / window.innerHeight;
                camera.updateProjectionMatrix();
                renderer.setSize(window.innerWidth, window.innerHeight);
            }});

            // Animation loop
            function animate() {{
                requestAnimationFrame(animate);
                renderer.render(scene, camera);
            }}
            animate();
        }}

        if (typeof THREE !== 'undefined') initScene();
    </script>
</body>
</html>
"#,
            size.length().max(10.0),
            bounds.min.y,
            center.x, center.y, center.z,
            camera_distance
        )?;

        Ok(())
    }

    /// Export scene as glTF JSON file
    pub fn export_gltf_json(&self) -> String {
        use std::fmt::Write as FmtWrite;

        let mut json = String::new();

        // Start JSON
        writeln!(json, "{{").unwrap();
        writeln!(json, "  \"asset\": {{").unwrap();
        writeln!(json, "    \"version\": \"2.0\",").unwrap();
        writeln!(json, "    \"generator\": \"CSTEngine\"").unwrap();
        writeln!(json, "  }},").unwrap();

        // Scene
        writeln!(json, "  \"scene\": 0,").unwrap();
        writeln!(json, "  \"scenes\": [{{").unwrap();
        write!(json, "    \"nodes\": [").unwrap();
        for i in 0..self.meshes.len() {
            if i > 0 { write!(json, ", ").unwrap(); }
            write!(json, "{}", i).unwrap();
        }
        writeln!(json, "]").unwrap();
        writeln!(json, "  }}],").unwrap();

        // Nodes
        writeln!(json, "  \"nodes\": [").unwrap();
        for (i, scene_mesh) in self.meshes.iter().enumerate() {
            writeln!(json, "    {{").unwrap();
            writeln!(json, "      \"name\": \"{}\",", scene_mesh.name).unwrap();
            writeln!(json, "      \"mesh\": {}", i).unwrap();
            write!(json, "    }}").unwrap();
            if i < self.meshes.len() - 1 {
                writeln!(json, ",").unwrap();
            } else {
                writeln!(json).unwrap();
            }
        }
        writeln!(json, "  ],").unwrap();

        // Meshes
        writeln!(json, "  \"meshes\": [").unwrap();
        for (i, scene_mesh) in self.meshes.iter().enumerate() {
            writeln!(json, "    {{").unwrap();
            writeln!(json, "      \"name\": \"{}\",", scene_mesh.name).unwrap();
            writeln!(json, "      \"primitives\": [{{").unwrap();
            writeln!(json, "        \"attributes\": {{").unwrap();
            writeln!(json, "          \"POSITION\": {},", i * 3).unwrap();
            writeln!(json, "          \"NORMAL\": {}", i * 3 + 1).unwrap();
            writeln!(json, "        }},").unwrap();
            writeln!(json, "        \"indices\": {},", i * 3 + 2).unwrap();
            writeln!(json, "        \"material\": {}", i).unwrap();
            writeln!(json, "      }}]").unwrap();
            write!(json, "    }}").unwrap();
            if i < self.meshes.len() - 1 {
                writeln!(json, ",").unwrap();
            } else {
                writeln!(json).unwrap();
            }
        }
        writeln!(json, "  ],").unwrap();

        // Materials
        writeln!(json, "  \"materials\": [").unwrap();
        for (i, scene_mesh) in self.meshes.iter().enumerate() {
            writeln!(json, "    {{").unwrap();
            writeln!(json, "      \"name\": \"{}_Material\",", scene_mesh.name).unwrap();
            writeln!(json, "      \"pbrMetallicRoughness\": {{").unwrap();
            writeln!(json, "        \"baseColorFactor\": [{}, {}, {}, 1.0],",
                scene_mesh.color[0], scene_mesh.color[1], scene_mesh.color[2]).unwrap();
            writeln!(json, "        \"metallicFactor\": 0.0,").unwrap();
            writeln!(json, "        \"roughnessFactor\": 0.5").unwrap();
            writeln!(json, "      }},").unwrap();
            writeln!(json, "      \"doubleSided\": true").unwrap();
            write!(json, "    }}").unwrap();
            if i < self.meshes.len() - 1 {
                writeln!(json, ",").unwrap();
            } else {
                writeln!(json).unwrap();
            }
        }
        writeln!(json, "  ],").unwrap();

        // Accessors
        writeln!(json, "  \"accessors\": [").unwrap();
        let mut accessor_idx = 0;
        for scene_mesh in &self.meshes {
            let vertex_count = scene_mesh.mesh.positions.len();
            let index_count = scene_mesh.mesh.indices.len();

            // Position accessor
            let bounds = self.compute_mesh_bounds(scene_mesh);
            writeln!(json, "    {{").unwrap();
            writeln!(json, "      \"bufferView\": {},", accessor_idx).unwrap();
            writeln!(json, "      \"componentType\": 5126,").unwrap();
            writeln!(json, "      \"count\": {},", vertex_count).unwrap();
            writeln!(json, "      \"type\": \"VEC3\",").unwrap();
            writeln!(json, "      \"max\": [{}, {}, {}],", bounds.max.x, bounds.max.y, bounds.max.z).unwrap();
            writeln!(json, "      \"min\": [{}, {}, {}]", bounds.min.x, bounds.min.y, bounds.min.z).unwrap();
            writeln!(json, "    }},").unwrap();

            // Normal accessor
            writeln!(json, "    {{").unwrap();
            writeln!(json, "      \"bufferView\": {},", accessor_idx + 1).unwrap();
            writeln!(json, "      \"componentType\": 5126,").unwrap();
            writeln!(json, "      \"count\": {},", vertex_count).unwrap();
            writeln!(json, "      \"type\": \"VEC3\"").unwrap();
            writeln!(json, "    }},").unwrap();

            // Index accessor
            writeln!(json, "    {{").unwrap();
            writeln!(json, "      \"bufferView\": {},", accessor_idx + 2).unwrap();
            writeln!(json, "      \"componentType\": 5125,").unwrap();
            writeln!(json, "      \"count\": {},", index_count).unwrap();
            writeln!(json, "      \"type\": \"SCALAR\"").unwrap();
            write!(json, "    }}").unwrap();

            accessor_idx += 3;
            if accessor_idx < self.meshes.len() * 3 {
                writeln!(json, ",").unwrap();
            } else {
                writeln!(json).unwrap();
            }
        }
        writeln!(json, "  ],").unwrap();

        // BufferViews
        writeln!(json, "  \"bufferViews\": [").unwrap();
        let mut offset = 0;
        let mut view_idx = 0;
        for scene_mesh in &self.meshes {
            let pos_bytes = scene_mesh.mesh.positions.len() * 12;
            let norm_bytes = scene_mesh.mesh.normals.len() * 12;
            let idx_bytes = scene_mesh.mesh.indices.len() * 4;

            // Position buffer view
            writeln!(json, "    {{").unwrap();
            writeln!(json, "      \"buffer\": 0,").unwrap();
            writeln!(json, "      \"byteOffset\": {},", offset).unwrap();
            writeln!(json, "      \"byteLength\": {},", pos_bytes).unwrap();
            writeln!(json, "      \"target\": 34962").unwrap();
            writeln!(json, "    }},").unwrap();
            offset += pos_bytes;

            // Normal buffer view
            writeln!(json, "    {{").unwrap();
            writeln!(json, "      \"buffer\": 0,").unwrap();
            writeln!(json, "      \"byteOffset\": {},", offset).unwrap();
            writeln!(json, "      \"byteLength\": {},", norm_bytes).unwrap();
            writeln!(json, "      \"target\": 34962").unwrap();
            writeln!(json, "    }},").unwrap();
            offset += norm_bytes;

            // Index buffer view
            writeln!(json, "    {{").unwrap();
            writeln!(json, "      \"buffer\": 0,").unwrap();
            writeln!(json, "      \"byteOffset\": {},", offset).unwrap();
            writeln!(json, "      \"byteLength\": {},", idx_bytes).unwrap();
            writeln!(json, "      \"target\": 34963").unwrap();
            write!(json, "    }}").unwrap();
            offset += idx_bytes;

            view_idx += 3;
            if view_idx < self.meshes.len() * 3 {
                writeln!(json, ",").unwrap();
            } else {
                writeln!(json).unwrap();
            }
        }
        writeln!(json, "  ],").unwrap();

        // Buffer (base64 encoded binary data)
        writeln!(json, "  \"buffers\": [{{").unwrap();
        writeln!(json, "    \"byteLength\": {},", offset).unwrap();
        write!(json, "    \"uri\": \"data:application/octet-stream;base64,").unwrap();

        // Generate binary data and encode to base64
        let binary_data = self.generate_gltf_binary_buffer();
        write!(json, "{}\"", base64_encode(&binary_data)).unwrap();
        writeln!(json).unwrap();
        writeln!(json, "  }}]").unwrap();

        writeln!(json, "}}").unwrap();

        json
    }

    fn compute_mesh_bounds(&self, scene_mesh: &SceneMesh) -> Aabb3 {
        Aabb3::from_points(&scene_mesh.mesh.positions).unwrap_or_else(|| {
            use cst_math::{Point3, DVec3};
            Aabb3::new(Point3::ZERO, DVec3::splat(1.0))
        })
    }

    /// Export scene mesh data as a compact binary file for web streaming.
    ///
    /// Format v3 (instancing): [u8 version=3][u32 regular_mesh_count][u32 instanced_group_count]
    /// Then per regular mesh:
    ///   [u32 name_len][name_utf8][f32 r][f32 g][f32 b]
    ///   [u32 vertex_count][u32 index_count]
    ///   [vertex_count * 3 * f32 positions]
    ///   [index_count * u32 indices]
    /// Then per instanced group:
    ///   [u32 name_len][name_utf8][f32 r][f32 g][f32 b]
    ///   [u32 vertex_count][u32 index_count][u32 instance_count]
    ///   [vertex_count * 3 * f32 positions]
    ///   [index_count * u32 indices]
    ///   [instance_count * 16 * f32 transform_matrices]
    pub fn export_binary_mesh(&self, path: &Path) -> std::io::Result<()> {
        let mut buf = Vec::new();

        let version: u8 = if self.instanced_groups.is_empty() { 2 } else { 3 };
        buf.push(version);

        if version == 3 {
            // v3: two counts
            buf.extend_from_slice(&(self.meshes.len() as u32).to_le_bytes());
            buf.extend_from_slice(&(self.instanced_groups.len() as u32).to_le_bytes());
        } else {
            // v2: single count
            buf.extend_from_slice(&(self.meshes.len() as u32).to_le_bytes());
        }

        // Regular meshes (same as v2)
        for sm in &self.meshes {
            let name_bytes = sm.name.as_bytes();
            buf.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(name_bytes);
            buf.extend_from_slice(&sm.color[0].to_le_bytes());
            buf.extend_from_slice(&sm.color[1].to_le_bytes());
            buf.extend_from_slice(&sm.color[2].to_le_bytes());
            let vc = sm.mesh.positions.len() as u32;
            let ic = sm.mesh.indices.len() as u32;
            buf.extend_from_slice(&vc.to_le_bytes());
            buf.extend_from_slice(&ic.to_le_bytes());
            for p in &sm.mesh.positions {
                buf.extend_from_slice(&(p.x as f32).to_le_bytes());
                buf.extend_from_slice(&(p.y as f32).to_le_bytes());
                buf.extend_from_slice(&(p.z as f32).to_le_bytes());
            }
            for &i in &sm.mesh.indices {
                buf.extend_from_slice(&i.to_le_bytes());
            }
        }

        // Instanced groups (v3 only)
        for ig in &self.instanced_groups {
            let name_bytes = ig.name.as_bytes();
            buf.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(name_bytes);
            buf.extend_from_slice(&ig.color[0].to_le_bytes());
            buf.extend_from_slice(&ig.color[1].to_le_bytes());
            buf.extend_from_slice(&ig.color[2].to_le_bytes());
            let vc = ig.mesh.positions.len() as u32;
            let ic = ig.mesh.indices.len() as u32;
            let inst_count = ig.transforms.len() as u32;
            buf.extend_from_slice(&vc.to_le_bytes());
            buf.extend_from_slice(&ic.to_le_bytes());
            buf.extend_from_slice(&inst_count.to_le_bytes());
            for p in &ig.mesh.positions {
                buf.extend_from_slice(&(p.x as f32).to_le_bytes());
                buf.extend_from_slice(&(p.y as f32).to_le_bytes());
                buf.extend_from_slice(&(p.z as f32).to_le_bytes());
            }
            for &i in &ig.mesh.indices {
                buf.extend_from_slice(&i.to_le_bytes());
            }
            for transform in &ig.transforms {
                for &val in transform {
                    buf.extend_from_slice(&val.to_le_bytes());
                }
            }
        }

        std::fs::write(path, &buf)
    }

    fn generate_gltf_binary_buffer(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        for scene_mesh in &self.meshes {
            // Write positions
            for pos in &scene_mesh.mesh.positions {
                buffer.extend_from_slice(&(pos.x as f32).to_le_bytes());
                buffer.extend_from_slice(&(pos.y as f32).to_le_bytes());
                buffer.extend_from_slice(&(pos.z as f32).to_le_bytes());
            }

            // Write normals
            for norm in &scene_mesh.mesh.normals {
                buffer.extend_from_slice(&(norm.x as f32).to_le_bytes());
                buffer.extend_from_slice(&(norm.y as f32).to_le_bytes());
                buffer.extend_from_slice(&(norm.z as f32).to_le_bytes());
            }

            // Write indices
            for idx in &scene_mesh.mesh.indices {
                buffer.extend_from_slice(&idx.to_le_bytes());
            }
        }

        buffer
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

// Simple base64 encoder
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    let mut i = 0;
    while i < data.len() {
        let b1 = data[i];
        let b2 = if i + 1 < data.len() { data[i + 1] } else { 0 };
        let b3 = if i + 2 < data.len() { data[i + 2] } else { 0 };

        result.push(CHARS[(b1 >> 2) as usize] as char);
        result.push(CHARS[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize] as char);

        if i + 1 < data.len() {
            result.push(CHARS[(((b2 & 0x0F) << 2) | (b3 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }

        if i + 2 < data.len() {
            result.push(CHARS[(b3 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use cst_math::DVec3;

    fn create_test_triangle() -> TriangleMesh {
        TriangleMesh {
            positions: vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![
                DVec3::new(0.0, 0.0, 1.0),
                DVec3::new(0.0, 0.0, 1.0),
                DVec3::new(0.0, 0.0, 1.0),
            ],
            indices: vec![0, 1, 2],
            uvs: vec![],
        }
    }

    #[test]
    fn test_empty_scene() {
        let scene = Scene::new();
        assert_eq!(scene.meshes.len(), 0);
        assert_eq!(scene.total_triangles(), 0);
    }

    #[test]
    fn test_add_mesh_and_bounds() {
        let mut scene = Scene::new();
        let mesh = create_test_triangle();

        scene.add_mesh("Triangle", mesh, [1.0, 0.0, 0.0]);

        assert_eq!(scene.meshes.len(), 1);
        assert_eq!(scene.meshes[0].name, "Triangle");

        let bounds = scene.bounds().unwrap();
        assert_eq!(bounds.min, DVec3::new(0.0, 0.0, 0.0));
        assert_eq!(bounds.max, DVec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn test_total_triangles() {
        let mut scene = Scene::new();

        let mesh1 = create_test_triangle();
        let mesh2 = create_test_triangle();

        scene.add_mesh("Triangle1", mesh1, [1.0, 0.0, 0.0]);
        scene.add_mesh("Triangle2", mesh2, [0.0, 1.0, 0.0]);

        assert_eq!(scene.total_triangles(), 2);
    }

    #[test]
    fn test_auto_color_cycles() {
        let mut scene = Scene::new();

        for i in 0..12 {
            let mesh = create_test_triangle();
            scene.add_mesh_auto_color(&format!("Mesh{}", i), mesh);
        }

        assert_eq!(scene.meshes.len(), 12);

        // Check that colors cycle through palette
        assert_eq!(scene.meshes[0].color, scene.meshes[10].color);
        assert_ne!(scene.meshes[0].color, scene.meshes[1].color);
    }

    #[test]
    fn test_html_export() {
        let mut scene = Scene::new();
        let mesh = create_test_triangle();
        scene.add_mesh("TestTriangle", mesh, [0.5, 0.6, 0.7]);

        let temp_dir = std::env::temp_dir();
        let html_path = temp_dir.join("test_scene.html");

        let result = scene.export_html(&html_path);
        assert!(result.is_ok());

        // Check file was created and has content
        let metadata = std::fs::metadata(&html_path);
        assert!(metadata.is_ok());
        assert!(metadata.unwrap().len() > 0);

        // Read file and check for key elements
        let content = std::fs::read_to_string(&html_path).unwrap();
        assert!(content.contains("<!DOCTYPE html>"));
        assert!(content.contains("three.min.js"));
        assert!(content.contains("TestTriangle"));
        assert!(content.contains("meshData"));

        // Cleanup
        let _ = std::fs::remove_file(html_path);
    }

    #[test]
    fn test_gltf_json_valid() {
        let mut scene = Scene::new();
        let mesh = create_test_triangle();
        scene.add_mesh("TestMesh", mesh, [0.8, 0.2, 0.3]);

        let json = scene.export_gltf_json();

        // Check JSON is valid by parsing it
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        assert!(parsed.is_ok(), "Generated glTF JSON should be valid");

        let gltf = parsed.unwrap();

        // Check key fields
        assert!(gltf["asset"]["version"].as_str().unwrap() == "2.0");
        assert!(gltf["scenes"].is_array());
        assert!(gltf["nodes"].is_array());
        assert!(gltf["meshes"].is_array());
        assert!(gltf["materials"].is_array());
        assert!(gltf["accessors"].is_array());
        assert!(gltf["bufferViews"].is_array());
        assert!(gltf["buffers"].is_array());
    }

    #[test]
    fn test_empty_bounds() {
        let scene = Scene::new();
        let bounds = scene.bounds();
        assert!(bounds.is_none());
    }
}
