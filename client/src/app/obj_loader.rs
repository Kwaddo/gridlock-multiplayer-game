use macroquad::models::Vertex;
use macroquad::prelude::*;
use std::collections::HashMap;

fn resolve_sibling_path(base_path: &str, sibling_name: &str) -> String {
    if let Some((dir, _)) = base_path.rsplit_once('/') {
        format!("{}/{}", dir, sibling_name)
    } else {
        sibling_name.to_string()
    }
}

fn parse_obj_index(raw: &str, len: usize) -> Option<usize> {
    if raw.is_empty() {
        return None;
    }
    let idx = raw.parse::<i32>().ok()?;
    if idx > 0 {
        Some((idx - 1) as usize)
    } else if idx < 0 {
        let resolved = len as i32 + idx;
        (resolved >= 0).then_some(resolved as usize)
    } else {
        None
    }
}

fn parse_face_ref(token: &str, pos_len: usize, norm_len: usize) -> Option<(usize, Option<usize>)> {
    let mut parts = token.split('/');
    let v = parts.next().unwrap_or("");
    let _vt = parts.next();
    let vn = parts.next();

    let v_idx = parse_obj_index(v, pos_len)?;
    let n_idx = vn.and_then(|s| parse_obj_index(s, norm_len));
    Some((v_idx, n_idx))
}

fn parse_mtl_diffuse_colors(mtl_text: &str) -> HashMap<String, [u8; 4]> {
    fn boost_channel(v: f32) -> u8 {
        let boosted = (v.clamp(0.0, 1.0).powf(0.6) * 1.65).clamp(0.0, 1.0);
        (boosted * 255.0) as u8
    }

    let mut colors = HashMap::new();
    let mut current_material: Option<String> = None;

    for line in mtl_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("newmtl ") {
            current_material = Some(rest.trim().to_string());
            continue;
        }

        if let Some(rest) = line.strip_prefix("Kd ") {
            if let Some(name) = &current_material {
                let comps: Vec<&str> = rest.split_whitespace().collect();
                if comps.len() >= 3 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        comps[0].parse::<f32>(),
                        comps[1].parse::<f32>(),
                        comps[2].parse::<f32>(),
                    ) {
                        colors.insert(
                            name.clone(),
                            [boost_channel(r), boost_channel(g), boost_channel(b), 255],
                        );
                    }
                }
            }
        }
    }

    colors
}

pub async fn load_obj_mesh_with_materials(obj_path: &str) -> Result<Mesh, String> {
    let obj_text = load_string(obj_path)
        .await
        .map_err(|e| format!("failed to read OBJ '{}': {:?}", obj_path, e))?;

    let mut positions: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut material_colors: HashMap<String, [u8; 4]> = HashMap::new();
    let mut current_color: [u8; 4] = [220, 220, 220, 255];

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();

    for line in obj_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("mtllib ") {
            let mtl_name = rest.trim();
            let candidates = [
                resolve_sibling_path(obj_path, mtl_name),
                mtl_name.to_string(),
            ];
            for candidate in &candidates {
                if let Ok(mtl_text) = load_string(candidate).await {
                    material_colors.extend(parse_mtl_diffuse_colors(&mtl_text));
                    break;
                }
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("usemtl ") {
            let mat_name = rest.trim();
            if let Some(color) = material_colors.get(mat_name) {
                current_color = *color;
            }
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "v" if parts.len() >= 4 => {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                ) {
                    positions.push(vec3(x, y, z));
                }
            }
            "vn" if parts.len() >= 4 => {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                ) {
                    let n = vec3(x, y, z);
                    let n = if n.length_squared() > 0.0 {
                        n.normalize()
                    } else {
                        vec3(0.0, 1.0, 0.0)
                    };
                    normals.push(n);
                }
            }
            "f" if parts.len() >= 4 => {
                let refs: Vec<(usize, Option<usize>)> = parts[1..]
                    .iter()
                    .filter_map(|token| parse_face_ref(token, positions.len(), normals.len()))
                    .collect();

                if refs.len() < 3 {
                    continue;
                }

                for tri_i in 1..(refs.len() - 1) {
                    let tri = [refs[0], refs[tri_i], refs[tri_i + 1]];

                    let p0 = positions.get(tri[0].0).copied().unwrap_or(Vec3::ZERO);
                    let p1 = positions.get(tri[1].0).copied().unwrap_or(Vec3::ZERO);
                    let p2 = positions.get(tri[2].0).copied().unwrap_or(Vec3::ZERO);
                    let computed = (p1 - p0).cross(p2 - p0);
                    let face_normal = if computed.length_squared() > 0.0 {
                        computed.normalize()
                    } else {
                        vec3(0.0, 1.0, 0.0)
                    };

                    for (v_idx, n_idx_opt) in tri {
                        let pos = match positions.get(v_idx) {
                            Some(p) => *p,
                            None => continue,
                        };
                        let normal = n_idx_opt
                            .and_then(|n_idx| normals.get(n_idx).copied())
                            .unwrap_or(face_normal);

                        if vertices.len() >= u16::MAX as usize {
                            return Err(format!(
                                "OBJ '{}' has too many generated vertices for u16 index buffer",
                                obj_path
                            ));
                        }
                        let idx = vertices.len() as u16;
                        vertices.push(Vertex {
                            position: pos,
                            uv: Vec2::ZERO,
                            color: current_color,
                            normal: normal.extend(0.0),
                        });
                        indices.push(idx);
                    }
                }
            }
            _ => {}
        }
    }

    if vertices.is_empty() || indices.is_empty() {
        return Err(format!("OBJ '{}' produced empty mesh", obj_path));
    }

    let mut min = vec3(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = vec3(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for v in &vertices {
        min.x = min.x.min(v.position.x);
        min.y = min.y.min(v.position.y);
        min.z = min.z.min(v.position.z);
        max.x = max.x.max(v.position.x);
        max.y = max.y.max(v.position.y);
        max.z = max.z.max(v.position.z);
    }

    let center = (min + max) * 0.5;
    let size = max - min;
    let max_extent = size.x.max(size.y).max(size.z).max(0.0001);
    for v in &mut vertices {
        v.position = (v.position - center) / max_extent;
    }

    Ok(Mesh {
        vertices,
        indices,
        texture: None,
    })
}
