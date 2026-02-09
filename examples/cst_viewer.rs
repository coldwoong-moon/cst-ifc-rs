//! CSTEngine IFC Viewer CLI
//!
//! A command-line tool to convert IFC files to HTML viewers or other formats.
//!
//! # Usage
//!
//! ```bash
//! # Convert IFC to HTML viewer
//! cst_viewer input.ifc [output.html]
//!
//! # If output is not specified, uses input filename with .html extension
//! cst_viewer building.ifc
//! # Creates: building.html
//!
//! # Show summary statistics
//! cst_viewer --summary input.ifc
//!
//! # Export to glTF
//! cst_viewer --gltf input.ifc output.gltf
//! ```

use std::path::{Path, PathBuf};
use std::process;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn print_usage() {
    eprintln!(
        r#"CSTEngine IFC Viewer CLI

USAGE:
    cst_viewer <input.ifc> [output.html]
    cst_viewer --summary <input.ifc>
    cst_viewer --gltf <input.ifc> <output.gltf>

ARGS:
    <input.ifc>     Path to the input IFC file
    [output.html]   Optional output path (defaults to input.html)

OPTIONS:
    --summary       Print statistics about the IFC file
    --gltf          Export to glTF format instead of HTML
    --help          Show this help message

EXAMPLES:
    # Convert to HTML viewer
    cst_viewer building.ifc

    # Specify output path
    cst_viewer building.ifc viewer/index.html

    # Show file statistics
    cst_viewer --summary building.ifc

    # Export to glTF
    cst_viewer --gltf building.ifc building.gltf
"#
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Parse command line arguments
    if args.len() < 2 {
        eprintln!("Error: Missing required argument <input.ifc>\n");
        print_usage();
        process::exit(1);
    }

    // Handle help flag
    if args[1] == "--help" || args[1] == "-h" {
        print_usage();
        process::exit(0);
    }

    // Handle summary mode
    if args[1] == "--summary" {
        if args.len() < 3 {
            eprintln!("Error: --summary requires an input file\n");
            print_usage();
            process::exit(1);
        }

        let ifc_path = Path::new(&args[2]);
        handle_summary(ifc_path);
        return;
    }

    // Handle web export mode (binary mesh data for web viewer)
    if args[1] == "--web" {
        if args.len() < 3 {
            eprintln!("Error: --web requires an input IFC file\n");
            print_usage();
            process::exit(1);
        }

        let ifc_path = Path::new(&args[2]);
        let out_dir = if args.len() > 3 {
            PathBuf::from(&args[3])
        } else {
            PathBuf::from("web_viewer")
        };
        handle_web_export(ifc_path, &out_dir);
        return;
    }

    // Handle glTF export mode
    if args[1] == "--gltf" {
        if args.len() < 4 {
            eprintln!("Error: --gltf requires input and output paths\n");
            print_usage();
            process::exit(1);
        }

        let ifc_path = Path::new(&args[2]);
        let gltf_path = Path::new(&args[3]);
        handle_gltf_export(ifc_path, gltf_path);
        return;
    }

    // Default mode: HTML export
    let ifc_path = Path::new(&args[1]);
    let html_path = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        // Default: replace .ifc extension with .html
        ifc_path.with_extension("html")
    };

    handle_html_export(ifc_path, &html_path);
}

fn handle_html_export(ifc_path: &Path, html_path: &Path) {
    eprintln!("╔════════════════════════════════════════════════════════════╗");
    eprintln!("║           CSTEngine IFC to HTML Viewer                    ║");
    eprintln!("╚════════════════════════════════════════════════════════════╝");
    eprintln!();
    eprintln!("Reading IFC file: {}", ifc_path.display());

    // Check if input file exists
    if !ifc_path.exists() {
        eprintln!("Error: Input file does not exist: {}", ifc_path.display());
        process::exit(1);
    }

    // Perform conversion
    match cst_api::ifc_pipeline::ifc_to_html(ifc_path, html_path) {
        Ok(()) => {
            eprintln!("✓ Conversion successful!");
            eprintln!();
            eprintln!("Exported HTML viewer: {}", html_path.display());
            eprintln!();
            eprintln!("Open the HTML file in a web browser to view the 3D model.");
        }
        Err(e) => {
            eprintln!("Error during conversion: {}", e);
            process::exit(1);
        }
    }
}

