use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use cst_math::{DVec3, DVec4, DMat4};
use cst_core::Result;

/// A lightweight parsed IFC entity from streaming reader
#[derive(Debug, Clone)]
pub struct IfcRawEntity {
    pub entity_id: u64,
    pub type_name: String,
    pub raw_args: String,  // raw argument text between outer parens
}

/// Face data extracted from IFC: outer boundary + optional hole boundaries
#[derive(Debug, Clone)]
pub struct IfcFaceData {
    pub outer: Vec<DVec3>,
    pub holes: Vec<Vec<DVec3>>,
}

/// Geometry data extracted from IFC file
#[derive(Debug, Clone)]
pub struct IfcMeshData {
    pub name: String,
    pub faces: Vec<IfcFaceData>,  // each face has outer boundary + optional holes
    pub placement: Option<[f64; 12]>,  // 3x4 transform matrix (row major), or None
    pub color: Option<[f32; 3]>,  // RGB color from IFC style chain, if found
}

/// Product types that carry geometry in IFC models
const PRODUCT_TYPES: &[&str] = &[
    "IFCBEAM", "IFCCOLUMN", "IFCSLAB", "IFCWALL", "IFCWALLSTANDARDCASE",
    "IFCPLATE", "IFCMEMBER", "IFCREINFORCINGBAR", "IFCFOOTING",
    "IFCBUILDINGELEMENTPROXY", "IFCROOF", "IFCSTAIR", "IFCSTAIRFLIGHT",
    "IFCRAILING", "IFCRAMP", "IFCRAMPFLIGHT", "IFCDOOR", "IFCWINDOW",
    "IFCCOVERING", "IFCCURTAINWALL", "IFCPILE", "IFCTENDON",
    "IFCREINFORCINGMESH",
];

/// Build a map from brep entity id -> [r, g, b] color by resolving the IFC style chain:
///   IFCSTYLEDITEM(brep_ref, (style_assignment), ...) ->
///   IFCPRESENTATIONSTYLEASSIGNMENT((surface_style, ...)) ->
///   IFCSURFACESTYLE(name, side, (rendering, ...)) ->
///   IFCSURFACESTYLERENDERING(colour_ref, ...) ->
///   IFCCOLOURRGB(name, r, g, b)
fn build_brep_color_map(entities: &HashMap<u64, IfcRawEntity>) -> HashMap<u64, [f32; 3]> {
    let mut color_map = HashMap::new();

    // Find all IFCSTYLEDITEM entities
    for (_, entity) in entities.iter() {
        if entity.type_name != "IFCSTYLEDITEM" {
            continue;
        }

        // IFCSTYLEDITEM(Item, Styles, Name)
        // Item = reference to a representation item (e.g., IFCFACETEDBREP)
        // Styles = set of style assignments
        let args = split_ifc_args(&entity.raw_args);
        if args.len() < 2 {
            continue;
        }

        let item_id = match extract_single_ref(&args[0]) {
            Some(id) => id,
            None => continue,
        };

        // Parse the style assignments set
        let style_refs = parse_entity_refs(&args[1]);

        for style_assign_id in style_refs {
            if let Some(color) = resolve_style_assignment_to_color(style_assign_id, entities) {
                color_map.insert(item_id, color);
                break;
            }
        }
    }

    color_map
}

/// Resolve an IFCPRESENTATIONSTYLEASSIGNMENT to an RGB color.
fn resolve_style_assignment_to_color(
    assign_id: u64,
    entities: &HashMap<u64, IfcRawEntity>,
) -> Option<[f32; 3]> {
    let assign = entities.get(&assign_id)?;
    if assign.type_name != "IFCPRESENTATIONSTYLEASSIGNMENT" {
        return None;
    }

    // IFCPRESENTATIONSTYLEASSIGNMENT((style1, style2, ...))
    let assign_args = split_ifc_args(&assign.raw_args);
    if assign_args.is_empty() {
        return None;
    }
    let style_refs = parse_entity_refs(&assign_args[0]);

    for style_id in style_refs {
        if let Some(color) = resolve_surface_style_to_color(style_id, entities) {
            return Some(color);
        }
    }
    None
}

/// Resolve an IFCSURFACESTYLE to an RGB color.
fn resolve_surface_style_to_color(
    style_id: u64,
    entities: &HashMap<u64, IfcRawEntity>,
) -> Option<[f32; 3]> {
    let style = entities.get(&style_id)?;
    if style.type_name != "IFCSURFACESTYLE" {
        return None;
    }

    // IFCSURFACESTYLE(Name, Side, Styles)
    // Styles is a set of surface style elements (rendering, lighting, etc.)
    let style_args = split_ifc_args(&style.raw_args);
    if style_args.len() < 3 {
        return None;
    }
    let rendering_refs = parse_entity_refs(&style_args[2]);

    for rendering_id in rendering_refs {
        if let Some(color) = resolve_rendering_to_color(rendering_id, entities) {
            return Some(color);
        }
    }
    None
}

/// Resolve an IFCSURFACESTYLERENDERING to an RGB color.
fn resolve_rendering_to_color(
    rendering_id: u64,
    entities: &HashMap<u64, IfcRawEntity>,
) -> Option<[f32; 3]> {
    let rendering = entities.get(&rendering_id)?;
    if rendering.type_name != "IFCSURFACESTYLERENDERING" {
        return None;
    }

    // IFCSURFACESTYLERENDERING(SurfaceColour, ...)
    // SurfaceColour is the first argument, a reference to IFCCOLOURRGB
    let rendering_args = split_ifc_args(&rendering.raw_args);
    if rendering_args.is_empty() {
        return None;
    }

    let colour_id = extract_single_ref(&rendering_args[0])?;
    resolve_colour_rgb(colour_id, entities)
}

