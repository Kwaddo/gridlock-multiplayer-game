use macroquad::prelude::*;

use super::constants::MAX_GPU_JOINTS;
use super::{AnimatedModel, AnimatedPart, Pose};

// ---------------------------------------------------------------- GPU skinning

/// `?cpuskin` in the page URL turns GPU skinning off (browser build only).
fn force_cpu_skinning() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        extern "C" {
            fn gridlock_force_cpu_skinning() -> u32;
        }
        thread_local! {
            static FORCED: bool = unsafe { gridlock_force_cpu_skinning() } != 0;
        }
        FORCED.with(|f| *f)
    }
    #[cfg(not(target_arch = "wasm32"))]
    false
}

pub(super) enum GpuGeometry {
    Untried,
    /// The parts merged into as few vertex/index buffers as fit u16 indices,
    /// uploaded once.
    Ready(GpuMesh),
    /// Too many joints (skin joints plus rigid nodes), or no shader: skinned
    /// on the CPU instead.
    Unavailable,
}

/// A model's GPU buffers, and where its rigid parts' nodes go in the joint
/// table: rigid (unskinned) primitives, like the gun in the sawn-off and
/// minigun first-person rigs, are drawn as vertices bound with weight 1 to
/// one extra joint slot each, filled with their node's matrix, which is the
/// matrix the CPU fallback moves them by (`Pose::global_of`).
pub(super) struct GpuMesh {
    batches: Vec<GpuBatch>,
    /// First joint slot past every joint a skinned vertex names.
    rigid_base: usize,
    /// Node of each rigid slot, from `rigid_base` on.
    rigid_nodes: Vec<usize>,
}

/// One vertex and index buffer holding several parts back to back, drawn as
/// one index range per run of parts with the same texture and colour (the
/// load-time chunks of one primitive merge back into one draw).
pub(super) struct GpuBatch {
    vertices: miniquad::BufferId,
    indices: miniquad::BufferId,
    draws: Vec<GpuDraw>,
}

struct GpuDraw {
    first_index: i32,
    index_count: i32,
    /// Kept so the texture outlives the buffers that sample it.
    texture: Option<Texture2D>,
    color: [f32; 4],
}

