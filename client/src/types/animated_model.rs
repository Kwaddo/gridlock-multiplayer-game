use gltf::animation::util::ReadOutputs;
use gltf::mesh::util::ReadIndices;
use macroquad::models::{Mesh, Vertex};
use macroquad::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::env;
use std::rc::{Rc, Weak};

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

#[derive(Clone, Copy)]
struct RigidVertex {
    bind_position: Vec3,
    bind_normal: Vec3,
    node_idx: usize,
}

pub struct AnimatedModel {
    /// Bind-pose mesh, skin weights and textures, shared by every clip of the
    /// same character (the player's 25 animation files hold one mesh).
    geometry: Rc<SharedGeometry>,
    /// This clip's CPU-skinned meshes, made the first time it is skinned on
    /// the CPU; clips drawn by the GPU, or not drawn at all, never need them.
    skinned: Option<Vec<Mesh>>,
    /// Whether the last draw skinned on the GPU (see `draw_gpu`); then only
    /// the joint matrices are updated here, every frame.
    gpu_drawn: Cell<bool>,
    node_rest: Vec<LocalTransform>,
    node_pose: Vec<LocalTransform>,
    node_parents: Vec<Option<usize>>,
    node_global: Vec<Mat4>,
    node_global_valid: Vec<bool>,
    node_tracks: Vec<NodeAnimationTrack>,
    joint_nodes: Vec<usize>,
    joint_names: Vec<String>,
    // here we have joint positions, we cache them every frame so we can detect hitboxes precisely everytime the animation moves
    joint_positions: Vec<Vec3>,
    inverse_bind: Vec<Mat4>,
    skin_mats: Vec<Mat4>,
    duration: f32,
    time: f32,
    skin_accum: f32,
    playback_loop: bool,
    playback_paused: bool,
    playback_finished: bool,
    playback_start: f32,
    playback_end: f32,
    /// Uniform scale to apply at draw time so the model is ~1.5 units tall.
    pub normalize_scale: f32,
    /// Y offset so that the model's feet sit at y=0 after scaling.
    pub y_offset: f32,
}

const SKIN_INTERVAL: f32 = 1.0 / 30.0; // skin at 30fps max

struct AnimatedPart {
    /// Bind pose; also the mesh drawn until the clip is first skinned.
    mesh: Mesh,
    skin_vertices: Vec<SkinVertex>,
    rigid_vertices: Vec<RigidVertex>,
    skinned_vertex_count: usize,
}

/// The parts of an animated model that do not depend on its clip.
struct SharedGeometry {
    parts: Vec<AnimatedPart>,
    /// GPU buffers for skinning in a shader, made on first use.
    gpu: RefCell<GpuGeometry>,
}

thread_local! {
    /// Geometry by a hash of its content, so identical meshes are kept once.
    static GEOMETRY: RefCell<HashMap<u64, Weak<SharedGeometry>>> = RefCell::new(HashMap::new());
}

impl SharedGeometry {
    /// The geometry already loaded with the same content as `parts`, or
    /// `parts` as new shared geometry.
    fn shared(parts: Vec<AnimatedPart>) -> Rc<SharedGeometry> {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for part in &parts {
            part.mesh.indices.hash(&mut hasher);
            for v in &part.mesh.vertices {
                for f in [v.position.x, v.position.y, v.position.z, v.uv.x, v.uv.y] {
                    f.to_bits().hash(&mut hasher);
                }
                v.color.hash(&mut hasher);
            }
            for v in &part.skin_vertices {
                v.joints.hash(&mut hasher);
                for w in v.weights {
                    w.to_bits().hash(&mut hasher);
                }
            }
            part.rigid_vertices.iter().for_each(|v| v.node_idx.hash(&mut hasher));
            part.skinned_vertex_count.hash(&mut hasher);
            format!("{:?}", part.mesh.texture.as_ref().map(|t| t.raw_miniquad_id())).hash(&mut hasher);
        }
        let key = hasher.finish();
        GEOMETRY.with(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some(existing) = cache.get(&key).and_then(Weak::upgrade) {
                return existing;
            }
            let geometry = Rc::new(SharedGeometry { parts, gpu: RefCell::new(GpuGeometry::Untried) });
            cache.insert(key, Rc::downgrade(&geometry));
            geometry
        })
    }
}