/// Resolve an IFCCOLOURRGB to [r, g, b].
fn resolve_colour_rgb(
    colour_id: u64,
    entities: &HashMap<u64, IfcRawEntity>,
) -> Option<[f32; 3]> {
    let colour = entities.get(&colour_id)?;
    if colour.type_name != "IFCCOLOURRGB" {
        return None;
    }

    // IFCCOLOURRGB(Name, Red, Green, Blue)
    let colour_args = split_ifc_args(&colour.raw_args);
    if colour_args.len() < 4 {
        return None;
    }

    let r = colour_args[1].trim().parse::<f32>().ok()?;
    let g = colour_args[2].trim().parse::<f32>().ok()?;
    let b = colour_args[3].trim().parse::<f32>().ok()?;
    Some([r, g, b])
}

/// Read an IFC file and extract faceted brep geometry data.
/// Resolves product placement chains and IFCMAPPEDITEM instances so that
/// geometry is placed at world coordinates rather than all at origin.
pub fn read_ifc_file(path: &Path) -> Result<Vec<IfcMeshData>> {
    // Phase 1: Stream through file, collect entities into HashMap by id
    let entities = parse_ifc_entities(path)?;

    // Phase 1b: Build brep -> color lookup from style chain
    let brep_color_map = build_brep_color_map(&entities);
    eprintln!("Built color map with {} entries", brep_color_map.len());

    // Phase 2: Find all product elements
    let products: Vec<(u64, &IfcRawEntity)> = entities.iter()
        .filter(|(_, e)| PRODUCT_TYPES.contains(&e.type_name.as_str()))
        .map(|(id, e)| (*id, e))
        .collect();

    eprintln!("Found {} product elements", products.len());

    // Phase 3: Resolve each product to positioned mesh data
    let mut results = Vec::new();

    for (product_id, product) in &products {
        let args = split_ifc_args(&product.raw_args);
        // Product args layout (IFC2x3/IFC4):
        // 0=GlobalId, 1=OwnerHistory, 2=Name, 3=Description, 4=ObjectType,
        // 5=ObjectPlacement, 6=Representation, 7=Tag, [8..]=type-specific
        if args.len() < 7 { continue; }

        let name = args[2].trim().trim_matches('\'').to_string();
        let name = if name == "$" || name.is_empty() {
            format!("{}_{}", product.type_name, product_id)
        } else {
            name
        };

        let placement_id = extract_single_ref(&args[5]);
        let representation_id = extract_single_ref(&args[6]);

        // Resolve world transform from IFCLOCALPLACEMENT chain
        let world_transform = placement_id
            .map(|pid| resolve_placement_chain(pid, &entities))
            .unwrap_or(DMat4::IDENTITY);

        // Resolve geometry from representation (IFCPRODUCTDEFINITIONSHAPE)
        let rep_id = match representation_id {
            Some(id) => id,
            None => continue,
        };

        let prod_def = match entities.get(&rep_id) {
            Some(e) => e,
            None => continue,
        };

        // IFCPRODUCTDEFINITIONSHAPE($,$,(#rep1,#rep2,...))
        // The shape reps are in the 3rd argument (index 2)
        let pd_args = split_ifc_args(&prod_def.raw_args);
        let shape_rep_arg = if pd_args.len() >= 3 { &pd_args[2] } else { &prod_def.raw_args };
        let shape_rep_refs = parse_entity_refs(shape_rep_arg);

        for shape_rep_id in shape_rep_refs {
            let shape_rep = match entities.get(&shape_rep_id) {
                Some(e) if e.type_name == "IFCSHAPEREPRESENTATION" => e,
                _ => continue,
            };

            // Get items from shape representation (4th arg, index 3)
            let sr_args = split_ifc_args(&shape_rep.raw_args);
            if sr_args.len() < 4 { continue; }
            let item_refs = parse_entity_refs(&sr_args[3]);

            for item_id in item_refs {
                let item = match entities.get(&item_id) {
                    Some(e) => e,
                    None => continue,
                };

                match item.type_name.as_str() {
                    "IFCFACETEDBREP" => {
                        // Direct brep - apply world transform
                        if let Some(mut mesh) = resolve_faceted_brep(item_id, &entities) {
                            mesh.name = format!("{}_{}", name, product_id);
                            mesh.color = brep_color_map.get(&item_id).copied();
                            apply_transform_to_faces(&mut mesh.faces, &world_transform);
                            results.push(mesh);
                        }
                    }
                    "IFCMAPPEDITEM" => {
                        // Mapped item: resolve source brep + mapping transform
                        resolve_mapped_item(
                            item_id, &item, &name, *product_id,
                            &world_transform, &entities, &brep_color_map, &mut results,
                        );
                    }
                    _ => {} // Skip other item types (e.g. IFCBOOLEANCLIPPINGRESULT)
                }
            }
        }
    }

    // Fallback: if no products found, use legacy brep-only approach
    if results.is_empty() {
        eprintln!("No products found, falling back to direct brep extraction");
        let brep_ids: Vec<u64> = entities.iter()
            .filter(|(_, entity)| entity.type_name == "IFCFACETEDBREP")
            .map(|(id, _)| *id)
            .collect();
        for brep_id in brep_ids {
            if let Some(mut mesh) = resolve_faceted_brep(brep_id, &entities) {
                mesh.color = brep_color_map.get(&brep_id).copied();
                results.push(mesh);
            }
        }
    }

    eprintln!("Parsed {} mesh objects from IFC", results.len());
    Ok(results)
}

