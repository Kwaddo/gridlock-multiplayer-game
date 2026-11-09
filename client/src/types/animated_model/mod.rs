mod clip_select;
mod constants;
mod glb;
mod gpu;
mod joints;
mod loader;
mod nodes;
mod parts;
mod playback;
mod pose;

#[cfg(target_arch = "wasm32")]
pub mod web_decoded;

use macroquad::models::Mesh;
use macroquad::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
pub use pose::Pose;


use gpu::GpuGeometry;
use joints::JointRoleTable;

pub use glb::{import_glb, GlbImages};
#[cfg(not(target_arch = "wasm32"))]
pub use glb::predecode_glb_images;
pub use glb::release_predecoded;
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
    inverse_bind: Vec<Mat4>,
    // this clip's own pose at its own clock (the hands still use this)
    pose: Pose,
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
