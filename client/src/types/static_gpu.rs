//! Static (unskinned) meshes kept on the GPU. `draw_mesh` copies every
//! vertex into macroquad's batcher and uploads it again every frame; a world
//! weapon is 7k-36k vertices whose only per-frame change is its transform.
//! Here each model's meshes are uploaded once into immutable buffers (merged
//! per texture, up to what u16 indices reach) and drawn with an MVP uniform,
//! the same way the GPU skinner draws bodies.

use std::cell::RefCell;
use std::collections::HashMap;

use macroquad::models::{Mesh, Vertex};
use macroquad::prelude::*;

const VERTEX_SHADER: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

uniform mat4 Mvp;

varying lowp vec2 uv;
varying lowp vec4 color;

void main() {
    gl_Position = Mvp * vec4(position, 1.0);
    color = color0 / 255.0;
    uv = texcoord;
}"#;

// macroquad's default material: vertex colour times texture
const FRAGMENT_SHADER: &str = r#"#version 100
precision mediump float;
varying lowp vec4 color;
varying lowp vec2 uv;

uniform sampler2D Texture;

void main() {
    gl_FragColor = color * texture2D(Texture, uv);
}"#;

#[repr(C)]
struct Uniforms {
    mvp: Mat4,
}

struct StaticPipeline {
    pipeline: miniquad::Pipeline,
    white: miniquad::TextureId,
}

/// One vertex+index buffer holding several meshes, drawn as one index range
/// per texture.
struct Batch {
    vertices: miniquad::BufferId,
    indices: miniquad::BufferId,
    draws: Vec<Draw>,
}

struct Draw {
    first_index: i32,
    index_count: i32,
    /// Kept so the texture outlives the buffers that sample it.
    texture: Option<Texture2D>,
}

struct Entry {
    /// Which `Vec<Mesh>` this was built from; a reloaded model re-uploads.
    source: Source,
    batches: Vec<Batch>,
}

/// Identifies the meshes an entry was uploaded from: where they live, plus a
/// cheap content sample, so a reloaded model that lands at the same address
/// with the same mesh count is still noticed.
#[derive(PartialEq)]
struct Source {
    address: (usize, usize),
    vertices: usize,
    sample: u64,
}

impl Source {
    fn of(meshes: &[Mesh]) -> Self {
        let mut sample = 0xcbf2_9ce4_8422_2325u64;
        let mut vertices = 0;
        for m in meshes {
            vertices += m.vertices.len();
            // first, middle and last vertex of every mesh
            for v in [m.vertices.first(), m.vertices.get(m.vertices.len() / 2), m.vertices.last()].into_iter().flatten() {
                for x in v.position.to_array() {
                    sample = (sample ^ x.to_bits() as u64).wrapping_mul(0x100_0000_01b3);
                }
            }
            sample = (sample ^ m.indices.len() as u64).wrapping_mul(0x100_0000_01b3);
        }
        Source { address: (meshes.as_ptr() as usize, meshes.len()), vertices, sample }
    }
}

thread_local! {
    static PIPELINE: std::cell::OnceCell<Option<StaticPipeline>> = const { std::cell::OnceCell::new() };
    static CACHE: RefCell<HashMap<String, Entry>> = RefCell::new(HashMap::new());
}

