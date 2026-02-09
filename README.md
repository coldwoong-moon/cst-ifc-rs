# cst-ifc-rs

High-performance Rust IFC (Industry Foundation Classes) parser and mesh converter for BIM/CAD applications.

## Features

- **Streaming IFC Parser**: Memory-efficient STEP text parsing for large IFC files (tested with 400MB+ files)
- **Geometry Extraction**: IFCFACETEDBREP triangulation with color/material support
- **Mesh Conversion**: Direct conversion to triangle meshes with vertex deduplication
- **Binary Export**: Compact binary mesh format (v3) with geometry instancing support
- **Three.js Integration**: Export scenes for web-based 3D rendering

## Crate Structure

| Crate | Description |
|-------|-------------|
| `cst-core` | Core types, errors, and utilities |
| `cst-math` | Math primitives (glam-based): vectors, matrices, transforms |
| `cst-topology` | B-Rep topology: Half-Edge data structure (slotmap arena) |
| `cst-geometry` | Curves, surfaces, NURBS evaluation |
| `cst-mesh` | B-Rep to triangle mesh tessellation |
| `cst-ifc` | IFC/STEP parser and entity mapping |
| `cst-render` | Scene management and binary mesh export |

## Quick Start

```rust
use cst_ifc::ifc_reader;
use cst_ifc::ifc_to_mesh;
use cst_mesh::TriangleMesh;
use cst_render::Scene;

// Parse IFC file
let ifc_data = ifc_reader::read_ifc_file("model.ifc".as_ref()).unwrap();

// Convert to meshes
let mut scene = Scene::new();
for mesh_data in &ifc_data {
    let trimesh = ifc_to_mesh::faces_to_trimesh(&mesh_data.name, &mesh_data.faces);
    if trimesh.triangle_count() > 0 {
        let mesh = TriangleMesh {
            positions: trimesh.positions,
            normals: trimesh.normals,
            indices: trimesh.indices,
            uvs: vec![],
        };
        let color = mesh_data.color.unwrap_or([0.7, 0.7, 0.7]);
        scene.add_mesh(&mesh_data.name, mesh, color);
    }
}

// Export to HTML viewer
scene.export_html("output.html".as_ref()).unwrap();
```

## Performance

Tested with production IFC files:
- **414MB IFC file**: 4.8M lines, 175K FacetedBreps -> 28.7M triangles
- **Streaming parser**: Constant memory usage regardless of file size
- **Parallel tessellation**: Multi-threaded mesh conversion via rayon

## Binary Mesh Format (v3)

```
[u8 version=3]
[u32 regular_mesh_count]
[u32 instanced_group_count]

For each regular mesh:
  [u32 name_len][name_bytes]
  [f32 r][f32 g][f32 b]
  [u32 vertex_count][u32 index_count]
  [vertex_count x 3 x f32 positions]
  [index_count x u32 indices]

For each instanced group:
  [u32 name_len][name_bytes]
  [f32 r][f32 g][f32 b]
  [u32 vertex_count][u32 index_count][u32 instance_count]
  [vertex_count x 3 x f32 positions]
  [index_count x u32 indices]
  [instance_count x 16 x f32 transform_matrices]
```

## License

Dual-licensed under MIT or Apache-2.0.