fn handle_summary(ifc_path: &Path) {
    if !ifc_path.exists() {
        eprintln!("Error: Input file does not exist: {}", ifc_path.display());
        process::exit(1);
    }

    match cst_api::ifc_pipeline::ifc_summary(ifc_path) {
        Ok(summary) => {
            println!("{}", summary);
        }
        Err(e) => {
            eprintln!("Error generating summary: {}", e);
            process::exit(1);
        }
    }
}

fn handle_web_export(ifc_path: &Path, out_dir: &Path) {
    eprintln!("╔════════════════════════════════════════════════════════════╗");
    eprintln!("║           CSTEngine IFC Web Viewer Export                  ║");
    eprintln!("╚════════════════════════════════════════════════════════════╝");
    eprintln!();
    eprintln!("Reading IFC file: {}", ifc_path.display());

    if !ifc_path.exists() {
        eprintln!("Error: Input file does not exist: {}", ifc_path.display());
        process::exit(1);
    }

    // Create output directory
    if !out_dir.exists() {
        std::fs::create_dir_all(out_dir).unwrap_or_else(|e| {
            eprintln!("Error creating directory: {}", e);
            process::exit(1);
        });
    }

    // Build scene with triangle budget + geometry instancing
    match cst_api::ifc_pipeline::ifc_to_meshes(ifc_path) {
        Ok(meshes) => {
            let mut scene = cst_render::Scene::new();
            let mut total_tris = 0usize;
            const MAX_TRIS: usize = usize::MAX;
            const MAX_BATCHES: usize = 200;

            // --- Phase 2: Hash-based geometry instancing ---
            // Hash each mesh's positions to find duplicates with same color
            use std::collections::HashMap;

            struct MeshEntry {
                idx: usize,
                hash: u64,
                color_key: [u8; 3],
                tris: usize,
            }

            // Compute position hash for each mesh
            let mut entries: Vec<MeshEntry> = Vec::with_capacity(meshes.len());
            for (i, (_, m, color)) in meshes.iter().enumerate() {
                let c = color.unwrap_or([0.7, 0.7, 0.7]);
                let color_key = [
                    (c[0] * 255.0) as u8,
                    (c[1] * 255.0) as u8,
                    (c[2] * 255.0) as u8,
                ];
                // Hash positions as f32 bytes
                let mut hasher = DefaultHasher::new();
                let pos_count = m.positions.len();
                pos_count.hash(&mut hasher);
                for p in &m.positions {
                    let xb = (p.x as f32).to_bits();
                    let yb = (p.y as f32).to_bits();
                    let zb = (p.z as f32).to_bits();
                    xb.hash(&mut hasher);
                    yb.hash(&mut hasher);
                    zb.hash(&mut hasher);
                }
                // Also hash indices
                let idx_count = m.indices.len();
                idx_count.hash(&mut hasher);
                for &idx in &m.indices {
                    idx.hash(&mut hasher);
                }
                let hash = hasher.finish();

                entries.push(MeshEntry {
                    idx: i,
                    hash,
                    color_key,
                    tris: m.triangle_count(),
                });
            }

            // Group by (hash, color_key) to find duplicates
            let mut instance_groups: HashMap<(u64, [u8; 3]), Vec<usize>> = HashMap::new();
            for entry in &entries {
                instance_groups
                    .entry((entry.hash, entry.color_key))
                    .or_default()
                    .push(entry.idx);
            }

            // Separate: groups with 2+ members are instanced, rest are regular
            let mut instanced_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
            let mut instance_group_list: Vec<(u64, [u8; 3], Vec<usize>)> = Vec::new();
            let mut instanced_tris = 0usize;
            let mut instanced_total_drawn = 0usize;

            for ((hash, color_key), indices) in &instance_groups {
                if indices.len() >= 2 {
                    for &idx in indices {
                        instanced_indices.insert(idx);
                    }
                    let base_tris = meshes[indices[0]].1.triangle_count();
                    instanced_tris += base_tris; // Only count base geometry once
                    instanced_total_drawn += base_tris * indices.len();
                    instance_group_list.push((*hash, *color_key, indices.clone()));
                }
            }

            let regular_count = meshes.len() - instanced_indices.len();
            eprintln!("Instancing: {} groups ({} meshes → {} base geometries, {} instanced tris drawn as {})",
                instance_group_list.len(),
                instanced_indices.len(),
                instance_group_list.len(),
                instanced_tris,
                instanced_total_drawn);

            // --- Add instanced groups to scene ---
            for (_hash, color_key, indices) in &instance_group_list {
                let base_idx = indices[0];
                let base_mesh = &meshes[base_idx].1;
                let color = [
                    color_key[0] as f32 / 255.0,
                    color_key[1] as f32 / 255.0,
                    color_key[2] as f32 / 255.0,
                ];

                // Compute centroid of base mesh
                let base_centroid = if base_mesh.positions.is_empty() {
                    cst_math::DVec3::ZERO
                } else {
                    let sum: cst_math::DVec3 = base_mesh.positions.iter().copied().sum();
                    sum / base_mesh.positions.len() as f64
                };

                // Build transforms: translation from base centroid to each instance centroid
                let mut transforms = Vec::with_capacity(indices.len());
                for &idx in indices {
                    let inst_mesh = &meshes[idx].1;
                    let inst_centroid = if inst_mesh.positions.is_empty() {
                        cst_math::DVec3::ZERO
                    } else {
                        let sum: cst_math::DVec3 = inst_mesh.positions.iter().copied().sum();
                        sum / inst_mesh.positions.len() as f64
                    };
                    let offset = inst_centroid - base_centroid;
                    // 4x4 identity + translation (column-major)
                    let mat: [f32; 16] = [
                        1.0, 0.0, 0.0, 0.0,
                        0.0, 1.0, 0.0, 0.0,
                        0.0, 0.0, 1.0, 0.0,
                        offset.x as f32, offset.y as f32, offset.z as f32, 1.0,
                    ];
                    transforms.push(mat);
                }

                let name = format!("Inst_{:02x}{:02x}{:02x}_{}", color_key[0], color_key[1], color_key[2], indices.len());
                scene.add_instanced_group(&name, base_mesh.clone(), color, transforms);
            }

            // --- Budget allocation for regular (non-instanced) meshes ---
            // Build color groups from non-instanced meshes only
            let mut all_color_groups: HashMap<[u8; 3], Vec<(usize, usize)>> = HashMap::new();
            for entry in &entries {
                if instanced_indices.contains(&entry.idx) {
                    continue;
                }
                all_color_groups
                    .entry(entry.color_key)
                    .or_default()
                    .push((entry.idx, entry.tris));
            }
            for group in all_color_groups.values_mut() {
                group.sort_by(|a, b| b.1.cmp(&a.1));
            }

            // Remaining budget for regular meshes (instanced already counted)
            let regular_budget = MAX_TRIS.saturating_sub(instanced_tris);

            // Step 2: Compute group stats
            let num_groups = all_color_groups.len();
            let grand_total: usize = all_color_groups.values()
                .flat_map(|g| g.iter()).map(|(_, t)| t).sum();

            // Step 3: Balanced allocation
            let max_share_pct = 0.35;
            let min_share = regular_budget / (num_groups * 2).max(1);

            let mut group_shares: Vec<([u8; 3], usize, usize)> = Vec::new();
            let mut capped_total = 0usize;
            let mut uncapped_total = 0usize;

            for (key, group) in &all_color_groups {
                let group_total: usize = group.iter().map(|(_, t)| *t).sum();
                let raw_share = if grand_total > 0 {
                    ((group_total as f64 / grand_total as f64) * regular_budget as f64) as usize
                } else { 0 };
                let max_cap = (regular_budget as f64 * max_share_pct) as usize;
                if raw_share > max_cap {
                    group_shares.push((*key, max_cap, group_total));
                    capped_total += max_cap;
                } else {
                    group_shares.push((*key, raw_share, group_total));
                    uncapped_total += raw_share;
                }
            }

            let excess = regular_budget.saturating_sub(capped_total + uncapped_total);
            let mut budget_indices = Vec::new();

            for (key, base_share, _group_total) in &group_shares {
                let mut share = *base_share;
                if share < (regular_budget as f64 * max_share_pct) as usize && uncapped_total > 0 {
                    share += ((share as f64 / uncapped_total as f64) * excess as f64) as usize;
                }
                let share = share.max(min_share);

                let group = &all_color_groups[key];
                let mut used = 0usize;
                for (idx, tris) in group {
                    if used + tris > share && used > 0 {
                        break;
                    }
                    used += tris;
                    budget_indices.push(*idx);
                }
                total_tris += used;
            }

            eprintln!("Regular meshes: {} of {} using {} tris (budget {})",
                budget_indices.len(), regular_count, total_tris, regular_budget);
            eprintln!("Total display: {} regular tris + {} instanced drawn = {} effective tris",
                total_tris, instanced_total_drawn, total_tris + instanced_total_drawn);

            // Group budget meshes by color for batch merge
            let mut color_groups: HashMap<[u8; 3], Vec<usize>> = HashMap::new();
            for &idx in &budget_indices {
                let color = meshes[idx].2.unwrap_or([0.7, 0.7, 0.7]);
                let key = [
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                ];
                color_groups.entry(key).or_default().push(idx);
            }

            // Merge each color group into batches
            for (color_key, group_indices) in &color_groups {
                let color = [
                    color_key[0] as f32 / 255.0,
                    color_key[1] as f32 / 255.0,
                    color_key[2] as f32 / 255.0,
                ];
                let max_per_batch = (group_indices.len() + MAX_BATCHES - 1).max(1);
                let sub_batch_size = (group_indices.len() / ((group_indices.len() / max_per_batch).max(1))).max(1);
                for (bi, chunk) in group_indices.chunks(sub_batch_size).enumerate() {
                    let mut positions = Vec::new();
                    let mut normals = Vec::new();
                    let mut indices = Vec::new();
                    let mut offset = 0u32;
                    for &idx in chunk {
                        let m = &meshes[idx].1;
                        positions.extend_from_slice(&m.positions);
                        normals.extend_from_slice(&m.normals);
                        for &i in &m.indices {
                            indices.push(i + offset);
                        }
                        offset += m.positions.len() as u32;
                    }
                    let merged = cst_mesh::TriangleMesh {
                        positions, normals, indices, uvs: vec![],
                    };
                    scene.add_mesh(
                        &format!("Color_{:02x}{:02x}{:02x}_{}", color_key[0], color_key[1], color_key[2], bi),
                        merged,
                        color,
                    );
                }
            }

            // Export binary mesh data
            let bin_path = out_dir.join("mesh.bin");
            match scene.export_binary_mesh(&bin_path) {
                Ok(()) => {
                    let size = std::fs::metadata(&bin_path).map(|m| m.len()).unwrap_or(0);
                    eprintln!("Exported mesh.bin: {} bytes ({:.1} MB)",
                        size, size as f64 / 1_048_576.0);
                }
                Err(e) => {
                    eprintln!("Error exporting binary mesh: {}", e);
                    process::exit(1);
                }
            }

            eprintln!();
            eprintln!("✓ Web export complete! Files in: {}", out_dir.display());
            eprintln!();
            eprintln!("To start the viewer:");
            eprintln!("  cd {} && node server.js", out_dir.display());
            eprintln!("  Then open http://localhost:3000");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn handle_gltf_export(ifc_path: &Path, gltf_path: &Path) {
    eprintln!("╔════════════════════════════════════════════════════════════╗");
    eprintln!("║           CSTEngine IFC to glTF Exporter                  ║");
    eprintln!("╚════════════════════════════════════════════════════════════╝");
    eprintln!();
    eprintln!("Reading IFC file: {}", ifc_path.display());

    if !ifc_path.exists() {
        eprintln!("Error: Input file does not exist: {}", ifc_path.display());
        process::exit(1);
    }

    match cst_api::ifc_pipeline::ifc_to_gltf(ifc_path, gltf_path) {
        Ok(()) => {
            eprintln!("✓ Export successful!");
            eprintln!();
            eprintln!("Exported glTF file: {}", gltf_path.display());
        }
        Err(e) => {
            eprintln!("Error during export: {}", e);
            eprintln!();
            eprintln!("Note: glTF export requires the cst-gltf exporter module,");
            eprintln!("which may not be implemented yet.");
            process::exit(1);
        }
    }
}
