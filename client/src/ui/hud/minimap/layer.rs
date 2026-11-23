//! The minimap's ground layer (maze cells or lobby floor tiles) as one
//! textured quad. Drawing ~1000 maze cells, or ~1500 lobby tiles with two
//! grid lines each, as separate rectangles rebuilt 900-3000 quads through
//! macroquad's CPU batcher every frame. Now the maze's cell colours live in a
//! small texture (one texel per cell, rebuilt only when a wall breaks) and a
//! shader does the per-cell edge fade, the lobby floor and its grid lines.
//!
//! The quad's texture coordinates are cell coordinates relative to the cell
//! the player is in, so the sub-cell scroll comes free and the numbers the
//! shader floors stay small (mediump-safe on WebGL1).

use std::cell::RefCell;

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;

use super::texels::{bake_cell_texels, TEX_PAD};
use crate::types::maze::CellKindCache;

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying vec2 rel;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    rel = texcoord;
    gl_Position = Projection * Model * vec4(position, 1);
}"#;

const FRAGMENT: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

varying vec2 rel;

uniform sampler2D Texture;
// cell the quad's coordinates are relative to
uniform vec2 Origin;
// player position, relative to Origin, in cells
uniform vec2 Player;
// x: cells from the centre to the edge, y: 1 circular / 0 square,
// z: 0 maze / 1 lobby, w: pixels per cell
uniform vec4 View;
// maze: texture width, height (cells), wall border (cells); lobby: w is the
// floor's half size (tiles)
uniform vec4 Grid;
uniform float FadeStart;
uniform vec4 FloorColor;
uniform vec4 LineColor;

float edge_fade(float t) {
    if (t >= 1.0) { return 0.0; }
    if (t <= FadeStart) { return 1.0; }
    float f = (1.0 - t) / (1.0 - FadeStart);
    return f * f * (3.0 - 2.0 * f);
}

void main() {
    vec2 cell = floor(rel);
    // one fade per cell, taken at its centre, as the per-cell rectangles had
    vec2 local = (cell + 0.5 - Player) / View.x;
    float t = View.y > 0.5 ? length(local) : max(abs(local.x), abs(local.y));
    float a = edge_fade(t);
    if (a <= 0.0) { discard; }
    vec2 world_cell = cell + Origin;
    if (View.z < 0.5) {
        vec4 c = texture2D(Texture, (world_cell + Grid.z + 0.5) / Grid.xy);
        gl_FragColor = vec4(c.rgb, c.a * a);
        return;
    }
    if (world_cell.x < -Grid.w || world_cell.y < -Grid.w
        || world_cell.x >= Grid.w || world_cell.y >= Grid.w) {
        discard;
    }
    // A 0.5 px grid line on each tile's top and left edge: how much of this
    // pixel it covers (no line on the floor's far edges, no tile owns one).
    vec2 f = rel - cell;
    vec2 near_edge = world_cell + step(0.5, f);
    vec2 d = min(f, 1.0 - f) * View.w;
    vec2 cover = clamp(0.75 - d, 0.0, 0.5) * step(near_edge, vec2(Grid.w - 0.5));
    float la = LineColor.a * a * max(cover.x, cover.y);
    float fa = FloorColor.a * a;
    // the line blended over the floor, as one colour
    float oa = la + fa * (1.0 - la);
    vec3 rgb = (LineColor.rgb * la + FloorColor.rgb * fa * (1.0 - la)) / max(oa, 0.0001);
    gl_FragColor = vec4(rgb, oa);
}"#;

thread_local! {
    static MATERIAL: Material = load_material(
        ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("Origin", UniformType::Float2),
                UniformDesc::new("Player", UniformType::Float2),
                UniformDesc::new("View", UniformType::Float4),
                UniformDesc::new("Grid", UniformType::Float4),
                UniformDesc::new("FadeStart", UniformType::Float1),
                UniformDesc::new("FloorColor", UniformType::Float4),
                UniformDesc::new("LineColor", UniformType::Float4),
            ],
            textures: vec![],
            // the 2D default: blended, no depth
            pipeline_params: PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        },
    )
    .expect("minimap shader failed to compile");

    /// The maze's cell colours, one texel per cell (see `texels`).
    static MAZE_TEXTURE: RefCell<Option<Texture2D>> = const { RefCell::new(None) };
}

/// What the layer shows.
pub(super) enum Ground<'a> {
    /// Maze cells from the texture `maze_texture` returned.
    Maze(&'a Texture2D),
    /// Lobby floor tiles from `-floor_half` to `floor_half` on both axes,
    /// with grid lines.
    Lobby { floor_half: f32, floor: Color, line: Color },
}

/// Where the layer goes: a `2 * half` square around `center` (screen),
/// showing cells of `px_per_cell` pixels around the player at `player_cells`.
pub(super) struct View {
    pub center: Vec2,
    pub half: f32,
    pub player_cells: Vec2,
    pub px_per_cell: f32,
    pub circular: bool,
    pub fade_start: f32,
}

/// The layer quad: screen corners and, under each, the cell coordinate
/// relative to `origin`.
#[derive(Debug, PartialEq)]
pub(super) struct Quad {
    pub origin: Vec2,
    pub player: Vec2,
    pub half_cells: f32,
    pub corners: [(Vec2, Vec2); 4],
}

