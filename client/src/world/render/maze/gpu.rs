//! Baked meshes kept on the GPU (one vertex and index buffer each, uploaded
//! when baked) and drawn with a pipeline matching macroquad's own 3D one, so
//! pixels come out as `draw_cube` made them. Without the shader (e.g. the
//! Metal backend) the meshes stay on the CPU and go through macroquad's
//! batcher instead, cell by cell.

use macroquad::models::Vertex;
use macroquad::prelude::*;
use macroquad::window::get_internal_gl;

use super::cull::DistanceCull;
use super::mesh::{Bounds, MeshData};

const VERTEX: &str = r#"#version 100
attribute vec3 in_pos;
attribute vec4 in_color;
attribute vec2 in_cull;

varying lowp vec4 color;

uniform mat4 Projection;
uniform vec4 Cull;

void main() {
    // Cells past the render distance: every vertex of the cell lands
    // outside the clip volume, so its triangles are dropped.
    vec2 d = in_cull - Cull.xy;
    if (d.x * d.x + d.y * d.y > Cull.z) {
        gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
    } else {
        gl_Position = Projection * vec4(in_pos, 1);
    }
    color = in_color / 255.0;
}"#;

const FRAGMENT: &str = r#"#version 100
varying lowp vec4 color;

void main() {
    gl_FragColor = color;
}"#;

#[repr(C)]
struct Uniforms {
    projection: Mat4,
    /// Viewer x, z; squared render distance; unused.
    cull: [f32; 4],
}

thread_local! {
    static PIPELINE: std::cell::OnceCell<Option<miniquad::Pipeline>> = const { std::cell::OnceCell::new() };
}

fn pipeline(ctx: &mut dyn miniquad::RenderingBackend) -> Option<miniquad::Pipeline> {
    use miniquad::*;
    PIPELINE.with(|p| {
        *p.get_or_init(|| {
            let shader = ctx
                .new_shader(
                    ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
                    ShaderMeta {
                        images: vec![],
                        uniforms: UniformBlockLayout {
                            uniforms: vec![
                                UniformDesc::new("Projection", UniformType::Mat4),
                                UniformDesc::new("Cull", UniformType::Float4),
                            ],
                        },
                    },
                )
                .map_err(|e| macroquad::logging::warn!("maze meshes stay on the CPU: {e:?}"))
                .ok()?;
            // Same state as macroquad's depth-tested triangle pipeline.
            Some(ctx.new_pipeline(
                &[BufferLayout::default()],
                &[
                    VertexAttribute::new("in_pos", VertexFormat::Float3),
                    VertexAttribute::new("in_color", VertexFormat::Byte4),
                    VertexAttribute::new("in_cull", VertexFormat::Float2),
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
            ))
        })
    })
}

pub(super) enum StoredMesh {
    Gpu {
        vertices: miniquad::BufferId,
        indices: miniquad::BufferId,
        index_count: i32,
        bounds: Bounds,
    },
    Cpu(MeshData),
}

impl StoredMesh {
    pub fn bounds(&self) -> &Bounds {
        match self {
            StoredMesh::Gpu { bounds, .. } => bounds,
            StoredMesh::Cpu(m) => &m.bounds,
        }
    }
}

/// Upload `meshes` to the GPU, or keep them on the CPU if it has no shader.
pub(super) fn store(meshes: Vec<MeshData>) -> Vec<StoredMesh> {
    use miniquad::*;
    let gl = unsafe { get_internal_gl() };
    if pipeline(gl.quad_context).is_none() {
        return meshes.into_iter().map(StoredMesh::Cpu).collect();
    }
    let ctx = gl.quad_context;
    meshes
        .into_iter()
        .map(|m| StoredMesh::Gpu {
            vertices: ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&m.vertices)),
            indices: ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&m.indices)),
            index_count: m.indices.len() as i32,
            bounds: m.bounds,
        })
        .collect()
}

/// Free the GPU buffers of `meshes`.
pub(super) fn release(meshes: Vec<StoredMesh>) {
    let gl = unsafe { get_internal_gl() };
    for m in meshes {
        if let StoredMesh::Gpu { vertices, indices, .. } = m {
            gl.quad_context.delete_buffer(vertices);
            gl.quad_context.delete_buffer(indices);
        }
    }
}

/// Draw `meshes` in order, dropping cells `dist` culls.
pub(super) fn draw(meshes: &[&StoredMesh], dist: &DistanceCull) {
    let mut gl = unsafe { get_internal_gl() };
    let projection = gl.quad_gl.get_projection_matrix();
    let cull_of = |b: &Bounds| {
        let max = if b.cullable { dist.max_dist_sq } else { f32::MAX };
        [dist.viewer.x, dist.viewer.y, max, 0.0]
    };

    if meshes.iter().any(|m| matches!(m, StoredMesh::Gpu { .. })) {
        if let Some(pipeline) = pipeline(gl.quad_context) {
            // Draw in macroquad's current pass, after what it has batched so far.
            gl.flush();
            let pass = gl.quad_gl.get_active_render_pass();
            let ctx = gl.quad_context;
            ctx.begin_pass(pass, miniquad::PassAction::Nothing);
            ctx.apply_pipeline(&pipeline);
            for m in meshes {
                if let StoredMesh::Gpu { vertices, indices, index_count, bounds } = m {
                    ctx.apply_bindings(&miniquad::Bindings {
                        vertex_buffers: vec![*vertices],
                        index_buffer: *indices,
                        images: vec![],
                    });
                    let uniforms = Uniforms { projection, cull: cull_of(bounds) };
                    ctx.apply_uniforms(miniquad::UniformsSource::table(&uniforms));
                    ctx.draw(0, *index_count, 1);
                }
            }
            ctx.end_render_pass();
        }
    }

    // CPU fallback: each cell through macroquad's batcher, as before.
    let mut scratch_v: Vec<Vertex> = Vec::new();
    let mut scratch_i: Vec<u16> = Vec::new();
    for m in meshes {
        let StoredMesh::Cpu(mesh) = m else { continue };
        for item in &mesh.items {
            if mesh.bounds.cullable && !dist.keeps(item.cull) {
                continue;
            }
            scratch_v.clear();
            scratch_i.clear();
            let verts = &mesh.vertices[item.first_vertex..item.first_vertex + item.vertex_count];
            scratch_v.extend(verts.iter().map(|v| Vertex {
                position: Vec3::from_array(v.pos),
                uv: Vec2::ZERO,
                color: v.color,
                normal: Vec4::ZERO,
            }));
            let first = item.first_vertex as u16;
            scratch_i.extend(
                mesh.indices[item.first_index..item.first_index + item.index_count]
                    .iter()
                    .map(|i| i - first),
            );
            gl.quad_gl.draw_mode(DrawMode::Triangles);
            gl.quad_gl.geometry(&scratch_v, &scratch_i);
        }
    }
}