/// Resolve an IFCMAPPEDITEM into one or more meshes and push them to results.
fn resolve_mapped_item(
    _item_id: u64,
    item: &IfcRawEntity,
    name: &str,
    product_id: u64,
    world_transform: &DMat4,
    entities: &HashMap<u64, IfcRawEntity>,
    brep_color_map: &HashMap<u64, [f32; 3]>,
    results: &mut Vec<IfcMeshData>,
) {
    let mi_args = split_ifc_args(&item.raw_args);
    if mi_args.len() < 2 { return; }
    let map_source_id = extract_single_ref(&mi_args[0]);
    let map_target_id = extract_single_ref(&mi_args[1]);

    // Resolve mapping target transform (IFCCARTESIANTRANSFORMATIONOPERATOR3D)
    let mapping_transform = map_target_id
        .map(|tid| resolve_cartesian_transform_operator(tid, entities))
        .unwrap_or(DMat4::IDENTITY);

    // Combined transform: world placement * mapping target operator
    let combined = *world_transform * mapping_transform;

    // Resolve RepresentationMap -> source shape rep -> find breps
    if let Some(map_id) = map_source_id {
        if let Some(rep_map) = entities.get(&map_id) {
            if rep_map.type_name == "IFCREPRESENTATIONMAP" {
                let rm_args = split_ifc_args(&rep_map.raw_args);
                // IFCREPRESENTATIONMAP(MappingOrigin, MappedRepresentation)
                if rm_args.len() >= 2 {
                    let _origin_id = extract_single_ref(&rm_args[0]);
                    let mapped_rep_id = extract_single_ref(&rm_args[1]);

                    // Note: MappingOrigin (IFCAXIS2PLACEMENT3D) defines the coordinate
                    // system of the mapped representation. In most IFC files this is
                    // identity (origin at 0,0,0 with default axes). The actual instance
                    // positioning comes from the CartesianTransformationOperator (map_target).
                    // We skip applying origin_transform since the brep vertices are already
                    // in the representation map's coordinate system.

                    if let Some(srep_id) = mapped_rep_id {
                        if let Some(srep) = entities.get(&srep_id) {
                            if srep.type_name == "IFCSHAPEREPRESENTATION" {
                                let srep_args = split_ifc_args(&srep.raw_args);
                                if srep_args.len() >= 4 {
                                    let brep_refs = parse_entity_refs(&srep_args[3]);
                                    for brep_id in brep_refs {
                                        if let Some(e) = entities.get(&brep_id) {
                                            if e.type_name == "IFCFACETEDBREP" {
                                                if let Some(mut mesh) = resolve_faceted_brep(brep_id, entities) {
                                                    mesh.name = format!("{}_{}", name, product_id);
                                                    // Color may be on the brep directly
                                                    mesh.color = brep_color_map.get(&brep_id).copied();
                                                    apply_transform_to_faces(&mut mesh.faces, &combined);
                                                    results.push(mesh);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Parse IFC file line-by-line and collect geometry-related entities
fn parse_ifc_entities(path: &Path) -> Result<HashMap<u64, IfcRawEntity>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut entities = HashMap::new();
    let mut line_count = 0usize;
    let mut current_line = String::new();

    // Geometry-related entity types we care about
    let geometry_types = [
        // Points, directions, loops
        "IFCCARTESIANPOINT", "IFCDIRECTION", "IFCPOLYLOOP",
        // Face bounds (both outer and regular)
        "IFCFACEOUTERBOUND", "IFCFACEBOUND",
        // Face and shell entities
        "IFCFACE", "IFCCLOSEDSHELL", "IFCOPENSHELL",
        // Brep
        "IFCFACETEDBREP",
        // Representation entities
        "IFCSHAPEREPRESENTATION", "IFCPRODUCTDEFINITIONSHAPE",
        // Placement entities
        "IFCAXIS2PLACEMENT3D", "IFCLOCALPLACEMENT",
        // MappedItem chain
        "IFCMAPPEDITEM", "IFCREPRESENTATIONMAP",
        "IFCCARTESIANTRANSFORMATIONOPERATOR3D",
        // Style chain for color extraction
        "IFCSTYLEDITEM", "IFCPRESENTATIONSTYLEASSIGNMENT",
        "IFCSURFACESTYLE", "IFCSURFACESTYLERENDERING", "IFCCOLOURRGB",
        // Structural product types
        "IFCSLAB", "IFCWALL", "IFCWALLSTANDARDCASE", "IFCBEAM", "IFCCOLUMN",
        "IFCPLATE", "IFCMEMBER",
        // Additional product types
        "IFCREINFORCINGBAR", "IFCBUILDINGELEMENTPROXY", "IFCFOOTING", "IFCROOF",
        "IFCSTAIR", "IFCSTAIRFLIGHT", "IFCRAILING", "IFCRAMP", "IFCRAMPFLIGHT",
        "IFCDOOR", "IFCWINDOW", "IFCCOVERING", "IFCCURTAINWALL",
        "IFCPILE", "IFCTENDON", "IFCREINFORCINGMESH",
    ];

    for line in reader.lines() {
        let line = line?;
        line_count += 1;

        if line_count % 500_000 == 0 {
            eprintln!("Parsed {} lines, {} entities...", line_count, entities.len());
        }

        // Skip non-entity lines
        if !line.starts_with('#') {
            continue;
        }

        // Accumulate multi-line entities
        current_line.push_str(&line);

        // Check if entity is complete (ends with semicolon)
        if !current_line.ends_with(';') {
            continue;
        }

        // Parse complete entity
        if let Some(entity) = parse_entity_line(&current_line) {
            // Only keep geometry-related entities
            if geometry_types.contains(&entity.type_name.as_str()) {
                entities.insert(entity.entity_id, entity);
            }
        }

        current_line.clear();
    }

    eprintln!("Finished parsing: {} total lines, {} geometry entities", line_count, entities.len());
    Ok(entities)
}

/// Parse a single entity line like "#47= IFCCARTESIANPOINT((165379.999999999,22500.,18830.));"
fn parse_entity_line(line: &str) -> Option<IfcRawEntity> {
    let line = line.trim();

    // Extract entity ID
    let id_end = line.find('=')?;
    let id_str = &line[1..id_end].trim();
    let entity_id = id_str.parse::<u64>().ok()?;

    // Extract type name
    let type_start = id_end + 1;
    let type_section = &line[type_start..].trim();
    let paren_pos = type_section.find('(')?;
    let type_name = type_section[..paren_pos].trim().to_string();

    // Extract raw args (between outer parens, excluding the parens themselves)
    let args_start = type_section.find('(')?;
    let args_end = type_section.rfind(')')?;
    let raw_args = type_section[args_start + 1..args_end].to_string();

    Some(IfcRawEntity {
        entity_id,
        type_name,
        raw_args,
    })
}

/// Split IFC arguments at top-level commas, respecting nested parens and strings.
///
/// For example, `"'name',$,#51,(#145),0.5,.NOTDEFINED."` produces:
/// `["'name'", "$", "#51", "(#145)", "0.5", ".NOTDEFINED."]`
fn split_ifc_args(raw_args: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut in_string = false;

    for ch in raw_args.chars() {
        match ch {
            '\'' => {
                in_string = !in_string;
                current.push(ch);
            }
            '(' if !in_string => {
                depth += 1;
                current.push(ch);
            }
            ')' if !in_string => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 && !in_string => {
                result.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    // Push the last argument
    let last = current.trim().to_string();
    if !last.is_empty() {
        result.push(last);
    }

    result
}

/// Extract a single entity reference (#NNN) from a positional argument string.
/// Returns None if the argument is "$", empty, or contains no reference.
fn extract_single_ref(arg: &str) -> Option<u64> {
    let trimmed = arg.trim();
    if trimmed == "$" || trimmed.is_empty() {
        return None;
    }

    // Find first # and parse the number after it
    if let Some(hash_pos) = trimmed.find('#') {
        let after_hash = &trimmed[hash_pos + 1..];
        let num_str: String = after_hash.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !num_str.is_empty() {
            return num_str.parse::<u64>().ok();
        }
    }

    None
}

// ── Transform resolution functions ──────────────────────────────────────────

/// Resolve an IFCLOCALPLACEMENT chain to a world transform matrix.
/// IFCLOCALPLACEMENT has two args: (PlacementRelTo, RelativePlacement).
/// PlacementRelTo is another IFCLOCALPLACEMENT or $ (world origin).
/// RelativePlacement is an IFCAXIS2PLACEMENT3D.
fn resolve_placement_chain(placement_id: u64, entities: &HashMap<u64, IfcRawEntity>) -> DMat4 {
    let entity = match entities.get(&placement_id) {
        Some(e) if e.type_name == "IFCLOCALPLACEMENT" => e,
        _ => return DMat4::IDENTITY,
    };

    let args = split_ifc_args(&entity.raw_args);

    // Parent placement (recursive)
    let parent_transform = if !args.is_empty() {
        extract_single_ref(&args[0])
            .map(|pid| resolve_placement_chain(pid, entities))
            .unwrap_or(DMat4::IDENTITY)
    } else {
        DMat4::IDENTITY
    };

    // Relative placement (IFCAXIS2PLACEMENT3D)
    let relative_transform = if args.len() > 1 {
        extract_single_ref(&args[1])
            .map(|aid| resolve_axis2placement3d(aid, entities))
            .unwrap_or(DMat4::IDENTITY)
    } else {
        DMat4::IDENTITY
    };

    parent_transform * relative_transform
}

/// Resolve IFCAXIS2PLACEMENT3D to a DMat4 transformation matrix.
/// Args: (Location, Axis, RefDirection) where Axis and RefDirection are optional.
fn resolve_axis2placement3d(id: u64, entities: &HashMap<u64, IfcRawEntity>) -> DMat4 {
    let entity = match entities.get(&id) {
        Some(e) if e.type_name == "IFCAXIS2PLACEMENT3D" => e,
        _ => return DMat4::IDENTITY,
    };

    let args = split_ifc_args(&entity.raw_args);

    let location = args.get(0)
        .and_then(|a| extract_single_ref(a))
        .and_then(|pid| parse_point(pid, entities))
        .unwrap_or(DVec3::ZERO);

    let axis = args.get(1)
        .and_then(|a| extract_single_ref(a))
        .and_then(|did| parse_direction(did, entities))
        .unwrap_or(DVec3::Z);

    let ref_dir = args.get(2)
        .and_then(|a| extract_single_ref(a))
        .and_then(|did| parse_direction(did, entities))
        .unwrap_or(DVec3::X);

    // Build rotation matrix from axis (local Z) and ref_direction (local X)
    let z = axis.normalize_or_zero();
    let x_raw = ref_dir.normalize_or_zero();
    let y = z.cross(x_raw).normalize_or_zero();
    let x = y.cross(z).normalize_or_zero(); // Re-orthogonalize

    DMat4::from_cols(
        DVec4::new(x.x, x.y, x.z, 0.0),
        DVec4::new(y.x, y.y, y.z, 0.0),
        DVec4::new(z.x, z.y, z.z, 0.0),
        DVec4::new(location.x, location.y, location.z, 1.0),
    )
}

/// Parse IFCDIRECTION to DVec3.
fn parse_direction(dir_id: u64, entities: &HashMap<u64, IfcRawEntity>) -> Option<DVec3> {
    let entity = entities.get(&dir_id)?;
    if entity.type_name != "IFCDIRECTION" { return None; }
    let coords = parse_real_list(&entity.raw_args);
    if coords.len() >= 3 {
        Some(DVec3::new(coords[0], coords[1], coords[2]))
    } else if coords.len() == 2 {
        Some(DVec3::new(coords[0], coords[1], 0.0))
    } else {
        None
    }
}

/// Resolve IFCCARTESIANTRANSFORMATIONOPERATOR3D to a DMat4 transformation matrix.
/// Args: (Axis1, Axis2, LocalOrigin, Scale, Axis3)
/// All args are optional except LocalOrigin.
fn resolve_cartesian_transform_operator(id: u64, entities: &HashMap<u64, IfcRawEntity>) -> DMat4 {
    let entity = match entities.get(&id) {
        Some(e) if e.type_name == "IFCCARTESIANTRANSFORMATIONOPERATOR3D" => e,
        _ => return DMat4::IDENTITY,
    };

    let args = split_ifc_args(&entity.raw_args);

    let axis1 = args.get(0)
        .and_then(|a| extract_single_ref(a))
        .and_then(|did| parse_direction(did, entities))
        .unwrap_or(DVec3::X);

    let axis2 = args.get(1)
        .and_then(|a| extract_single_ref(a))
        .and_then(|did| parse_direction(did, entities))
        .unwrap_or(DVec3::Y);

    let origin = args.get(2)
        .and_then(|a| extract_single_ref(a))
        .and_then(|pid| parse_point(pid, entities))
        .unwrap_or(DVec3::ZERO);

    // Scale (arg 3) - parse as float, default 1.0
    let scale = args.get(3)
        .and_then(|a| {
            let trimmed = a.trim();
            if trimmed == "$" { None }
            else { trimmed.parse::<f64>().ok() }
        })
        .unwrap_or(1.0);

    let axis3 = args.get(4)
        .and_then(|a| extract_single_ref(a))
        .and_then(|did| parse_direction(did, entities))
        .unwrap_or(DVec3::Z);

    let x = axis1.normalize_or_zero() * scale;
    let y = axis2.normalize_or_zero() * scale;
    let z = axis3.normalize_or_zero() * scale;

    DMat4::from_cols(
        DVec4::new(x.x, x.y, x.z, 0.0),
        DVec4::new(y.x, y.y, y.z, 0.0),
        DVec4::new(z.x, z.y, z.z, 0.0),
        DVec4::new(origin.x, origin.y, origin.z, 1.0),
    )
}

/// Apply a 4x4 transform matrix to all face vertices in-place.
fn apply_transform_to_faces(faces: &mut Vec<IfcFaceData>, transform: &DMat4) {
    if *transform == DMat4::IDENTITY { return; }
    for face in faces.iter_mut() {
        transform_points(&mut face.outer, transform);
        for hole in face.holes.iter_mut() {
            transform_points(hole, transform);
        }
    }
}

/// Apply a 4x4 transform to a list of points in-place.
fn transform_points(points: &mut [DVec3], transform: &DMat4) {
    for point in points.iter_mut() {
        let p4 = DVec4::new(point.x, point.y, point.z, 1.0);
        let tp = *transform * p4;
        *point = DVec3::new(tp.x, tp.y, tp.z);
    }
}

// ── Existing geometry resolution (unchanged) ────────────────────────────────

/// Resolve a IFCFACETEDBREP entity to mesh data
fn resolve_faceted_brep(brep_id: u64, entities: &HashMap<u64, IfcRawEntity>) -> Option<IfcMeshData> {
    let brep = entities.get(&brep_id)?;

    // Get shell reference from brep args
    let shell_refs = parse_entity_refs(&brep.raw_args);
    let shell_id = *shell_refs.first()?;

    let shell = entities.get(&shell_id)?;

    // Get face references from shell
    let face_refs = parse_entity_refs(&shell.raw_args);

    // Resolve each face to outer boundary + holes
    let mut faces = Vec::new();
    for face_id in face_refs {
        if let Some(face_data) = resolve_face(face_id, entities) {
            faces.push(face_data);
        }
    }

    if faces.is_empty() {
        return None;
    }

    Some(IfcMeshData {
        name: format!("Brep_{}", brep_id),
        faces,
        placement: None,
        color: None,
    })
}

/// Resolve an IFCFACE to an IfcFaceData with outer boundary and hole boundaries.
/// IFCFACEOUTERBOUND marks the outer loop; IFCFACEBOUND marks inner (hole) loops.
fn resolve_face(face_id: u64, entities: &HashMap<u64, IfcRawEntity>) -> Option<IfcFaceData> {
    let face = entities.get(&face_id)?;

    // Get all bound references for this face
    let bound_refs = parse_entity_refs(&face.raw_args);
    if bound_refs.is_empty() {
        return None;
    }

    let mut outer: Option<Vec<DVec3>> = None;
    let mut holes: Vec<Vec<DVec3>> = Vec::new();

    for bound_id in bound_refs {
        let bound = match entities.get(&bound_id) {
            Some(b) => b,
            None => continue,
        };

        let is_outer = bound.type_name == "IFCFACEOUTERBOUND";

        // Resolve the polyloop from the bound
        let bound_args = split_ifc_args(&bound.raw_args);
        if bound_args.is_empty() {
            continue;
        }
        let loop_id = match extract_single_ref(&bound_args[0]) {
            Some(id) => id,
            None => continue,
        };

        let poly_loop = match entities.get(&loop_id) {
            Some(e) => e,
            None => continue,
        };

        // Get point references from loop
        let point_refs = parse_entity_refs(&poly_loop.raw_args);
        let mut points = Vec::new();
        for pt_id in point_refs {
            if let Some(point) = parse_point(pt_id, entities) {
                points.push(point);
            }
        }

        if points.is_empty() {
            continue;
        }

        // Check orientation flag (.T. or .F.) - second arg of bound
        // If .F., reverse the point order
        if bound_args.len() >= 2 {
            let orient = bound_args[1].trim();
            if orient == ".F." {
                points.reverse();
            }
        }

        if is_outer || outer.is_none() {
            // First bound or explicitly marked as outer
            if let Some(prev_outer) = outer.take() {
                // We had a previous outer that wasn't marked - push it as hole
                holes.push(prev_outer);
            }
            outer = Some(points);
        } else {
            holes.push(points);
        }
    }

    let outer = outer?;
    Some(IfcFaceData { outer, holes })
}

/// Parse IFCCARTESIANPOINT to DVec3
fn parse_point(point_id: u64, entities: &HashMap<u64, IfcRawEntity>) -> Option<DVec3> {
    let entity = entities.get(&point_id)?;

    if entity.type_name != "IFCCARTESIANPOINT" {
        return None;
    }

    let coords = parse_real_list(&entity.raw_args);

    if coords.len() >= 3 {
        Some(DVec3::new(coords[0], coords[1], coords[2]))
    } else {
        None
    }
}

/// Parse entity references from raw args like "(#55,#56,#57,#58)"
pub fn parse_entity_refs(raw_args: &str) -> Vec<u64> {
    let mut refs = Vec::new();
    let mut current_num = String::new();
    let mut in_hash = false;

    for ch in raw_args.chars() {
        if ch == '#' {
            in_hash = true;
            current_num.clear();
        } else if in_hash {
            if ch.is_ascii_digit() {
                current_num.push(ch);
            } else {
                if !current_num.is_empty() {
                    if let Ok(id) = current_num.parse::<u64>() {
                        refs.push(id);
                    }
                    current_num.clear();
                }
                in_hash = false;
            }
        }
    }

    // Handle last number if line ends with digit
    if in_hash && !current_num.is_empty() {
        if let Ok(id) = current_num.parse::<u64>() {
            refs.push(id);
        }
    }

    refs
}

/// Parse comma-separated real numbers from text like "(165379.999999999,22500.,18830.)"
pub fn parse_real_list(text: &str) -> Vec<f64> {
    let mut numbers = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in text.chars() {
        match ch {
            '(' => {
                depth += 1;
                if depth > 1 {
                    current.push(ch);
                }
            }
            ')' => {
                depth -= 1;
                if depth > 0 {
                    current.push(ch);
                } else {
                    // End of list
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        if let Ok(num) = trimmed.parse::<f64>() {
                            numbers.push(num);
                        }
                    }
                }
            }
            ',' => {
                if depth == 1 {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        if let Ok(num) = trimmed.parse::<f64>() {
                            numbers.push(num);
                        }
                    }
                    current.clear();
                } else {
                    current.push(ch);
                }
            }
            _ => {
                if depth > 0 {
                    current.push(ch);
                }
            }
        }
    }

    numbers
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_entity_refs() {
        assert_eq!(parse_entity_refs("(#55,#56,#57,#58)"), vec![55, 56, 57, 58]);
        assert_eq!(parse_entity_refs("(#47)"), vec![47]);
        assert_eq!(parse_entity_refs("(#95)"), vec![95]);
        assert_eq!(parse_entity_refs(""), Vec::<u64>::new());
    }

    #[test]
    fn test_parse_real_list() {
        let coords = parse_real_list("(165379.999999999,22500.,18830.)");
        assert_eq!(coords.len(), 3);
        assert!((coords[0] - 165379.999999999).abs() < 1e-6);
        assert!((coords[1] - 22500.0).abs() < 1e-6);
        assert!((coords[2] - 18830.0).abs() < 1e-6);
    }

    #[test]
    fn test_parse_cartesian_point() {
        let mut entities = HashMap::new();
        entities.insert(47, IfcRawEntity {
            entity_id: 47,
            type_name: "IFCCARTESIANPOINT".to_string(),
            raw_args: "(165379.999999999,22500.,18830.)".to_string(),
        });

        let point = parse_point(47, &entities).unwrap();
        assert!((point.x - 165379.999999999).abs() < 1e-6);
        assert!((point.y - 22500.0).abs() < 1e-6);
        assert!((point.z - 18830.0).abs() < 1e-6);
    }

    #[test]
    fn test_parse_entity_line() {
        let line = "#47= IFCCARTESIANPOINT((165379.999999999,22500.,18830.));";
        let entity = parse_entity_line(line).unwrap();

        assert_eq!(entity.entity_id, 47);
        assert_eq!(entity.type_name, "IFCCARTESIANPOINT");
        assert_eq!(entity.raw_args, "(165379.999999999,22500.,18830.)");
    }

    #[test]
    fn test_handle_missing_entities() {
        let entities = HashMap::new();
        let point = parse_point(999, &entities);
        assert!(point.is_none());

        let face = resolve_face(999, &entities);
        assert!(face.is_none());
    }

    #[test]
    fn test_minimal_ifc_file() {
        let ifc_content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('','2025-03-11T00:00:00',(''),(''),'','','');
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1= IFCCARTESIANPOINT((0.,0.,0.));
#2= IFCCARTESIANPOINT((100.,0.,0.));
#3= IFCCARTESIANPOINT((100.,100.,0.));
#4= IFCCARTESIANPOINT((0.,100.,0.));
#5= IFCPOLYLOOP((#1,#2,#3,#4));
#6= IFCFACEOUTERBOUND(#5,.T.);
#7= IFCFACE((#6));
#8= IFCCLOSEDSHELL((#7));
#9= IFCFACETEDBREP(#8);
ENDSEC;
END-ISO-10303-21;
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(ifc_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = read_ifc_file(temp_file.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].faces.len(), 1);
        assert_eq!(result[0].faces[0].outer.len(), 4);

        // Check first point
        let p0 = result[0].faces[0].outer[0];
        assert!((p0.x - 0.0).abs() < 1e-6);
        assert!((p0.y - 0.0).abs() < 1e-6);
        assert!((p0.z - 0.0).abs() < 1e-6);
    }

    // ── New tests for added functionality ───────────────────────────────

    #[test]
    fn test_split_ifc_args_simple() {
        let args = split_ifc_args("'name',$,#51,(#145),0.5,.NOTDEFINED.");
        assert_eq!(args.len(), 6);
        assert_eq!(args[0], "'name'");
        assert_eq!(args[1], "$");
        assert_eq!(args[2], "#51");
        assert_eq!(args[3], "(#145)");
        assert_eq!(args[4], "0.5");
        assert_eq!(args[5], ".NOTDEFINED.");
    }

    #[test]
    fn test_split_ifc_args_nested_parens() {
        let args = split_ifc_args("$,$,(#101,#102,#103)");
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "$");
        assert_eq!(args[1], "$");
        assert_eq!(args[2], "(#101,#102,#103)");
    }

    #[test]
    fn test_split_ifc_args_string_with_commas() {
        let args = split_ifc_args("'hello, world',$,#10");
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "'hello, world'");
        assert_eq!(args[1], "$");
        assert_eq!(args[2], "#10");
    }

    #[test]
    fn test_extract_single_ref() {
        assert_eq!(extract_single_ref("#51"), Some(51));
        assert_eq!(extract_single_ref(" #123 "), Some(123));
        assert_eq!(extract_single_ref("$"), None);
        assert_eq!(extract_single_ref(""), None);
        assert_eq!(extract_single_ref(".T."), None);
    }

    #[test]
    fn test_parse_direction() {
        let mut entities = HashMap::new();
        entities.insert(10, IfcRawEntity {
            entity_id: 10,
            type_name: "IFCDIRECTION".to_string(),
            raw_args: "(0.,0.,1.)".to_string(),
        });

        let dir = parse_direction(10, &entities).unwrap();
        assert!((dir.x - 0.0).abs() < 1e-6);
        assert!((dir.y - 0.0).abs() < 1e-6);
        assert!((dir.z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_resolve_axis2placement3d_identity() {
        let mut entities = HashMap::new();
        // Origin at 0,0,0 with default axes
        entities.insert(100, IfcRawEntity {
            entity_id: 100,
            type_name: "IFCAXIS2PLACEMENT3D".to_string(),
            raw_args: "#101,$,$".to_string(),
        });
        entities.insert(101, IfcRawEntity {
            entity_id: 101,
            type_name: "IFCCARTESIANPOINT".to_string(),
            raw_args: "(0.,0.,0.)".to_string(),
        });

        let mat = resolve_axis2placement3d(100, &entities);
        // Should be identity
        assert!((mat.col(3).x - 0.0).abs() < 1e-6);
        assert!((mat.col(3).y - 0.0).abs() < 1e-6);
        assert!((mat.col(3).z - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_resolve_axis2placement3d_translated() {
        let mut entities = HashMap::new();
        entities.insert(100, IfcRawEntity {
            entity_id: 100,
            type_name: "IFCAXIS2PLACEMENT3D".to_string(),
            raw_args: "#101,#102,#103".to_string(),
        });
        entities.insert(101, IfcRawEntity {
            entity_id: 101,
            type_name: "IFCCARTESIANPOINT".to_string(),
            raw_args: "(10.,20.,30.)".to_string(),
        });
        entities.insert(102, IfcRawEntity {
            entity_id: 102,
            type_name: "IFCDIRECTION".to_string(),
            raw_args: "(0.,0.,1.)".to_string(),
        });
        entities.insert(103, IfcRawEntity {
            entity_id: 103,
            type_name: "IFCDIRECTION".to_string(),
            raw_args: "(1.,0.,0.)".to_string(),
        });

        let mat = resolve_axis2placement3d(100, &entities);
        // Translation part
        assert!((mat.col(3).x - 10.0).abs() < 1e-6);
        assert!((mat.col(3).y - 20.0).abs() < 1e-6);
        assert!((mat.col(3).z - 30.0).abs() < 1e-6);
    }

    #[test]
    fn test_resolve_placement_chain() {
        let mut entities = HashMap::new();

        // Parent placement: translate by (100, 200, 0)
        entities.insert(10, IfcRawEntity {
            entity_id: 10,
            type_name: "IFCLOCALPLACEMENT".to_string(),
            raw_args: "$,#11".to_string(),
        });
        entities.insert(11, IfcRawEntity {
            entity_id: 11,
            type_name: "IFCAXIS2PLACEMENT3D".to_string(),
            raw_args: "#12,$,$".to_string(),
        });
        entities.insert(12, IfcRawEntity {
            entity_id: 12,
            type_name: "IFCCARTESIANPOINT".to_string(),
            raw_args: "(100.,200.,0.)".to_string(),
        });

        // Child placement: translate by (10, 20, 0) relative to parent
        entities.insert(20, IfcRawEntity {
            entity_id: 20,
            type_name: "IFCLOCALPLACEMENT".to_string(),
            raw_args: "#10,#21".to_string(),
        });
        entities.insert(21, IfcRawEntity {
            entity_id: 21,
            type_name: "IFCAXIS2PLACEMENT3D".to_string(),
            raw_args: "#22,$,$".to_string(),
        });
        entities.insert(22, IfcRawEntity {
            entity_id: 22,
            type_name: "IFCCARTESIANPOINT".to_string(),
            raw_args: "(10.,20.,0.)".to_string(),
        });

        let mat = resolve_placement_chain(20, &entities);
        // Combined: 100+10=110, 200+20=220, 0+0=0
        let test_point = DVec4::new(0.0, 0.0, 0.0, 1.0);
        let result = mat * test_point;
        assert!((result.x - 110.0).abs() < 1e-6);
        assert!((result.y - 220.0).abs() < 1e-6);
        assert!((result.z - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_apply_transform_to_faces() {
        let mut faces = vec![IfcFaceData {
            outer: vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(1.0, 1.0, 0.0),
            ],
            holes: vec![],
        }];

        // Translation by (10, 20, 30)
        let transform = DMat4::from_cols(
            DVec4::new(1.0, 0.0, 0.0, 0.0),
            DVec4::new(0.0, 1.0, 0.0, 0.0),
            DVec4::new(0.0, 0.0, 1.0, 0.0),
            DVec4::new(10.0, 20.0, 30.0, 1.0),
        );

        apply_transform_to_faces(&mut faces, &transform);

        assert!((faces[0].outer[0].x - 10.0).abs() < 1e-6);
        assert!((faces[0].outer[0].y - 20.0).abs() < 1e-6);
        assert!((faces[0].outer[0].z - 30.0).abs() < 1e-6);
        assert!((faces[0].outer[1].x - 11.0).abs() < 1e-6);
        assert!((faces[0].outer[2].y - 21.0).abs() < 1e-6);
    }

    #[test]
    fn test_apply_transform_identity_noop() {
        let original = vec![IfcFaceData {
            outer: vec![DVec3::new(1.0, 2.0, 3.0)],
            holes: vec![],
        }];
        let mut faces = original.clone();
        apply_transform_to_faces(&mut faces, &DMat4::IDENTITY);
        assert!((faces[0].outer[0].x - 1.0).abs() < 1e-6);
        assert!((faces[0].outer[0].y - 2.0).abs() < 1e-6);
        assert!((faces[0].outer[0].z - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_product_with_placement_and_brep() {
        // Full chain: IFCBEAM -> IFCPRODUCTDEFINITIONSHAPE -> IFCSHAPEREPRESENTATION -> IFCFACETEDBREP
        // with IFCLOCALPLACEMENT providing translation
        let ifc_content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('','2025-03-11T00:00:00',(''),(''),'','','');
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1= IFCCARTESIANPOINT((0.,0.,0.));
#2= IFCCARTESIANPOINT((1.,0.,0.));
#3= IFCCARTESIANPOINT((1.,1.,0.));
#4= IFCCARTESIANPOINT((0.,1.,0.));
#5= IFCPOLYLOOP((#1,#2,#3,#4));
#6= IFCFACEOUTERBOUND(#5,.T.);
#7= IFCFACE((#6));
#8= IFCCLOSEDSHELL((#7));
#9= IFCFACETEDBREP(#8);
#10= IFCCARTESIANPOINT((100.,200.,300.));
#11= IFCAXIS2PLACEMENT3D(#10,$,$);
#12= IFCLOCALPLACEMENT($,#11);
#13= IFCSHAPEREPRESENTATION($,'Body','Brep',(#9));
#14= IFCPRODUCTDEFINITIONSHAPE($,$,(#13));
#15= IFCBEAM('guid',#46,'TestBeam','A beam','beamtype',#12,#14,'tag');
ENDSEC;
END-ISO-10303-21;
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(ifc_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = read_ifc_file(temp_file.path()).unwrap();
        assert_eq!(result.len(), 1, "Should find 1 mesh from the beam");

        // The brep vertices should be translated by (100, 200, 300)
        let p0 = result[0].faces[0].outer[0];
        assert!((p0.x - 100.0).abs() < 1e-6, "x={} expected 100", p0.x);
        assert!((p0.y - 200.0).abs() < 1e-6, "y={} expected 200", p0.y);
        assert!((p0.z - 300.0).abs() < 1e-6, "z={} expected 300", p0.z);
    }

    #[test]
    fn test_mapped_item_with_placement() {
        // Test the IFCMAPPEDITEM path:
        // IFCREINFORCINGBAR -> IFCPRODUCTDEFINITIONSHAPE -> IFCSHAPEREPRESENTATION
        //   -> IFCMAPPEDITEM(#repmap, #cartop) -> IFCREPRESENTATIONMAP(#origin, #shaperep)
        //   -> IFCSHAPEREPRESENTATION -> IFCFACETEDBREP
        let ifc_content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('','2025-03-11T00:00:00',(''),(''),'','','');
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1= IFCCARTESIANPOINT((0.,0.,0.));
#2= IFCCARTESIANPOINT((1.,0.,0.));
#3= IFCCARTESIANPOINT((1.,1.,0.));
#4= IFCPOLYLOOP((#1,#2,#3));
#5= IFCFACEOUTERBOUND(#4,.T.);
#6= IFCFACE((#5));
#7= IFCCLOSEDSHELL((#6));
#8= IFCFACETEDBREP(#7);
#9= IFCAXIS2PLACEMENT3D(#1,$,$);
#10= IFCSHAPEREPRESENTATION($,'Body','Brep',(#8));
#11= IFCREPRESENTATIONMAP(#9,#10);
#20= IFCCARTESIANPOINT((50.,60.,70.));
#21= IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#20,$,$);
#22= IFCMAPPEDITEM(#11,#21);
#23= IFCSHAPEREPRESENTATION($,'Body','MappedRepresentation',(#22));
#24= IFCPRODUCTDEFINITIONSHAPE($,$,(#23));
#30= IFCCARTESIANPOINT((0.,0.,0.));
#31= IFCAXIS2PLACEMENT3D(#30,$,$);
#32= IFCLOCALPLACEMENT($,#31);
#33= IFCREINFORCINGBAR('guid',#46,'Rebar1',$,$,#32,#24,'tag',$,19.,0.,$,.NOTDEFINED.,$);
ENDSEC;
END-ISO-10303-21;
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(ifc_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = read_ifc_file(temp_file.path()).unwrap();
        assert_eq!(result.len(), 1, "Should find 1 mesh from the rebar mapped item");

        // The triangle vertices should be translated by (50, 60, 70) from the CartesianTransformOp
        let p0 = result[0].faces[0].outer[0];
        assert!((p0.x - 50.0).abs() < 1e-6, "x={} expected 50", p0.x);
        assert!((p0.y - 60.0).abs() < 1e-6, "y={} expected 60", p0.y);
        assert!((p0.z - 70.0).abs() < 1e-6, "z={} expected 70", p0.z);
    }
}