impl GpuDraw {
    fn same_material(&self, texture: &Option<Texture2D>, color: [f32; 4]) -> bool {
        let id = |t: &Option<Texture2D>| t.as_ref().map(|t| t.raw_miniquad_id());
        id(&self.texture) == id(texture) && self.color == color
    }
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

/// Vertices one u16-indexed batch can hold.
const BATCH_MAX_VERTICES: usize = u16::MAX as usize + 1;

/// One batch's content before upload.
struct BatchData {
    vertices: Vec<GpuSkinVertex>,
    indices: Vec<u16>,
    draws: Vec<GpuDraw>,
}

/// The parts laid out for the GPU: batches, the first rigid slot and the
/// node of each. `None` when they do not fit the shader's joint table.
fn gpu_layout(parts: &[AnimatedPart]) -> Option<(Vec<BatchData>, usize, Vec<usize>)> {
    for part in parts {
        let skinned = part.skinned_vertex_count;
        if part.skin_vertices.len() != skinned || skinned + part.rigid_vertices.len() != part.mesh.vertices.len() {
            return None;
        }
        if part.mesh.vertices.len() > BATCH_MAX_VERTICES {
            return None;
        }
    }
    let skin_joints = parts.iter().flat_map(|p| &p.skin_vertices).flat_map(|v| v.joints);
    let rigid_base = skin_joints.map(|j| j as usize + 1).max().unwrap_or(0);
    let mut rigid_nodes: Vec<usize> = Vec::new();
    for v in parts.iter().flat_map(|p| &p.rigid_vertices) {
        if !rigid_nodes.contains(&v.node_idx) {
            rigid_nodes.push(v.node_idx);
        }
    }
    if rigid_base + rigid_nodes.len() > MAX_GPU_JOINTS {
        return None;
    }

    let mut out = Vec::new();
    let mut batch = BatchData { vertices: Vec::new(), indices: Vec::new(), draws: Vec::new() };
    for part in parts {
        if batch.vertices.len() + part.mesh.vertices.len() > BATCH_MAX_VERTICES && !batch.draws.is_empty() {
            let empty = BatchData { vertices: Vec::new(), indices: Vec::new(), draws: Vec::new() };
            out.push(std::mem::replace(&mut batch, empty));
        }
        let base = batch.vertices.len() as u16;
        let first_index = batch.indices.len() as i32;
        let (skinned_uvs, rigid_uvs) = part.mesh.vertices.split_at(part.skinned_vertex_count);
        batch.vertices.extend(skinned_uvs.iter().zip(&part.skin_vertices).map(|(v, s)| GpuSkinVertex {
            position: s.bind_position.to_array(),
            uv: v.uv.to_array(),
            joints: s.joints.map(|j| j as f32),
            weights: s.weights,
        }));
        batch.vertices.extend(rigid_uvs.iter().zip(&part.rigid_vertices).map(|(v, r)| {
            let slot = rigid_base + rigid_nodes.iter().position(|&n| n == r.node_idx).unwrap_or(0);
            GpuSkinVertex {
                position: r.bind_position.to_array(),
                uv: v.uv.to_array(),
                joints: [slot as f32, 0.0, 0.0, 0.0],
                weights: [1.0, 0.0, 0.0, 0.0],
            }
        }));
        batch.indices.extend(part.mesh.indices.iter().map(|&i| base + i));
        let index_count = part.mesh.indices.len() as i32;
        let color = part.mesh.vertices.first().map_or([255; 4], |v| v.color).map(|c| c as f32 / 255.0);
        match batch.draws.last_mut() {
            Some(last) if last.same_material(&part.mesh.texture, color) => last.index_count += index_count,
            _ => batch.draws.push(GpuDraw { first_index, index_count, texture: part.mesh.texture.clone(), color }),
        }
    }
    if !batch.draws.is_empty() {
        out.push(batch);
    }
    Some((out, rigid_base, rigid_nodes))
}

fn gpu_parts(ctx: &mut dyn miniquad::RenderingBackend, parts: &[AnimatedPart]) -> Option<GpuMesh> {
    use miniquad::*;
    let (layout, rigid_base, rigid_nodes) = gpu_layout(parts)?;
    let batches = layout
        .into_iter()
        .map(|b| GpuBatch {
            vertices: ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&b.vertices)),
            indices: ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&b.indices)),
            draws: b.draws,
        })
        .collect();
    Some(GpuMesh { batches, rigid_base, rigid_nodes })
}

/// The shader's joint table for `pose`: its skin matrices, then the matrix
/// of each rigid part's node (as the CPU fallback moves rigid vertices).
fn joint_table(pose: &Pose, rigid_base: usize, rigid_nodes: &[usize]) -> [Mat4; MAX_GPU_JOINTS] {
    let mut joints = [Mat4::IDENTITY; MAX_GPU_JOINTS];
    joints[..pose.skin_mats.len()].copy_from_slice(&pose.skin_mats);
    for (slot, &node) in joints[rigid_base..].iter_mut().zip(rigid_nodes) {
        if node < pose.local.len() {
            *slot = pose.global_of(node);
        }
    }
    joints
}

impl AnimatedModel {
    /// Compile the skinning shader and upload this model's buffers now (at
    /// load) rather than in the first frame that draws it, which would hitch.
    pub fn prepare_gpu(&self) {
        let gl = unsafe { get_internal_gl() };
        let _ = self.ensure_gpu(gl.quad_context);
    }

    /// The skinning pipeline and white texture once this model's buffers are
    /// on the GPU (uploading them the first time); `None` when this model or
    /// GPU cannot skin on the GPU.
    fn ensure_gpu(
        &self,
        ctx: &mut dyn miniquad::RenderingBackend,
    ) -> Option<(miniquad::Pipeline, miniquad::TextureId)> {
        if self.pose.skin_mats.len() > MAX_GPU_JOINTS || force_cpu_skinning() {
            return None;
        }
        let skinning = GPU_SKINNING
            .with(|s| s.get_or_init(|| gpu_skinning_setup(ctx)).as_ref().map(|s| (s.pipeline, s.white)))?;
        let mut gpu = self.geometry.gpu.borrow_mut();
        if matches!(*gpu, GpuGeometry::Untried) {
            *gpu = match gpu_parts(ctx, &self.geometry.parts) {
                Some(parts) => GpuGeometry::Ready(parts),
                None => GpuGeometry::Unavailable,
            };
        }
        matches!(*gpu, GpuGeometry::Ready(_)).then_some(skinning)
    }

