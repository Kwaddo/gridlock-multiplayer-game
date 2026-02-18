use gltf::animation::util::ReadOutputs;
use gltf::mesh::util::ReadIndices;
use macroquad::models::{Mesh, Vertex};
use macroquad::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipSelectionMode {
    LongestDuration,
    MostMotion,
    LeastMotion,
}

#[derive(Clone, Copy)]
struct LocalTransform {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

impl LocalTransform {
    fn to_mat4(self) -> Mat4 {
        Mat4::from_translation(self.translation)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_scale(self.scale)
    }
}

#[derive(Clone)]
struct KeyframesVec3 {
    times: Vec<f32>,
    values: Vec<Vec3>,
}

#[derive(Clone)]
struct KeyframesQuat {
    times: Vec<f32>,
    values: Vec<Quat>,
}

#[derive(Default, Clone)]
struct NodeAnimationTrack {
    translation: Option<KeyframesVec3>,
    rotation: Option<KeyframesQuat>,
    scale: Option<KeyframesVec3>,
}

#[derive(Clone, Copy)]
struct SkinVertex {
    bind_position: Vec3,
    bind_normal: Vec3,
    joints: [u16; 4],
    weights: [f32; 4],
}

pub struct AnimatedModel {
    mesh: Mesh,
    skin_vertices: Vec<SkinVertex>,
    node_rest: Vec<LocalTransform>,
    node_pose: Vec<LocalTransform>,
    node_parents: Vec<Option<usize>>,
    node_global: Vec<Mat4>,
    node_global_valid: Vec<bool>,
    node_tracks: Vec<NodeAnimationTrack>,
    joint_nodes: Vec<usize>,
    inverse_bind: Vec<Mat4>,
    skin_mats: Vec<Mat4>,
    duration: f32,
    time: f32,
    skin_accum: f32,
    /// Uniform scale to apply at draw time so the model is ~1.6 units tall (player height).
    pub normalize_scale: f32,
    /// Y offset so that the model's feet sit at y=0 after scaling.
    pub y_offset: f32,
}

const SKIN_INTERVAL: f32 = 1.0 / 30.0; // skin at 30fps max

impl AnimatedModel {
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    pub fn update(&mut self, dt: f32) {
        if self.duration > 0.0 {
            self.time = (self.time + dt) % self.duration;
        }

        // Only reskin vertices at ~30fps to save CPU
        self.skin_accum += dt;
        if self.skin_accum < SKIN_INTERVAL {
            return;
        }
        self.skin_accum = 0.0;

        for (i, pose) in self.node_pose.iter_mut().enumerate() {
            let mut p = self.node_rest[i];
            if let Some(track) = &self.node_tracks[i].translation {
                p.translation = sample_vec3(track, self.time);
            }
            if let Some(track) = &self.node_tracks[i].rotation {
                p.rotation = sample_quat(track, self.time);
            }
            if let Some(track) = &self.node_tracks[i].scale {
                p.scale = sample_vec3(track, self.time);
            }
            *pose = p;
            self.node_global_valid[i] = false;
        }

        for i in 0..self.joint_nodes.len() {
            let node_idx = self.joint_nodes[i];
            let global = self.global_for_node(node_idx);
            self.skin_mats[i] = global * self.inverse_bind[i];
        }

        for (i, vtx) in self.skin_vertices.iter().enumerate() {
            let mut p = Vec3::ZERO;
            let mut n = Vec3::ZERO;
            let mut total_w = 0.0;
            let bind_p = vtx.bind_position.extend(1.0);
            let bind_n = vtx.bind_normal.extend(0.0);

            for j in 0..4 {
                let w = vtx.weights[j];
                if w <= 0.0 {
                    continue;
                }
                let joint_idx = vtx.joints[j] as usize;
                if joint_idx >= self.skin_mats.len() {
                    continue;
                }
                let m = self.skin_mats[joint_idx];
                p += (m * bind_p).truncate() * w;
                n += (m * bind_n).truncate() * w;
                total_w += w;
            }

            let out_p = if total_w > 0.0 {
                p
            } else {
                vtx.bind_position
            };
            let out_n = if n.length_squared() > 0.0 {
                n.normalize()
            } else {
                vtx.bind_normal
            };
            self.mesh.vertices[i].position = out_p;
            self.mesh.vertices[i].normal = out_n.extend(0.0);
        }
    }

