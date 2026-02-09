> **[한국어](README.ko.md)** | English

# cst-ifc-rs

High-performance Rust IFC (Industry Foundation Classes) parser and mesh converter for BIM/CAD applications.

## Features

- **Streaming IFC Parser**: Memory-efficient STEP text parsing for large IFC files (tested with 400MB+ files)
- **Geometry Extraction**: IFCFACETEDBREP triangulation with color/material support
- **Mesh Conversion**: Direct conversion to triangle meshes with vertex deduplication
- **Binary Export**: Compact binary mesh format (v3) with geometry instancing support
- **Three.js Integration**: Export scenes for web-based 3D rendering

## Benchmarks

**Test File**: Production concrete building IFC (Tekla Structures export)
- File size: **395 MB**
- Lines: **4,847,558**
- IFC products: **276,593 elements**
- Geometry entities: **3,714,855**

**Performance** (Windows 11, Release build, Rust 1.93.0):

| Operation | Time | Throughput |
|-----------|------|------------|
| Parse + Mesh Convert | **39.3 seconds** | 10 MB/s, 123K lines/sec |
| Full Pipeline (parse + convert + export) | **51.4 seconds** | 730K triangles/sec |
| Binary Export | ~12 seconds | 848.5 MB output |

**Output**:
- Vertices: **45,424,444**
- Triangles: **28,716,826**
- Binary mesh size: **848.5 MB** (v3 format with instancing)
- Geometry instancing: **8 duplicate groups** detected

**Architecture**: Single-threaded STEP parsing, multi-threaded mesh tessellation (rayon)

## Test Results

✅ **183 tests passing** across all crates:
- cst-geometry: 47 tests
- cst-ifc: 70 tests (66 unit + 4 integration)
- cst-math: 10 tests
- cst-mesh: 20 tests
- cst-render: 22 tests
- cst-topology: 14 tests

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

// Parse IFC file (streaming, constant memory)
let ifc_data = ifc_reader::read_ifc_file("model.ifc".as_ref())?;

// Convert geometries to triangle meshes
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

// Export to interactive HTML viewer
scene.export_html("output.html".as_ref())?;
```

### CLI Tools

```bash
# Parse and convert IFC to binary mesh
cargo run --release --bin ifc_viewer -- input.ifc output.html

# Run test suite
cargo test --release
```

## Binary Mesh Format (v3)

Efficient binary format with geometry instancing support:

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

**Features**:
- Automatic duplicate geometry detection
- Transform-based instancing for repeated elements
- ~40-60% size reduction for typical BIM models

## Architecture

- **Memory Efficiency**: Streaming STEP parser with constant memory footprint
- **Parallel Processing**: Multi-threaded mesh tessellation via rayon
- **Geometry Instancing**: Automatic detection and reuse of duplicate meshes
- **Robust Parsing**: Error recovery and validation for malformed IFC data

## Dependencies

All dependencies are MIT/Apache-2.0 licensed:
- glam 0.29 (math primitives)
- nalgebra 0.33 (linear algebra)
- slotmap 1.0 (arena allocation)
- rayon 1.10 (parallel processing)
- serde 1.0 + bincode 1.0 (serialization)

## License

Dual-licensed under MIT or Apache-2.0.