    /// Draw the model skinned by the GPU at `model` (its world transform),
    /// multiplied by `tint`, from buffers uploaded once. Returns false when
    /// this model or GPU cannot (too many joints, no shader);
    /// draw `meshes()` then, which `update` keeps skinned on the CPU.
    // did the gpu draw this clip last time, if not the cpu has to bend the mesh
    // before this only the clip itself checked, inside update()
    pub fn drawn_on_gpu(&self) -> bool {
        self.gpu_drawn.get()
    }

    // the clip's own pose, same as before
    pub fn draw_gpu(&self, model: Mat4, tint: Color) -> bool {
        self.draw_gpu_pose(&self.pose, model, tint)
    }

    /// The clip's own pose with every material's alpha forced to 1, like
    /// the first-person hands' `draw_mesh_opaque` (texture alpha still applies).
    pub fn draw_gpu_opaque(&self, model: Mat4) -> bool {
        self.draw_gpu_impl(&self.pose, model, WHITE, true)
    }

    // draw bent like any pose you hand it, before this it could only draw the clip's own pose
    // so everyone on the same clip got drawn in the exact same pose
    pub fn draw_gpu_pose(&self, pose: &Pose, model: Mat4, tint: Color) -> bool {
        self.draw_gpu_impl(pose, model, tint, false)
    }

    fn draw_gpu_impl(&self, pose: &Pose, model: Mat4, tint: Color, opaque: bool) -> bool {
        let mut gl = unsafe { get_internal_gl() };
        let Some(skinning) = self.ensure_gpu(gl.quad_context) else {
            self.gpu_drawn.set(false);
            return false;
        };
        let gpu = self.geometry.gpu.borrow();
        let GpuGeometry::Ready(mesh) = &*gpu else {
            self.gpu_drawn.set(false);
            return false;
        };
        let batches = &mesh.batches;
        // From now on `update` only poses this clip, every frame.
        self.gpu_drawn.set(true);

        let mut uniforms = GpuSkinUniforms {
            mvp: gl.quad_gl.get_projection_matrix() * model,
            color: [1.0; 4],
            tint: [tint.r, tint.g, tint.b, tint.a],
            // this pose's joints (and rigid parts' nodes) go to the gpu
            joints: joint_table(pose, mesh.rigid_base, &mesh.rigid_nodes),
        };

        // Draw in macroquad's current pass, after what it has batched so far.
        gl.flush();
        let pass = gl.quad_gl.get_active_render_pass();
        let ctx = gl.quad_context;
        ctx.begin_pass(pass, miniquad::PassAction::Nothing);
        ctx.apply_pipeline(&skinning.0);
        // The 5 KB uniform block goes up once, and again only when a draw's
        // colour differs; bindings change only with the batch or texture.
        let mut uploaded_color: Option<[f32; 4]> = None;
        for batch in batches {
            let mut bound: Option<miniquad::TextureId> = None;
            for draw in &batch.draws {
                let texture = draw.texture.as_ref().map_or(skinning.1, |t| t.raw_miniquad_id());
                if bound != Some(texture) {
                    ctx.apply_bindings_from_slice(&[batch.vertices], batch.indices, &[texture]);
                    bound = Some(texture);
                }
                let color = if opaque { [draw.color[0], draw.color[1], draw.color[2], 1.0] } else { draw.color };
                if uploaded_color != Some(color) {
                    uniforms.color = color;
                    ctx.apply_uniforms(miniquad::UniformsSource::table(&uniforms));
                    uploaded_color = Some(color);
                }
                ctx.draw(draw.first_index, draw.index_count, 1);
            }
        }
        ctx.end_render_pass();
        true
    }
}

#[cfg(test)]
mod tests;