    fn global_for_node(&mut self, node_idx: usize) -> Mat4 {
        if self.node_global_valid[node_idx] {
            return self.node_global[node_idx];
        }
        let local = self.node_pose[node_idx].to_mat4();
        let global = if let Some(parent) = self.node_parents[node_idx] {
            self.global_for_node(parent) * local
        } else {
            local
        };
        self.node_global[node_idx] = global;
        self.node_global_valid[node_idx] = true;
        global
    }
}

fn sample_interval(times: &[f32], t: f32) -> (usize, usize, f32) {
    if times.len() <= 1 {
        return (0, 0, 0.0);
    }
    if t <= times[0] {
        return (0, 0, 0.0);
    }
    let last = times.len() - 1;
    if t >= times[last] {
        return (last, last, 0.0);
    }
    let mut lo = 0usize;
    let mut hi = last;
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if times[mid] <= t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let t0 = times[lo];
    let t1 = times[hi];
    let alpha = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
    (lo, hi, alpha.clamp(0.0, 1.0))
}

fn sample_vec3(track: &KeyframesVec3, t: f32) -> Vec3 {
    let (i0, i1, a) = sample_interval(&track.times, t);
    if i0 == i1 {
        return track.values[i0];
    }
    track.values[i0].lerp(track.values[i1], a)
}

fn sample_quat(track: &KeyframesQuat, t: f32) -> Quat {
    let (i0, i1, a) = sample_interval(&track.times, t);
    if i0 == i1 {
        return track.values[i0];
    }
    track.values[i0].slerp(track.values[i1], a)
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

pub fn load_gltf_animated_model(path: &str) -> Result<AnimatedModel, String> {
    load_gltf_animated_model_with_mode(path, ClipSelectionMode::LongestDuration)
}

pub fn load_gltf_animated_model_with_mode(
    path: &str,
    mode: ClipSelectionMode,
) -> Result<AnimatedModel, String> {
    let (doc, buffers, images) = gltf::import(path)
        .map_err(|e| format!("failed to import gltf '{}': {e}", path))?;

    let mesh_count = doc.meshes().count();
    let skin_count = doc.skins().count();
    let anim_count = doc.animations().count();
    println!(
        "[anim] {} meshes={} skins={} animations={}",
        path, mesh_count, skin_count, anim_count
    );

    let skin = doc
        .skins()
        .next()
        .ok_or_else(|| format!("no skin found in '{}'", path))?;
    // Collect ALL skinned primitives across all meshes
    let mut skinned_prims: Vec<(gltf::Mesh, gltf::Primitive)> = Vec::new();
    for mesh in doc.meshes() {
        for prim in mesh.primitives() {
            let r = prim.reader(|buffer| Some(&buffers[buffer.index()]));
            if r.read_positions().is_some() && r.read_joints(0).is_some() && r.read_weights(0).is_some() {
                skinned_prims.push((mesh.clone(), prim));
            }
        }
    }
    if skinned_prims.is_empty() {
        return Err(format!("no skinned primitive (POSITION+JOINTS_0+WEIGHTS_0) found in '{}'", path));
    }
    let mesh = skinned_prims[0].0.clone();
    let first_prim_index = skinned_prims[0].1.index();
    let file_stem = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let stem_tokens: Vec<String> = file_stem
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();

    let mut selected_anim = None;
    let mut best_name_score = -1i32;
    let mut best_metric = f32::NEG_INFINITY;
    let mut best_duration = f32::NEG_INFINITY;
    for anim in doc.animations() {
        let mut has_channels = false;
        let mut first_t = f32::INFINITY;
        let mut last_t = f32::NEG_INFINITY;
        let mut motion_score = 0.0f32;
        for channel in anim.channels() {
            has_channels = true;
            let r = channel.reader(|buffer| Some(&buffers[buffer.index()]));
            if let Some(mut inputs) = r.read_inputs() {
                if let Some(first) = inputs.next() {
                    let mut last = first;
                    for t in inputs {
                        last = t;
                    }
                    first_t = first_t.min(first);
                    last_t = last_t.max(last);
                }
            }
            if let Some(ReadOutputs::Translations(v)) = r.read_outputs() {
                let mut prev: Option<Vec3> = None;
                for t in v {
                    let cur = vec3(t[0], t[1], t[2]);
                    if let Some(p) = prev {
                        motion_score += (cur - p).length();
                    }
                    prev = Some(cur);
                }
            }
        }
        let clip_duration = if first_t.is_finite() && last_t.is_finite() && last_t > first_t {
            last_t - first_t
        } else {
            0.0
        };
        let anim_name = anim.name().unwrap_or("").to_lowercase();
        let mut name_score = 0i32;
        if !anim_name.is_empty() && !stem_tokens.is_empty() {
            for t in &stem_tokens {
                if anim_name.contains(t) {
                    name_score += 1;
                }
            }
        }
        let metric = match mode {
            ClipSelectionMode::MostMotion => motion_score,
            ClipSelectionMode::LeastMotion => -motion_score,
            ClipSelectionMode::LongestDuration => clip_duration,
        };
        println!(
            "[anim] {} candidate anim={} name='{}' name_score={} duration={} motion={}",
            path,
            anim.index(),
            anim.name().unwrap_or(""),
            name_score,
            clip_duration,
            motion_score
        );
        if has_channels
            && (name_score > best_name_score
                || (name_score == best_name_score && metric > best_metric)
                || (name_score == best_name_score
                    && (metric - best_metric).abs() <= f32::EPSILON
                    && clip_duration > best_duration))
        {
            best_name_score = name_score;
            best_metric = metric;
            best_duration = clip_duration;
            selected_anim = Some(anim);
        }
    }
    let anim = selected_anim.ok_or_else(|| format!("no usable animation clip found in '{}'", path))?;

    println!(
        "[anim] {} using mesh={} first_prim={} anim={}",
        path,
        mesh.index(),
        first_prim_index,
        anim.index()
    );

    // Merge ALL skinned primitives into combined vertex/index buffers
    let mut all_positions: Vec<Vec3> = Vec::new();
    let mut all_normals: Vec<Vec3> = Vec::new();
    let mut all_uvs: Vec<Vec2> = Vec::new();
    let mut all_joints: Vec<[u16; 4]> = Vec::new();
    let mut all_weights: Vec<[f32; 4]> = Vec::new();
    let mut all_indices: Vec<u16> = Vec::new();
    let mut all_colors: Vec<[u8; 4]> = Vec::new();
    let mut mesh_texture: Option<Texture2D> = None;

    println!("[anim] '{}' merging {} skinned primitives", path, skinned_prims.len());

    for (_mesh_ref, sp) in &skinned_prims {
        let reader = sp.reader(|buffer| Some(&buffers[buffer.index()]));
        let positions: Vec<Vec3> = reader
            .read_positions()
            .unwrap()
            .map(|p| vec3(p[0], p[1], p[2]))
            .collect();
        let normals: Vec<Vec3> = reader
            .read_normals()
            .map(|it| it.map(|n| vec3(n[0], n[1], n[2])).collect())
            .unwrap_or_else(|| vec![vec3(0.0, 1.0, 0.0); positions.len()]);
        let uvs: Vec<Vec2> = reader
            .read_tex_coords(0)
            .map(|uv| uv.into_f32().map(|u| vec2(u[0], u[1])).collect())
            .unwrap_or_else(|| vec![Vec2::ZERO; positions.len()]);
        let joints: Vec<[u16; 4]> = reader
            .read_joints(0)
            .unwrap()
            .into_u16()
            .collect();
        let mut weights: Vec<[f32; 4]> = reader
            .read_weights(0)
            .unwrap()
            .into_f32()
            .collect();

        if normals.len() != positions.len()
            || uvs.len() != positions.len()
            || joints.len() != positions.len()
            || weights.len() != positions.len()
        {
            return Err(format!("vertex attribute length mismatch in prim {} of '{}'", sp.index(), path));
        }

        for w in &mut weights {
            let sum = w[0] + w[1] + w[2] + w[3];
            if sum > 0.0001 {
                w[0] /= sum;
                w[1] /= sum;
                w[2] /= sum;
                w[3] /= sum;
            } else {
                *w = [1.0, 0.0, 0.0, 0.0];
            }
        }

        // Load texture from first primitive that has one
        if mesh_texture.is_none() {
            if let Some(base_color) = sp
                .material()
                .pbr_metallic_roughness()
                .base_color_texture()
            {
                let image_index = base_color.texture().source().index();
                if let Some(image) = images.get(image_index) {
                    let rgba = gltf_image_to_rgba8(image)?;
                    if image.width <= u16::MAX as u32 && image.height <= u16::MAX as u32 {
                        let tex = Texture2D::from_rgba8(image.width as u16, image.height as u16, &rgba);
                        tex.set_filter(FilterMode::Nearest);
                        mesh_texture = Some(tex);
                    }
                }
            }
        }

        let f = sp.material().pbr_metallic_roughness().base_color_factor();
        let mat_color = [
            (f[0] * 255.0) as u8,
            (f[1] * 255.0) as u8,
            (f[2] * 255.0) as u8,
            (f[3] * 255.0) as u8,
        ];

        // Read indices with offset for merged vertex buffer
        let base_vertex = all_positions.len() as u16;
        let prim_indices: Vec<u16> = match reader.read_indices() {
            Some(ReadIndices::U8(it)) => it.map(|i| i as u16 + base_vertex).collect(),
            Some(ReadIndices::U16(it)) => it.map(|i| i + base_vertex).collect(),
            Some(ReadIndices::U32(it)) => {
                let mut out = Vec::new();
                for i in it {
                    let shifted = i as u32 + base_vertex as u32;
                    if shifted > u16::MAX as u32 {
                        return Err(format!("merged index {} exceeds u16 in '{}'", shifted, path));
                    }
                    out.push(shifted as u16);
                }
                out
            }
            None => {
                let mut out = Vec::with_capacity(positions.len());
                for i in 0..positions.len() {
                    let shifted = i as u32 + base_vertex as u32;
                    if shifted > u16::MAX as u32 {
                        return Err(format!("merged vertex index {} exceeds u16 in '{}'", shifted, path));
                    }
                    out.push(shifted as u16);
                }
                out
            }
        };

        all_colors.extend(vec![mat_color; positions.len()]);
        all_positions.extend_from_slice(&positions);
        all_normals.extend_from_slice(&normals);
        all_uvs.extend_from_slice(&uvs);
        all_joints.extend_from_slice(&joints);
        all_weights.extend(weights);
        all_indices.extend_from_slice(&prim_indices);
    }

    let runtime_vertices: Vec<Vertex> = all_positions
        .iter()
        .zip(all_normals.iter())
        .zip(all_uvs.iter())
        .zip(all_colors.iter())
        .map(|(((p, n), uv), col)| Vertex {
            position: *p,
            uv: *uv,
            color: *col,
            normal: n.extend(0.0),
        })
        .collect();
    let skin_vertices: Vec<SkinVertex> = all_positions
        .iter()
        .zip(all_normals.iter())
        .zip(all_joints.iter())
        .zip(all_weights.iter())
        .map(|(((p, n), j), w)| SkinVertex {
            bind_position: *p,
            bind_normal: *n,
            joints: *j,
            weights: *w,
        })
        .collect();

    println!("[anim] '{}' total vertices={} indices={}", path, runtime_vertices.len(), all_indices.len());

    let node_count = doc.nodes().len();
    let mut node_parents = vec![None; node_count];
    for node in doc.nodes() {
        for child in node.children() {
            node_parents[child.index()] = Some(node.index());
        }
    }

    let mut node_rest = vec![
        LocalTransform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        node_count
    ];
    for node in doc.nodes() {
        let idx = node.index();
        let (t, r, s) = node.transform().decomposed();
        node_rest[idx] = LocalTransform {
            translation: vec3(t[0], t[1], t[2]),
            rotation: Quat::from_xyzw(r[0], r[1], r[2], r[3]).normalize(),
            scale: vec3(s[0], s[1], s[2]),
        };
    }

    let mut node_tracks = vec![NodeAnimationTrack::default(); node_count];
    let mut duration = 0.0f32;
    let mut track_count = 0usize;
    for channel in anim.channels() {
        let node_idx = channel.target().node().index();
        let r = channel.reader(|buffer| Some(&buffers[buffer.index()]));
        let times: Vec<f32> = match r.read_inputs() {
            Some(t) => t.collect(),
            None => continue,
        };
        if times.is_empty() {
            continue;
        }
        if let Some(last) = times.last() {
            duration = duration.max(*last);
        }
        let outputs = match r.read_outputs() {
            Some(o) => o,
            None => continue,
        };
        match outputs {
            ReadOutputs::Translations(v) => {
                let vals: Vec<Vec3> = v.map(|x| vec3(x[0], x[1], x[2])).collect();
                node_tracks[node_idx].translation = Some(KeyframesVec3 { times, values: vals });
                track_count += 1;
            }
            ReadOutputs::Rotations(v) => {
                let vals: Vec<Quat> = v
                    .into_f32()
                    .map(|x| Quat::from_xyzw(x[0], x[1], x[2], x[3]).normalize())
                    .collect();
                node_tracks[node_idx].rotation = Some(KeyframesQuat { times, values: vals });
                track_count += 1;
            }
            ReadOutputs::Scales(v) => {
                let vals: Vec<Vec3> = v.map(|x| vec3(x[0], x[1], x[2])).collect();
                node_tracks[node_idx].scale = Some(KeyframesVec3 { times, values: vals });
                track_count += 1;
            }
            ReadOutputs::MorphTargetWeights(_) => {}
        }
    }
    println!(
        "[anim] {} clip_duration={} track_count={}",
        path, duration, track_count
    );
    if duration <= 0.0 || track_count == 0 {
        return Err(format!(
            "animation clip has no effective tracks in '{}'",
            path
        ));
    }

    let joint_nodes: Vec<usize> = skin.joints().map(|j| j.index()).collect();
    let inverse_bind: Vec<Mat4> = match skin
        .reader(|buffer| Some(&buffers[buffer.index()]))
        .read_inverse_bind_matrices()
    {
        Some(mats) => mats.map(|m| Mat4::from_cols_array_2d(&m)).collect(),
        None => vec![Mat4::IDENTITY; joint_nodes.len()],
    };

    if inverse_bind.len() != joint_nodes.len() {
        return Err(format!(
            "inverse bind matrix count {} != joint count {} in '{}'",
            inverse_bind.len(),
            joint_nodes.len(),
            path
        ));
    }

    // Compute bounding box from bind pose to determine scale/offset
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for p in &all_positions {
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    let raw_height = (max_y - min_y).max(0.0001);
    let target_height = 1.5_f32; // slightly shorter than eye height (1.6) so players look right
    let normalize_scale = target_height / raw_height;
    let y_offset = -min_y * normalize_scale; // shift feet to y=0

    let mut model = AnimatedModel {
        mesh: Mesh {
            vertices: runtime_vertices,
            indices: all_indices,
            texture: mesh_texture,
        },
        skin_vertices,
        node_pose: node_rest.clone(),
        node_rest,
        node_parents,
        node_global: vec![Mat4::IDENTITY; node_count],
        node_global_valid: vec![false; node_count],
        node_tracks,
        joint_nodes,
        inverse_bind,
        skin_mats: vec![Mat4::IDENTITY; skin.joints().len()],
        duration,
        time: 0.0,
        skin_accum: SKIN_INTERVAL, // force first update
        normalize_scale,
        y_offset,
    };
    model.update(0.0);

    println!(
        "[anim] {} raw_height={:.1} scale={:.4} y_offset={:.2}",
        path, raw_height, normalize_scale, y_offset
    );

    Ok(model)
}