impl AnimatedModel {
    pub fn mesh(&self) -> &Mesh {
        self.meshes().next().expect("an animated model has at least one part")
    }

    pub fn meshes(&self) -> impl Iterator<Item = &Mesh> {
        let skinned = self.skinned.as_deref();
        let bind = self.geometry.parts.iter().map(|p| &p.mesh);
        skinned.into_iter().flatten().chain(bind.filter(move |_| skinned.is_none()))
    }

    pub fn has_any_texture(&self) -> bool {
        self.geometry.parts.iter().any(|p| p.mesh.texture.is_some())
    }

    pub fn total_vertex_count(&self) -> usize {
        self.geometry.parts.iter().map(|p| p.mesh.vertices.len()).sum()
    }

    /// Advance the clip and bring its pose up to date: every frame when the
    /// GPU skins it, otherwise re-skinning on the CPU at most ~30 times a second.
    pub fn update(&mut self, dt: f32) {
        self.advance(dt);
        if self.gpu_drawn.get() {
            self.skin_accum = 0.0;
            self.pose();
        } else if self.skin_accum >= SKIN_INTERVAL {
            self.skin_accum = 0.0;
            self.pose();
            self.skin_on_cpu();
        }
    }

    /// Advance the clip's clock without re-skinning: cheap, for a model nobody
    /// is drawing right now. The next `update` re-skins it at once.
    pub fn advance(&mut self, dt: f32) {
        self.skin_accum += dt;
        if self.duration > 0.0 && !self.playback_paused {
            let start = self.playback_start.clamp(0.0, self.duration);
            let end = self.playback_end.clamp(start, self.duration);
            if self.playback_loop {
                let range = (end - start).max(0.0001);
                let mut t = self.time + dt;
                if t < start || t > end {
                    t = start;
                }
                while t > end {
                    t = start + (t - end);
                }
                if t < start {
                    t = start + (t - start).rem_euclid(range);
                }
                self.time = t;
                self.playback_finished = false;
            } else {
                self.time += dt;
                if self.time >= end {
                    self.time = end;
                    self.playback_paused = true;
                    self.playback_finished = true;
                }
            }
        }
    }