pub(super) fn layer_quad(view: &View) -> Quad {
    let origin = view.player_cells.floor();
    let player = view.player_cells - origin;
    let half_cells = view.half / view.px_per_cell;
    // One cell past the minimap's edge: a cell centred just inside it drew
    // its whole rectangle, overhanging by up to half a cell (faded nearly out).
    let (c, h, hc) = (view.center, view.half + view.px_per_cell, half_cells + 1.0);
    // screen y grows down, as cell y (world z) does: no flip
    let corner = |sx: f32, sy: f32| (c + vec2(sx * h, sy * h), player + vec2(sx * hc, sy * hc));
    Quad {
        origin,
        player,
        half_cells,
        corners: [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)],
    }
}

/// The maze's cell texture, rebuilt when `changed` (the cache reclassified).
pub(super) fn maze_texture(kinds: &CellKindCache, changed: bool) -> Texture2D {
    MAZE_TEXTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if !changed {
            if let Some(tex) = slot.as_ref() {
                return tex.clone();
            }
        }
        let (w, h) = kinds.size();
        let (tw, th, bytes) = bake_cell_texels(w, h, |x, y| kinds.kind_at(x, y));
        match slot.as_ref() {
            Some(tex) if tex.width() as u16 == tw && tex.height() as u16 == th => {
                tex.update_from_bytes(tw as u32, th as u32, &bytes);
            }
            _ => {
                let tex = Texture2D::from_rgba8(tw, th, &bytes);
                tex.set_filter(FilterMode::Nearest);
                *slot = Some(tex);
            }
        }
        slot.as_ref().expect("set above").clone()
    })
}

pub(super) fn draw_layer(view: &View, ground: Ground) {
    let quad = layer_quad(view);
    let vertices = quad
        .corners
        .iter()
        .map(|&(s, uv)| Vertex::new(s.x, s.y, 0.0, uv.x, uv.y, WHITE))
        .collect();
    let mode = matches!(ground, Ground::Lobby { .. }) as u8 as f32;
    let view_u = vec4(quad.half_cells, view.circular as u8 as f32, mode, view.px_per_cell);
    let (texture, grid, floor, line) = match ground {
        Ground::Maze(tex) => {
            let grid = vec4(tex.width(), tex.height(), TEX_PAD as f32, 0.0);
            (Some(tex.clone()), grid, Color::default(), Color::default())
        }
        Ground::Lobby { floor_half, floor, line } => (None, vec4(1.0, 1.0, 0.0, floor_half), floor, line),
    };
    let mesh = Mesh { vertices, indices: vec![0, 1, 2, 0, 2, 3], texture };
    MATERIAL.with(|material| {
        material.set_uniform("Origin", quad.origin);
        material.set_uniform("Player", quad.player);
        material.set_uniform("View", view_u);
        material.set_uniform("Grid", grid);
        material.set_uniform("FadeStart", view.fade_start);
        material.set_uniform("FloorColor", floor.to_vec());
        material.set_uniform("LineColor", line.to_vec());
        gl_use_material(material);
        draw_mesh(&mesh);
        gl_use_default_material();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cell under screen point `s`, interpolating the quad as the GPU does.
    fn cell_at(q: &Quad, s: Vec2) -> (i32, i32) {
        let (tl, br) = (q.corners[0], q.corners[2]);
        let t = (s - tl.0) / (br.0 - tl.0);
        let rel = tl.1 + (br.1 - tl.1) * t;
        let cell = (rel + q.origin).floor();
        (cell.x as i32, cell.y as i32)
    }

    #[test]
    fn quad_puts_each_cell_where_the_rectangles_were() {
        for &(player_cells, px) in &[
            (vec2(12.3, 7.8), 200.0 / 33.0),
            (vec2(0.05, 36.95), 150.0 / 33.0),
            (vec2(-3.5, 2.25), 260.0 / 36.0),
        ] {
            let view = View {
                center: vec2(900.0, 116.0),
                half: 100.0,
                player_cells,
                px_per_cell: px,
                circular: true,
                fade_start: 0.8,
            };
            let q = layer_quad(&view);
            assert_eq!(q.origin, player_cells.floor());
            assert!((q.half_cells - 100.0 / px).abs() < 1e-4);
            let pc = player_cells.floor();
            let frac = player_cells - pc;
            // the old loop: cell (pc + d) centred at center + (d - frac + 0.5) * px
            for dy in -16..=16 {
                for dx in -16..=16 {
                    let d = vec2(dx as f32, dy as f32);
                    let s = view.center + (d - frac + 0.5) * px;
                    let expected = (pc.x as i32 + dx, pc.y as i32 + dy);
                    assert_eq!(cell_at(&q, s), expected, "{player_cells:?} d {d:?}");
                }
            }
        }
    }

    #[test]
    fn quad_covers_the_minimap_square_and_overhang() {
        let view = View {
            center: vec2(50.0, 60.0),
            half: 75.0,
            player_cells: vec2(4.5, 4.5),
            px_per_cell: 5.0,
            circular: false,
            fade_start: 0.8,
        };
        let q = layer_quad(&view);
        // the 150 px square plus one 5 px cell on every side
        assert_eq!(q.corners[0].0, vec2(-30.0, -20.0));
        assert_eq!(q.corners[2].0, vec2(130.0, 140.0));
        assert_eq!(q.corners[0].1, vec2(0.5 - 16.0, 0.5 - 16.0));
        assert_eq!(q.corners[2].1, vec2(0.5 + 16.0, 0.5 + 16.0));
        assert_eq!(q.half_cells, 15.0);
    }
}
