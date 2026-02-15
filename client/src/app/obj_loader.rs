use std::collections::HashMap;

use gltf::mesh::util::ReadIndices;
use macroquad::prelude::*;

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

fn gltf_image_to_rgba8(image: &gltf::image::Data) -> Result<Vec<u8>, String> {
    use gltf::image::Format;

    let pixels = &image.pixels;
    let out = match image.format {
        Format::R8G8B8A8 => pixels.clone(),
        Format::R8G8B8 => {
            let mut out = Vec::with_capacity((image.width * image.height * 4) as usize);
            for chunk in pixels.chunks_exact(3) {
                out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            out
        }
        Format::R8G8 => {
            let mut out = Vec::with_capacity((image.width * image.height * 4) as usize);
            for chunk in pixels.chunks_exact(2) {
                out.extend_from_slice(&[chunk[0], chunk[1], 0, 255]);
            }
            out
        }
        Format::R8 => {
            let mut out = Vec::with_capacity((image.width * image.height * 4) as usize);
            for &v in pixels {
                out.extend_from_slice(&[v, v, v, 255]);
            }
            out
        }
        other => {
            return Err(format!("unsupported gltf image format: {:?}", other));
        }
    };

    Ok(out)
}

pub async fn load_gltf_static_mesh(path: &str) -> Result<Mesh, String> {
    let (doc, buffers, images) = gltf::import(path)
        .map_err(|e| format!("failed to import gltf '{}': {e}", path))?;

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    let mut mesh_texture: Option<Texture2D> = None;

    for mesh in doc.meshes() {
        for prim in mesh.primitives() {
            let reader = prim.reader(|buffer| Some(&buffers[buffer.index()]));

            let f = prim.material().pbr_metallic_roughness().base_color_factor();
            let mat_color = [
                (f[0] * 255.0) as u8,
                (f[1] * 255.0) as u8,
                (f[2] * 255.0) as u8,
                (f[3] * 255.0) as u8,
            ];

            if mesh_texture.is_none() {
                if let Some(base_color) = prim
                    .material()
                    .pbr_metallic_roughness()
                    .base_color_texture()
                {
                    let image_index = base_color.texture().source().index();
                    if let Some(image) = images.get(image_index) {
                        let rgba = gltf_image_to_rgba8(image)?;
                        if image.width > u16::MAX as u32 || image.height > u16::MAX as u32 {
                            return Err(format!(
                                "texture too large for macroquad: {}x{}",
                                image.width, image.height
                            ));
                        }
                        let tex = Texture2D::from_rgba8(
                            image.width as u16,
                            image.height as u16,
                            &rgba,
                        );
                        tex.set_filter(FilterMode::Nearest);
                        mesh_texture = Some(tex);
                    }
                }
            }

            let Some(pos_iter) = reader.read_positions() else {
                continue;
            };

            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|n| n.collect())
                .unwrap_or_default();
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|uv| uv.into_f32().collect())
                .unwrap_or_default();

            let base = vertices.len() as u32;
            for (i, p) in pos_iter.enumerate() {
                let n = normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]);
                let uv = uvs.get(i).copied().unwrap_or([0.0, 0.0]);
                vertices.push(Vertex {
                    position: vec3(p[0], p[1], p[2]),
                    uv: vec2(uv[0], uv[1]),
                    color: mat_color,
                    normal: vec4(n[0], n[1], n[2], 0.0),
                });
            }

            if let Some(read_idx) = reader.read_indices() {
                match read_idx {
                    ReadIndices::U8(it) => {
                        for i in it {
                            let idx = base + i as u32;
                            if idx > u16::MAX as u32 {
                                return Err("too many vertices".into());
                            }
                            indices.push(idx as u16);
                        }
                    }
                    ReadIndices::U16(it) => {
                        for i in it {
                            let idx = base + i as u32;
                            if idx > u16::MAX as u32 {
                                return Err("too many vertices".into());
                            }
                            indices.push(idx as u16);
                        }
                    }
                    ReadIndices::U32(it) => {
                        for i in it {
                            let idx = base + i;
                            if idx > u16::MAX as u32 {
                                return Err("too many vertices".into());
                            }
                            indices.push(idx as u16);
                        }
                    }
                }
            } else {
                let added = (vertices.len() as u32) - base;
                for i in 0..added {
                    let idx = base + i;
                    if idx > u16::MAX as u32 {
                        return Err("too many vertices".into());
                    }
                    indices.push(idx as u16);
                }
            }
        }
    }

    if vertices.is_empty() || indices.is_empty() {
        return Err("GLB produced empty mesh".into());
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

    let size = max - min;
    let current_height = size.y.max(0.0001);
    let target_height = 1.6_f32;
    let scale = target_height / current_height;

    let center_x = (min.x + max.x) * 0.5;
    let center_z = (min.z + max.z) * 0.5;

    for v in &mut vertices {
        v.position.x = (v.position.x - center_x) * scale;
        v.position.y = (v.position.y - min.y) * scale;
        v.position.z = (v.position.z - center_z) * scale;
    }

    Ok(Mesh {
        vertices,
        indices,
        texture: mesh_texture,
    })
}
