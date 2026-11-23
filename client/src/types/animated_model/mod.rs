mod aim_bend;
mod clip;
mod clip_select;
mod constants;
mod glb;
mod gpu;
mod hit_pose;
mod joints;
mod loader;
mod nodes;
mod parts;
mod playback;
mod upper_body;
mod pose;

#[cfg(target_arch = "wasm32")]
pub mod web_decoded;

use macroquad::models::Mesh;
use macroquad::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
pub use pose::{Nodes, Pose};
#[cfg(test)]
pub(crate) use hit_pose::test_rig;


use gpu::GpuGeometry;
use joints::JointRoleTable;

pub use glb::{import_glb, GlbImages};
#[cfg(not(target_arch = "wasm32"))]
pub use glb::predecode_glb_images;
pub use glb::release_predecoded;
pub use clip::{load_clip, Clip};
pub use joints::JointRole;
pub use loader::{
    load_gltf_animated_model, load_gltf_animated_model_with_clip_index,
    load_gltf_animated_model_with_mode, load_gltf_animated_model_with_unskinned,
};

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
    // translation * rotation * scale, built in one go, before this it was three matrices and two multiplies per node
    fn to_mat4(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
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
    #[allow(dead_code)] // unread while every material is unlit (see skin_pose_on_cpu)
    bind_normal: Vec3,
    joints: [u16; 4],
    weights: [f32; 4],
}

#[derive(Clone, Copy)]
struct RigidVertex {
    bind_position: Vec3,
    #[allow(dead_code)] // unread while every material is unlit (see skin_pose_on_cpu)
    bind_normal: Vec3,
    node_idx: usize,
}

pub struct AnimatedModel {
    /// Bind-pose mesh, skin weights and textures, shared by every clip of the
    /// same character (the player's 25 animation files hold one mesh).
    geometry: Rc<SharedGeometry>,
    /// Whether the last draw skinned on the GPU (see `draw_gpu`); then only
    /// the joint matrices are updated here, every frame.
    gpu_drawn: Cell<bool>,
    node_rest: Vec<LocalTransform>,
    node_parents: Vec<Option<usize>>,
    node_tracks: Vec<NodeAnimationTrack>,
    joint_nodes: Vec<usize>,
    joint_names: Vec<String>,
    /// Joint index of each `JointRole`, matched by name once at load.
    joint_roles: JointRoleTable,
    // spine + neck bones that tilt to look up/down, with their share of the tilt (see aim_bend)
    spine_chain: Vec<(usize, f32)>,
    // per node how much an upper body layer takes over (see upper_body.rs)
    upper_mask: Vec<f32>,
    inverse_bind: Vec<Mat4>,
    // this clip's own pose at its own clock (the hands still use this)
    pose: Pose,
    duration: f32,
    time: f32,
    skin_accum: f32,
    /// Clip time `pose` was last sampled at; the pose depends only on the
    /// time, so a held (paused) clip is not re-sampled every update.
    posed_time: Option<f32>,
    /// Clip time the CPU-skinned meshes were last bent for.
    cpu_skinned_time: Option<f32>,
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
    /// Some part has rigid (unskinned) vertices, which the CPU fallback
    /// places by non-joint nodes (see `finish_pose`). Worked out once here.
    has_rigid: bool,
}

thread_local! {
    /// Geometry by a hash of its content, so identical meshes are kept once.
    static GEOMETRY: RefCell<HashMap<u64, Weak<SharedGeometry>>> = RefCell::new(HashMap::new());
}

impl SharedGeometry {
    fn new(parts: Vec<AnimatedPart>) -> Self {
        let has_rigid = parts.iter().any(|p| !p.rigid_vertices.is_empty());
        Self { parts, gpu: RefCell::new(GpuGeometry::Untried), has_rigid }
    }

    /// The geometry already loaded with the same content as `parts`, or
    /// `parts` as new shared geometry.
    fn shared(parts: Vec<AnimatedPart>) -> Rc<SharedGeometry> {
        use std::hash::{Hash, Hasher};
        let mut hasher = QuickHasher(0);
        let pair = |a: f32, b: f32| a.to_bits() as u64 | (b.to_bits() as u64) << 32;
        parts.len().hash(&mut hasher);
        for part in &parts {
            part.mesh.indices.hash(&mut hasher);
            part.mesh.vertices.len().hash(&mut hasher);
            for v in &part.mesh.vertices {
                // whole words, two floats each, before this it was one small write per float
                hasher.write_u64(pair(v.position.x, v.position.y));
                hasher.write_u64(pair(v.position.z, v.uv.x));
                hasher.write_u64(v.uv.y.to_bits() as u64 | (u32::from_le_bytes(v.color) as u64) << 32);
                hasher.write_u64(pair(v.normal.x, v.normal.y));
                hasher.write_u64(pair(v.normal.z, v.normal.w));
            }
            part.skin_vertices.len().hash(&mut hasher);
            for v in &part.skin_vertices {
                let [a, b, c, d] = v.joints.map(u64::from);
                hasher.write_u64(a | b << 16 | c << 32 | d << 48);
                hasher.write_u64(pair(v.weights[0], v.weights[1]));
                hasher.write_u64(pair(v.weights[2], v.weights[3]));
            }
            part.rigid_vertices.len().hash(&mut hasher);
            part.rigid_vertices.iter().for_each(|v| v.node_idx.hash(&mut hasher));
            part.skinned_vertex_count.hash(&mut hasher);
            part.mesh.texture.as_ref().map(|t| t.raw_miniquad_id()).hash(&mut hasher);
        }
        let key = hasher.finish();
        GEOMETRY.with(|cache| {
            let mut cache = cache.borrow_mut();
            // a hit is checked against the content too: the hash is a quick
            // one, and a collision would give this model another one's mesh
            if let Some(existing) = cache.get(&key).and_then(Weak::upgrade) {
                if same_parts(&existing.parts, &parts) {
                    return existing;
                }
                // a collision: keep this geometry unshared
                return Rc::new(SharedGeometry::new(parts));
            }
            let geometry = Rc::new(SharedGeometry::new(parts));
            cache.insert(key, Rc::downgrade(&geometry));
            geometry
        })
    }
}