    /// Pose the skeleton at the current time: joint matrices for skinning and
    /// joint positions for hitboxes. Cheap (one matrix per joint).
    fn pose(&mut self) {
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
            // here we are caching each individual joint position for every joint in the skeleton so we can do hit detection against them for the player
            self.joint_positions[i] = global.transform_point3(Vec3::ZERO);
            self.skin_mats[i] = global * self.inverse_bind[i];
        }
    }

    /// Skin the mesh on the CPU with the current pose.
    fn skin_on_cpu(&mut self) {
        let geometry = Rc::clone(&self.geometry);
        let skinned = self
            .skinned
            .get_or_insert_with(|| geometry.parts.iter().map(|p| clone_mesh(&p.mesh)).collect());

        for (part, mesh) in geometry.parts.iter().zip(skinned.iter_mut()) {
            for (i, vtx) in part.skin_vertices.iter().enumerate() {
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
                mesh.vertices[i].position = out_p;
                mesh.vertices[i].normal = out_n.extend(0.0);
            }
        }

        // Rigid (unskinned) primitives still follow node animation transforms.
        for part_idx in 0..geometry.parts.len() {
            let part = &geometry.parts[part_idx];
            for (i, vtx) in part.rigid_vertices.iter().enumerate() {
                let out_idx = part.skinned_vertex_count + i;
                let global = self.global_for_node(vtx.node_idx);
                let out_p = global.transform_point3(vtx.bind_position);
                let out_n = (global * vtx.bind_normal.extend(0.0))
                    .truncate()
                    .normalize_or_zero();
                let Some(out) = self.skinned.as_mut().and_then(|m| m[part_idx].vertices.get_mut(out_idx)) else {
                    continue;
                };
                out.position = out_p;
                out.normal = out_n.extend(0.0);
            }
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
    // lets us find a joint position by matching tokens within the joint name (head, hips, etc) so we can create hitboxes for the player
    pub fn joint_position_with_tokens(&self, tokens: &[&str]) -> Option<Vec3> {
        self.joint_names
            .iter()
            .zip(self.joint_positions.iter())
            .find_map(|(name, pos)| {
                if tokens.iter().all(|t| name.contains(t)) {
                    Some(*pos)
                } else {
                    None
                }
            })
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn play_from_start(&mut self, looping: bool) {
        self.time = 0.0;
        self.playback_loop = looping;
        self.playback_paused = false;
        self.playback_finished = false;
        self.playback_start = 0.0;
        self.playback_end = self.duration.max(0.0);
        self.skin_accum = SKIN_INTERVAL;
    }

    pub fn stop_at_start(&mut self) {
        self.time = 0.0;
        self.playback_loop = false;
        self.playback_paused = true;
        self.playback_finished = false;
        self.playback_start = 0.0;
        self.playback_end = self.duration.max(0.0);
        self.skin_accum = SKIN_INTERVAL;
    }

    pub fn is_playback_finished(&self) -> bool {
        self.playback_finished
    }

    pub fn play_segment(&mut self, start: f32, end: f32, looping: bool) {
        let s = start.max(0.0);
        let e = end.max(s).min(self.duration.max(0.0));
        self.time = s;
        self.playback_loop = looping;
        self.playback_paused = false;
        self.playback_finished = false;
        self.playback_start = s;
        self.playback_end = e;
        self.skin_accum = SKIN_INTERVAL;
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

/*
    this function is to hard lock specific gltf files to known-good animation clip indexes when a file has multiple clips.
    before implementing this we had a lot of animations where auto selection could pick an idle/t-pose/incorrect clip due to generic clip names,
    so we now keep a deterministic per-file override table based on debug logs.
    these values come from the printed candidate + selected clip output during loading, so there is no guesswork and no extra metadata needed.
    if a source gltf changes clip ordering, update the index here for that file stem.
*/
fn animation_override_index_for_stem(file_stem: &str) -> Option<usize> {
    match file_stem {
        // values below found from log messages
        // each number represents the animation index in the gltf file that matches the intended animation for that file
        "crouch" => Some(8),
        "crouch_aim" => Some(8),
        "crouch_back" => Some(8),
        "crouch_death" => Some(0),
        "crouch_forward" => Some(8),
        "crouch_left" => Some(8),
        "crouch_right" => Some(8),
        "jump" => Some(3),
        "prone_death" => Some(0),
        "prone_forward" => Some(0),
        "prone_idle" => Some(6),
        "prone_turn_right" => Some(0),
        "rifle_aim_idle" => Some(0),
        "rifle_idle" => Some(8),
        "slide" => Some(3),
        "sprint_forward" => Some(0),
        "sprint_left" => Some(0),
        "sprint_right" => Some(0),
        "stand_death" => Some(2),
        "walk_back" => Some(2),
        "walk_forward" => Some(0),
        "walk_left" => Some(0),
        "walk_right" => Some(0),
        "fps_ak-74m_animations" => Some(1), // Rig|AK_Idle
        _ => None,
    }
}

pub fn load_gltf_animated_model(path: &str, bytes: &[u8]) -> Result<AnimatedModel, String> {
    load_gltf_animated_model_internal(path, bytes, ClipSelectionMode::LongestDuration, None, false)
}

pub fn load_gltf_animated_model_with_mode(
    path: &str,
    bytes: &[u8],
    mode: ClipSelectionMode,
) -> Result<AnimatedModel, String> {
    load_gltf_animated_model_internal(path, bytes, mode, None, false)
}

pub fn load_gltf_animated_model_with_clip_index(
    path: &str,
    bytes: &[u8],
    clip_index: usize,
) -> Result<AnimatedModel, String> {
    load_gltf_animated_model_internal(
        path,
        bytes,
        ClipSelectionMode::LongestDuration,
        Some(clip_index),
        false,
    )
}

pub fn load_gltf_animated_model_with_unskinned(
    path: &str,
    bytes: &[u8],
    include_unskinned_primitives: bool,
) -> Result<AnimatedModel, String> {
    load_gltf_animated_model_internal(
        path,
        bytes,
        ClipSelectionMode::LongestDuration,
        None,
        include_unskinned_primitives,
    )
}

fn load_gltf_animated_model_internal(
    path: &str,
    bytes: &[u8],
    mode: ClipSelectionMode,
    forced_clip_index: Option<usize>,
    include_unskinned_primitives: bool,
) -> Result<AnimatedModel, String> {
    let (doc, buffers, images) = import_glb(path, bytes)?;

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

    let mut prim_nodes: HashMap<(usize, usize), usize> = HashMap::new();
    if include_unskinned_primitives {
        // Map mesh primitive -> node index so rigid primitives can follow node transforms.
        for node in doc.nodes() {
            if let Some(node_mesh) = node.mesh() {
                for prim in node_mesh.primitives() {
                    prim_nodes
                        .entry((node_mesh.index(), prim.index()))
                        .or_insert(node.index());
                }
            }
        }
    }

    // Collect all skinned primitives plus rigid (unskinned) primitives.
    let mut skinned_prims: Vec<(gltf::Mesh, gltf::Primitive)> = Vec::new();
    let mut rigid_prims: Vec<(gltf::Mesh, gltf::Primitive, usize)> = Vec::new();
    for mesh in doc.meshes() {
        for prim in mesh.primitives() {
            let r = prim.reader(|buffer| Some(&buffers[buffer.index()]));
            if r.read_positions().is_some() && r.read_joints(0).is_some() && r.read_weights(0).is_some() {
                skinned_prims.push((mesh.clone(), prim));
            } else if include_unskinned_primitives && r.read_positions().is_some() {
                let node_idx = prim_nodes
                    .get(&(mesh.index(), prim.index()))
                    .copied()
                    .unwrap_or(0);
                rigid_prims.push((mesh.clone(), prim, node_idx));
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
    let override_anim_index = animation_override_index_for_stem(&file_stem);
    let strict_override_mode = env::var("ANIM_REQUIRE_OVERRIDE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let mut selected_anim = None;
    let mut best_name_score = -1i32;
    let mut best_metric = f32::NEG_INFINITY;
    let mut best_duration = f32::NEG_INFINITY;
    let mut multi_clip_count = 0usize;
    let mut override_found = false;
    for anim in doc.animations() {
        if let Some(forced) = forced_clip_index {
            if anim.index() != forced {
                continue;
            }
        }
        multi_clip_count += 1;
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
        if forced_clip_index.is_none() {
        if let Some(override_idx) = override_anim_index {
            if anim.index() == override_idx {
                if has_channels {
                    println!(
                        "[anim] {} using override anim index={} (stem='{}')",
                        path, override_idx, file_stem
                    );
                    selected_anim = Some(anim);
                    override_found = true;
                    break;
                }
                println!(
                    "[anim] {} override anim index={} has no channels; falling back to auto selection",
                    path, override_idx
                );
            }
        }
        }
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
    if forced_clip_index.is_none() && strict_override_mode && multi_clip_count > 1 && override_anim_index.is_none() {
        return Err(format!(
            "ANIM_REQUIRE_OVERRIDE is enabled and '{}' has {} clips but no override. \
Add one in animation_override_index_for_stem().",
            path, multi_clip_count
        ));
    }
    if forced_clip_index.is_none() && strict_override_mode && multi_clip_count > 1 && override_anim_index.is_some() && !override_found {
        return Err(format!(
            "ANIM_REQUIRE_OVERRIDE is enabled and '{}' override index {:?} was not found/usable.",
            path, override_anim_index
        ));
    }
    let anim = selected_anim.ok_or_else(|| format!("no usable animation clip found in '{}'", path))?;
    if forced_clip_index.is_none() && multi_clip_count > 1 && override_anim_index.is_none() {
        println!(
            "[anim] {} auto-selected anim={} (stem='{}'). To lock this clip, add override: \"{}\" => Some({}),",
            path,
            anim.index(),
            file_stem,
            file_stem,
            anim.index()
        );
    }

    println!(
        "[anim] {} using mesh={} first_prim={} anim={}",
        path,
        mesh.index(),
        first_prim_index,
        anim.index()
    );

    // Build one animated part per primitive so each part keeps its own texture/material.
    let mut parts: Vec<AnimatedPart> = Vec::new();
    let mut all_positions: Vec<Vec3> = Vec::new();

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

        let mesh_texture = sp
            .material()
            .pbr_metallic_roughness()
            .base_color_texture()
            .and_then(|base_color| images.texture(base_color.texture().source().index()));

        let f = sp.material().pbr_metallic_roughness().base_color_factor();
        let mat_color = [
            (f[0] * 255.0) as u8,
            (f[1] * 255.0) as u8,
            (f[2] * 255.0) as u8,
            (f[3] * 255.0) as u8,
        ];

        // Primitive-local indices.
        let prim_indices: Vec<u16> = match reader.read_indices() {
            Some(ReadIndices::U8(it)) => it.map(|i| i as u16).collect(),
            Some(ReadIndices::U16(it)) => it.collect(),
            Some(ReadIndices::U32(it)) => {
                let mut out = Vec::new();
                for i in it {
                    if i > u16::MAX as u32 {
                        return Err(format!("index {} exceeds u16 in '{}'", i, path));
                    }
                    out.push(i as u16);
                }
                out
            }
            None => {
                let mut out = Vec::with_capacity(positions.len());
                for i in 0..positions.len() {
                    if i > u16::MAX as usize {
                        return Err(format!("vertex index {} exceeds u16 in '{}'", i, path));
                    }
                    out.push(i as u16);
                }
                out
            }
        };

        let runtime_vertices: Vec<Vertex> = positions
            .iter()
            .zip(normals.iter())
            .zip(uvs.iter())
            .map(|((p, n), uv)| Vertex {
                position: *p,
                uv: *uv,
                color: mat_color,
                normal: n.extend(0.0),
            })
            .collect();
        let skin_vertices: Vec<SkinVertex> = positions
            .iter()
            .zip(normals.iter())
            .zip(joints.iter())
            .zip(weights.iter())
            .map(|(((p, n), j), w)| SkinVertex {
                bind_position: *p,
                bind_normal: *n,
                joints: *j,
                weights: *w,
            })
            .collect();

        all_positions.extend_from_slice(&positions);
        parts.push(AnimatedPart {
            mesh: Mesh {
                vertices: runtime_vertices,
                indices: prim_indices,
                texture: mesh_texture,
            },
            skin_vertices,
            rigid_vertices: Vec::new(),
            skinned_vertex_count: positions.len(),
        });
    }

    println!("[anim] '{}' merging {} rigid primitives", path, rigid_prims.len());
    for (_mesh_ref, rp, node_idx) in &rigid_prims {
        let reader = rp.reader(|buffer| Some(&buffers[buffer.index()]));
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

        if normals.len() != positions.len() || uvs.len() != positions.len() {
            return Err(format!(
                "rigid vertex attribute length mismatch in prim {} of '{}'",
                rp.index(),
                path
            ));
        }

        let mesh_texture = rp
            .material()
            .pbr_metallic_roughness()
            .base_color_texture()
            .and_then(|base_color| images.texture(base_color.texture().source().index()));

        let f = rp.material().pbr_metallic_roughness().base_color_factor();
        let mat_color = [
            (f[0] * 255.0) as u8,
            (f[1] * 255.0) as u8,
            (f[2] * 255.0) as u8,
            (f[3] * 255.0) as u8,
        ];

        let prim_indices: Vec<u16> = match reader.read_indices() {
            Some(ReadIndices::U8(it)) => it.map(|i| i as u16).collect(),
            Some(ReadIndices::U16(it)) => it.collect(),
            Some(ReadIndices::U32(it)) => {
                let mut out = Vec::new();
                for i in it {
                    if i > u16::MAX as u32 {
                        return Err(format!("rigid index {} exceeds u16 in '{}'", i, path));
                    }
                    out.push(i as u16);
                }
                out
            }
            None => {
                let mut out = Vec::with_capacity(positions.len());
                for i in 0..positions.len() {
                    if i > u16::MAX as usize {
                        return Err(format!(
                            "rigid vertex index {} exceeds u16 in '{}'",
                            i, path
                        ));
                    }
                    out.push(i as u16);
                }
                out
            }
        };

        let mut rigid_runtime_vertices: Vec<Vertex> = Vec::with_capacity(positions.len());
        let mut rigid_vertices: Vec<RigidVertex> = Vec::with_capacity(positions.len());
        for i in 0..positions.len() {
            rigid_runtime_vertices.push(Vertex {
                position: positions[i],
                uv: uvs[i],
                color: mat_color,
                normal: normals[i].extend(0.0),
            });
            rigid_vertices.push(RigidVertex {
                bind_position: positions[i],
                bind_normal: normals[i],
                node_idx: *node_idx,
            });
        }
        all_positions.extend_from_slice(&positions);
        parts.push(AnimatedPart {
            mesh: Mesh {
                vertices: rigid_runtime_vertices,
                indices: prim_indices,
                texture: mesh_texture,
            },
            skin_vertices: Vec::new(),
            rigid_vertices,
            skinned_vertex_count: 0,
        });
    }
    if parts.is_empty() {
        return Err(format!("no renderable primitives found in '{}'", path));
    }
    let total_vertices: usize = parts.iter().map(|p| p.mesh.vertices.len()).sum();
    let total_indices: usize = parts.iter().map(|p| p.mesh.indices.len()).sum();
    println!(
        "[anim] '{}' total vertices={} indices={} parts={}",
        path,
        total_vertices,
        total_indices,
        parts.len()
    );

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
    let node_names: Vec<String> = doc
        .nodes()
        .map(|n| n.name().unwrap_or("").to_ascii_lowercase())
        .collect();

    // here we are collecting joint names for each joint node so we can do token matching against them to find hitbox positions for the player
    let joint_names: Vec<String> = joint_nodes
        .iter()
        .map(|&i| node_names.get(i).cloned().unwrap_or_default())
        .collect();
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
        geometry: SharedGeometry::shared(parts),
        skinned: None,
        gpu_drawn: Cell::new(false),
        node_pose: node_rest.clone(),
        node_rest,
        node_parents,
        node_global: vec![Mat4::IDENTITY; node_count],
        node_global_valid: vec![false; node_count],
        node_tracks,
        joint_nodes,
        joint_names,
        joint_positions: vec![Vec3::ZERO; skin.joints().len()],
        inverse_bind,
        skin_mats: vec![Mat4::IDENTITY; skin.joints().len()],
        duration,
        time: 0.0,
        skin_accum: SKIN_INTERVAL, // force first update
        playback_loop: true,
        playback_paused: false,
        playback_finished: false,
        playback_start: 0.0,
        playback_end: duration.max(0.0),
        normalize_scale,
        y_offset,
    };
    // Joint positions (hitboxes) are valid from the start; vertices are skinned
    // the first time the clip is drawn.
    model.pose();

    println!(
        "[anim] {} raw_height={:.1} scale={:.4} y_offset={:.2}",
        path, raw_height, normalize_scale, y_offset
    );

    Ok(model)
}

/// Parse a self-contained `.glb` from memory. The bytes are read up front with
/// macroquad's `load_file`, which works on native builds and in the browser
/// alike; `path` is only for error messages. Images are left encoded until a
/// mesh asks for one (see [`GlbImages`]).
pub fn import_glb(
    path: &str,
    bytes: &[u8],
) -> Result<(gltf::Document, Vec<gltf::buffer::Data>, GlbImages), String> {
    let fail = |e: gltf::Error| format!("failed to import gltf '{}': {e}", path);
    let gltf::Gltf { document, blob } = gltf::Gltf::from_slice(bytes).map_err(fail)?;
    let buffers = gltf::import_buffers(&document, None, blob).map_err(fail)?;
    // Only images stored in the binary chunk are supported; the game's GLBs
    // embed all of theirs that way.
    let encoded = document
        .images()
        .map(|image| match image.source() {
            gltf::image::Source::View { view, .. } => {
                let data = &buffers[view.buffer().index()];
                data.get(view.offset()..view.offset() + view.length()).map(<[u8]>::to_vec)
            }
            gltf::image::Source::Uri { .. } => None,
        })
        .collect();
    Ok((document, buffers, GlbImages { encoded }))
}

/// A GLB's embedded images (PNG or JPEG), decoded when a mesh uses one.
/// Decoded textures are shared by content: the player's animation files each
/// embed the same textures, and decoding and uploading those once instead of
/// once per file saves most of the load time and GPU memory.
pub struct GlbImages {
    encoded: Vec<Option<Vec<u8>>>,
}

thread_local! {
    /// Textures by a hash of their encoded bytes; `None` if one failed to decode.
    static TEXTURES: std::cell::RefCell<HashMap<u64, Option<Texture2D>>> =
        std::cell::RefCell::new(HashMap::new());
}

impl GlbImages {
    /// The image at `index` as a texture (nearest filtering), decoded at most
    /// once per distinct image.
    pub fn texture(&self, index: usize) -> Option<Texture2D> {
        use std::hash::{Hash, Hasher};
        let bytes = self.encoded.get(index)?.as_ref()?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        let key = hasher.finish();
        TEXTURES.with(|textures| {
            textures.borrow_mut().entry(key).or_insert_with(|| decode_texture(bytes)).clone()
        })
    }
}

fn decode_texture(bytes: &[u8]) -> Option<Texture2D> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    if width > u16::MAX as u32 || height > u16::MAX as u32 {
        return None;
    }
    let texture = Texture2D::from_rgba8(width as u16, height as u16, &image);
    texture.set_filter(FilterMode::Nearest);
    Some(texture)
}

fn clone_mesh(mesh: &Mesh) -> Mesh {
    Mesh { vertices: mesh.vertices.clone(), indices: mesh.indices.clone(), texture: mesh.texture.clone() }
}

// ---------------------------------------------------------------- GPU skinning

/// The most joints the skinning shader takes. Each costs 4 of the vertex
/// shader's uniform vectors; WebGL only guarantees 128, but desktop and most
/// mobile GPUs offer 1024 or more. A GPU that cannot fit the shader falls
/// back to skinning on the CPU.
const MAX_GPU_JOINTS: usize = 80;

enum GpuGeometry {
    Untried,
    /// One vertex and index buffer per part, uploaded once.
    Ready(Vec<GpuPart>),
    /// Rigid parts, too many joints, or no shader: skinned on the CPU instead.
    Unavailable,
}

struct GpuPart {
    vertices: miniquad::BufferId,
    indices: miniquad::BufferId,
    index_count: i32,
    texture: Option<Texture2D>,
    color: [f32; 4],
}

/// Pipeline and a 1x1 white texture for untextured parts; `None` if the GPU
/// cannot compile the shader.
struct GpuSkinning {
    pipeline: miniquad::Pipeline,
    white: miniquad::TextureId,
}

thread_local! {
    static GPU_SKINNING: std::cell::OnceCell<Option<GpuSkinning>> = const { std::cell::OnceCell::new() };
}

#[repr(C)]
struct GpuSkinUniforms {
    mvp: Mat4,
    color: [f32; 4],
    tint: [f32; 4],
    joints: [Mat4; MAX_GPU_JOINTS],
}

/// Per vertex: position, uv, joint indices and weights.
#[repr(C)]
struct GpuSkinVertex {
    position: [f32; 3],
    uv: [f32; 2],
    joints: [f32; 4],
    weights: [f32; 4],
}

const GPU_SKIN_VERTEX: &str = r#"#version 100
attribute vec3 in_position;
attribute vec2 in_uv;
attribute vec4 in_joints;
attribute vec4 in_weights;

uniform mat4 Mvp;
uniform mat4 Joints[80];

varying lowp vec2 uv;

void main() {
    mat4 skin = Joints[int(in_joints.x)] * in_weights.x
        + Joints[int(in_joints.y)] * in_weights.y
        + Joints[int(in_joints.z)] * in_weights.z
        + Joints[int(in_joints.w)] * in_weights.w;
    gl_Position = Mvp * skin * vec4(in_position, 1.0);
    uv = in_uv;
}"#;

const GPU_SKIN_FRAGMENT: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;

uniform sampler2D Texture;
uniform lowp vec4 Color;
uniform lowp vec4 Tint;

void main() {
    gl_FragColor = Color * texture2D(Texture, uv) * Tint;
}"#;

fn gpu_skinning_setup(ctx: &mut dyn miniquad::RenderingBackend) -> Option<GpuSkinning> {
    use miniquad::*;
    let shader = ctx
        .new_shader(
            ShaderSource::Glsl { vertex: GPU_SKIN_VERTEX, fragment: GPU_SKIN_FRAGMENT },
            ShaderMeta {
                images: vec!["Texture".to_string()],
                uniforms: UniformBlockLayout {
                    uniforms: vec![
                        UniformDesc::new("Mvp", UniformType::Mat4),
                        UniformDesc::new("Color", UniformType::Float4),
                        UniformDesc::new("Tint", UniformType::Float4),
                        UniformDesc::new("Joints", UniformType::Mat4).array(MAX_GPU_JOINTS),
                    ],
                },
            },
        )
        .map_err(|e| macroquad::logging::warn!("GPU skinning unavailable, skinning on the CPU: {e:?}"))
        .ok()?;
    let pipeline = ctx.new_pipeline(
        &[BufferLayout::default()],
        &[
            VertexAttribute::new("in_position", VertexFormat::Float3),
            VertexAttribute::new("in_uv", VertexFormat::Float2),
            VertexAttribute::new("in_joints", VertexFormat::Float4),
            VertexAttribute::new("in_weights", VertexFormat::Float4),
        ],
        shader,
        PipelineParams {
            depth_test: Comparison::LessOrEqual,
            depth_write: true,
            color_blend: Some(BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            )),
            ..Default::default()
        },
    );
    let white = ctx.new_texture_from_rgba8(1, 1, &[255, 255, 255, 255]);
    Some(GpuSkinning { pipeline, white })
}

fn gpu_parts(ctx: &mut dyn miniquad::RenderingBackend, parts: &[AnimatedPart]) -> Option<Vec<GpuPart>> {
    use miniquad::*;
    let mut out = Vec::with_capacity(parts.len());
    for part in parts {
        if !part.rigid_vertices.is_empty() || part.skin_vertices.len() != part.mesh.vertices.len() {
            return None;
        }
        if part.skin_vertices.iter().any(|v| v.joints.iter().any(|&j| j as usize >= MAX_GPU_JOINTS)) {
            return None;
        }
        let vertices: Vec<GpuSkinVertex> = part
            .mesh
            .vertices
            .iter()
            .zip(&part.skin_vertices)
            .map(|(v, s)| GpuSkinVertex {
                position: s.bind_position.to_array(),
                uv: v.uv.to_array(),
                joints: s.joints.map(|j| j as f32),
                weights: s.weights,
            })
            .collect();
        let color = part.mesh.vertices.first().map_or([255; 4], |v| v.color).map(|c| c as f32 / 255.0);
        out.push(GpuPart {
            vertices: ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&vertices)),
            indices: ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&part.mesh.indices)),
            index_count: part.mesh.indices.len() as i32,
            texture: part.mesh.texture.clone(),
            color,
        });
    }
    Some(out)
}

impl AnimatedModel {
    /// Draw the model skinned by the GPU at `model` (its world transform),
    /// multiplied by `tint`, from buffers uploaded once. Returns false when
    /// this model or GPU cannot (rigid parts, too many joints, no shader);
    /// draw `meshes()` then, which `update` keeps skinned on the CPU.
    pub fn draw_gpu(&self, model: Mat4, tint: Color) -> bool {
        if self.skin_mats.len() > MAX_GPU_JOINTS {
            self.gpu_drawn.set(false);
            return false;
        }
        let mut gl = unsafe { get_internal_gl() };
        let Some(skinning) = GPU_SKINNING.with(|s| {
            s.get_or_init(|| gpu_skinning_setup(gl.quad_context))
                .as_ref()
                .map(|s| (s.pipeline, s.white))
        }) else {
            self.gpu_drawn.set(false);
            return false;
        };
        let mut gpu = self.geometry.gpu.borrow_mut();
        if matches!(*gpu, GpuGeometry::Untried) {
            *gpu = match gpu_parts(gl.quad_context, &self.geometry.parts) {
                Some(parts) => GpuGeometry::Ready(parts),
                None => GpuGeometry::Unavailable,
            };
        }
        let GpuGeometry::Ready(parts) = &*gpu else {
            self.gpu_drawn.set(false);
            return false;
        };
        // From now on `update` only poses this clip, every frame.
        self.gpu_drawn.set(true);

        let mut uniforms = GpuSkinUniforms {
            mvp: gl.quad_gl.get_projection_matrix() * model,
            color: [1.0; 4],
            tint: [tint.r, tint.g, tint.b, tint.a],
            joints: [Mat4::IDENTITY; MAX_GPU_JOINTS],
        };
        uniforms.joints[..self.skin_mats.len()].copy_from_slice(&self.skin_mats);

        // Draw in macroquad's current pass, after what it has batched so far.
        gl.flush();
        let pass = gl.quad_gl.get_active_render_pass();
        let ctx = gl.quad_context;
        ctx.begin_pass(pass, miniquad::PassAction::Nothing);
        ctx.apply_pipeline(&skinning.0);
        for part in parts {
            uniforms.color = part.color;
            let texture = part.texture.as_ref().map_or(skinning.1, |t| t.raw_miniquad_id());
            ctx.apply_bindings(&miniquad::Bindings {
                vertex_buffers: vec![part.vertices],
                index_buffer: part.indices,
                images: vec![texture],
            });
            ctx.apply_uniforms(miniquad::UniformsSource::table(&uniforms));
            ctx.draw(0, part.index_count, 1);
        }
        ctx.end_render_pass();
        true
    }
}