fn setup(ctx: &mut dyn miniquad::RenderingBackend) -> Option<StaticPipeline> {
    use miniquad::*;
    let shader = ctx
        .new_shader(
            ShaderSource::Glsl { vertex: VERTEX_SHADER, fragment: FRAGMENT_SHADER },
            ShaderMeta {
                images: vec!["Texture".to_string()],
                uniforms: UniformBlockLayout { uniforms: vec![UniformDesc::new("Mvp", UniformType::Mat4)] },
            },
        )
        .map_err(|e| macroquad::logging::warn!("static GPU meshes unavailable, batching on the CPU: {e:?}"))
        .ok()?;
    let pipeline = ctx.new_pipeline(
        // macroquad's own 40-byte vertex, uploaded as is; the normal is skipped
        &[BufferLayout { stride: std::mem::size_of::<Vertex>() as i32, ..Default::default() }],
        &[
            VertexAttribute::new("position", VertexFormat::Float3),
            VertexAttribute::new("texcoord", VertexFormat::Float2),
            VertexAttribute::new("color0", VertexFormat::Byte4),
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
    Some(StaticPipeline { pipeline, white })
}

const BATCH_MAX_VERTICES: usize = u16::MAX as usize + 1;

fn upload(ctx: &mut dyn miniquad::RenderingBackend, meshes: &[Mesh]) -> Vec<Batch> {
    use miniquad::*;
    // same-texture meshes next to each other, so they merge into one draw
    let mut order: Vec<&Mesh> = meshes.iter().filter(|m| m.vertices.len() <= BATCH_MAX_VERTICES).collect();
    order.sort_by_key(|m| m.texture.as_ref().map(|t| format!("{:?}", t.raw_miniquad_id())));

    let mut out = Vec::new();
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    let mut draws: Vec<Draw> = Vec::new();
    let flush = |vertices: &mut Vec<Vertex>, indices: &mut Vec<u16>, draws: &mut Vec<Draw>, out: &mut Vec<Batch>, ctx: &mut dyn RenderingBackend| {
        if draws.is_empty() {
            return;
        }
        out.push(Batch {
            vertices: ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(vertices)),
            indices: ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(indices)),
            draws: std::mem::take(draws),
        });
        vertices.clear();
        indices.clear();
    };
    let texture_id = |t: &Option<Texture2D>| t.as_ref().map(|t| t.raw_miniquad_id());

    for mesh in order {
        if mesh.indices.is_empty() {
            continue;
        }
        if vertices.len() + mesh.vertices.len() > BATCH_MAX_VERTICES {
            flush(&mut vertices, &mut indices, &mut draws, &mut out, ctx);
        }
        let base = vertices.len() as u16;
        let first_index = indices.len() as i32;
        vertices.extend_from_slice(&mesh.vertices);
        indices.extend(mesh.indices.iter().map(|&i| base + i));
        let index_count = mesh.indices.len() as i32;
        match draws.last_mut() {
            Some(last) if texture_id(&last.texture) == texture_id(&mesh.texture) => last.index_count += index_count,
            _ => draws.push(Draw { first_index, index_count, texture: mesh.texture.clone() }),
        }
    }
    flush(&mut vertices, &mut indices, &mut draws, &mut out, ctx);
    out
}

/// Draw `meshes` (the model stored under `key`) at `transform`, from buffers
/// uploaded the first time. Returns false when the GPU path is unavailable;
/// draw the meshes with `draw_mesh` then.
pub fn draw_static_meshes(key: &str, meshes: &[Mesh], transform: Mat4) -> bool {
    if meshes.is_empty() {
        return true;
    }
    let mut gl = unsafe { get_internal_gl() };
    let Some((pipeline, white)) =
        PIPELINE.with(|p| p.get_or_init(|| setup(gl.quad_context)).as_ref().map(|p| (p.pipeline, p.white)))
    else {
        return false;
    };
    let source = Source::of(meshes);

    CACHE.with_borrow_mut(|cache| {
        let stale = cache.get(key).is_some_and(|e| e.source != source);
        if stale {
            if let Some(old) = cache.remove(key) {
                for b in old.batches {
                    gl.quad_context.delete_buffer(b.vertices);
                    gl.quad_context.delete_buffer(b.indices);
                }
            }
        }
        let entry = cache
            .entry(key.to_string())
            .or_insert_with(|| Entry { source, batches: upload(gl.quad_context, meshes) });

        let uniforms = Uniforms { mvp: gl.quad_gl.get_projection_matrix() * transform };
        // Draw in macroquad's current pass, after what it has batched so far.
        gl.flush();
        let pass = gl.quad_gl.get_active_render_pass();
        let ctx = gl.quad_context;
        ctx.begin_pass(pass, miniquad::PassAction::Nothing);
        ctx.apply_pipeline(&pipeline);
        ctx.apply_uniforms(miniquad::UniformsSource::table(&uniforms));
        for batch in &entry.batches {
            for draw in &batch.draws {
                let texture = draw.texture.as_ref().map_or(white, |t| t.raw_miniquad_id());
                ctx.apply_bindings_from_slice(&[batch.vertices], batch.indices, &[texture]);
                ctx.draw(draw.first_index, draw.index_count, 1);
            }
        }
        ctx.end_render_pass();
    });
    true
}

/// Drop the buffers uploaded for `key`, so the next draw uploads its meshes
/// again. The asset loader calls this whenever it (re)writes a model, since a
/// reloaded model can sit at the same address with the same shape.
pub fn forget(key: &str) {
    let Some(entry) = CACHE.with_borrow_mut(|cache| cache.remove(key)) else { return };
    let gl = unsafe { get_internal_gl() };
    for b in entry.batches {
        gl.quad_context.delete_buffer(b.vertices);
        gl.quad_context.delete_buffer(b.indices);
    }
}