/// Whether two part lists hold the same mesh: the content the hash covers,
/// compared bit for bit.
fn same_parts(a: &[AnimatedPart], b: &[AnimatedPart]) -> bool {
    let bits3 = |v: Vec3| v.to_array().map(f32::to_bits);
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.skinned_vertex_count == b.skinned_vertex_count
                && a.mesh.indices == b.mesh.indices
                && a.mesh.texture.as_ref().map(|t| t.raw_miniquad_id())
                    == b.mesh.texture.as_ref().map(|t| t.raw_miniquad_id())
                && a.mesh.vertices.len() == b.mesh.vertices.len()
                && a.mesh.vertices.iter().zip(&b.mesh.vertices).all(|(x, y)| {
                    bits3(x.position) == bits3(y.position)
                        && x.uv.to_array().map(f32::to_bits) == y.uv.to_array().map(f32::to_bits)
                        && x.color == y.color
                        && x.normal.to_array().map(f32::to_bits) == y.normal.to_array().map(f32::to_bits)
                })
                && a.skin_vertices.len() == b.skin_vertices.len()
                && a.skin_vertices.iter().zip(&b.skin_vertices).all(|(x, y)| {
                    x.joints == y.joints && x.weights.map(f32::to_bits) == y.weights.map(f32::to_bits)
                })
                && a.rigid_vertices.len() == b.rigid_vertices.len()
                && a.rigid_vertices.iter().zip(&b.rigid_vertices).all(|(x, y)| x.node_idx == y.node_idx)
        })
}

/// A quick FxHash style hash fed whole words, only to spot the same mesh
/// loaded twice. Before this it was SipHash with about 14 small writes per
/// vertex, a few milliseconds per character.
struct QuickHasher(u64);

impl std::hash::Hasher for QuickHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write_u64(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x517c_c1b7_2722_0a95);
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut words = bytes.chunks_exact(8);
        for word in &mut words {
            self.write_u64(u64::from_le_bytes(word.try_into().unwrap()));
        }
        let rest = words.remainder();
        if !rest.is_empty() {
            let mut last = [0u8; 8];
            last[..rest.len()].copy_from_slice(rest);
            self.write_u64(u64::from_le_bytes(last));
        }
    }

    fn write_u8(&mut self, i: u8) {
        self.write_u64(i as u64);
    }

    fn write_u16(&mut self, i: u16) {
        self.write_u64(i as u64);
    }

    fn write_u32(&mut self, i: u32) {
        self.write_u64(i as u64);
    }

    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::models::Vertex;

    fn parts(x: f32, weight: f32) -> Vec<AnimatedPart> {
        let vertices: Vec<Vertex> = (0..4).map(|i| Vertex::new(x + i as f32, 1.0, 2.0, 0.5, 0.25, WHITE)).collect();
        let skin_vertices = (0..4)
            .map(|i| SkinVertex {
                bind_position: vec3(i as f32, 0.0, 0.0),
                bind_normal: Vec3::Y,
                joints: [1, 2, 0, 0],
                weights: [weight, 1.0 - weight, 0.0, 0.0],
            })
            .collect();
        vec![AnimatedPart {
            mesh: Mesh { vertices, indices: vec![0, 1, 2, 2, 1, 3], texture: None },
            skin_vertices,
            rigid_vertices: Vec::new(),
            skinned_vertex_count: 4,
        }]
    }

    // the same mesh loaded twice is kept once, any change in position or skin weights is its own
    #[test]
    fn same_geometry_is_shared_and_different_is_not() {
        let a = SharedGeometry::shared(parts(0.0, 0.75));
        let b = SharedGeometry::shared(parts(0.0, 0.75));
        assert!(Rc::ptr_eq(&a, &b));
        let moved = SharedGeometry::shared(parts(0.001, 0.75));
        let reweighted = SharedGeometry::shared(parts(0.0, 0.7));
        assert!(!Rc::ptr_eq(&a, &moved));
        assert!(!Rc::ptr_eq(&a, &reweighted));
        assert!(!Rc::ptr_eq(&moved, &reweighted));
    }
}
