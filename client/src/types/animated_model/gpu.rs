use macroquad::prelude::*;

use super::constants::MAX_GPU_JOINTS;
use super::{AnimatedModel, AnimatedPart};

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
    /// One vertex and index buffer per part, uploaded once.
    Ready(Vec<GpuPart>),
    /// Rigid parts, too many joints, or no shader: skinned on the CPU instead.
    Unavailable,
}

pub(super) struct GpuPart {
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
        if self.skin_mats.len() > MAX_GPU_JOINTS || force_cpu_skinning() {
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
