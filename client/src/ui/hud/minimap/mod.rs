use macroquad::prelude::*;

use crate::core::GameConfig;
use crate::types::maze::{CellKindCache, Maze};
use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;
use crate::ui::prims::{fill_disc, ring};
use crate::weapons::combat::CombatEntity;
use crate::world::lobby::{LANDMARK_CUBES, LOBBY_CONSOLE_POS, LOBBY_CONSOLE_SIZE, PARKOUR_CUBES};

mod layer;
mod texels;

const MINIMAP_MARGIN: f32 = 16.0;
/// Fraction of the radius where edge fade begins.
const FADE_START: f32 = 0.80;

const BG_COLOR: Color = Color::new(0.05, 0.06, 0.10, 0.90);
const LOCAL_PLAYER_COLOR: Color = Color::new(0.3, 0.9, 0.4, 1.0);
const BORDER_COLOR: Color = Color::new(0.45, 0.50, 0.60, 0.95);
const BORDER_GLOW: Color = Color::new(0.30, 0.35, 0.45, 0.35);

thread_local! {
    /// Cell kinds of the maze on the minimap, re-derived only when it changes
    /// (and the cell texture with them).
    static CELL_KINDS: std::cell::RefCell<CellKindCache> = const { std::cell::RefCell::new(CellKindCache::new()) };
}

// Match in-game lobby: floor is DARKGRAY, grid is Color::from_rgba(100,100,100,255)
const LOBBY_FLOOR_COLOR: Color = Color::new(0.30, 0.30, 0.30, 1.0);
const LOBBY_GRID_COLOR: Color = Color::new(0.39, 0.39, 0.39, 0.8);
/// How many world units are visible from center to edge.
const LOBBY_VIEW_RANGE: f32 = 18.0;

fn minimap_diameter(config: &GameConfig) -> f32 {
    match config.minimap_size {
        0 => 150.0,
        2 => 260.0,
        _ => 200.0,
    }
}

fn is_circular(config: &GameConfig) -> bool {
    config.minimap_shape == 0
}

pub fn draw_minimap(
    maze: Option<&Maze>,
    local_pos: Vec3,
    local_yaw: f32,
    other_players: &[OtherPlayer],
    current_world: WorldId,
    local_team: u8,
    config: &GameConfig,
    smoke_minimap: Option<(Vec<u32>, Vec<(Vec3, Color)>)>,
) {
    let diameter = minimap_diameter(config);
    let half = diameter / 2.0;
    let circular = is_circular(config);

    let center_x = screen_width() - MINIMAP_MARGIN - half;
    let center_y = MINIMAP_MARGIN + half;

    // Background
    if circular {
        fill_disc(center_x, center_y, half + 1.0, 20, BG_COLOR);
    } else {
        draw_rectangle(
            center_x - half,
            center_y - half,
            diameter,
            diameter,
            BG_COLOR,
        );
    }

    match current_world {
        WorldId::Maze => {
            if let Some(maze) = maze {
                draw_maze_layer(
                    maze, local_pos, center_x, center_y, half, diameter, circular,
                );
            }
        }
        WorldId::Lobby => {
            draw_lobby_layer(local_pos, center_x, center_y, half, diameter, circular);
        }
    }

    // Other players
    let ppwu = pixels_per_world_unit(current_world, maze, diameter);
    draw_other_players(
        other_players,
        current_world,
        local_pos,
        center_x,
        center_y,
        half,
        circular,
        ppwu,
        local_team,
        smoke_minimap.as_ref().map(|(ids, _)| ids.as_slice()),
    );

    if let Some((_, markers)) = &smoke_minimap {
        for (at, color) in markers {
            let sx = center_x + (at.x - local_pos.x) * ppwu;
            let sy = center_y + (at.z - local_pos.z) * ppwu;
            let alpha = fade_alpha(sx, sy, center_x, center_y, half, circular);
            if alpha > 0.0 { draw_circle(sx, sy, 4.5, Color::new(color.r, color.g, color.b, color.a * alpha)); }
        }
    }

    // Local player arrow (rotates to show facing direction)
    draw_player_arrow(center_x, center_y, local_yaw, ppwu);

    // Border
    if circular {
        ring(center_x, center_y, half + 2.0, 3.0, 30, BORDER_GLOW);
        ring(center_x, center_y, half, 2.0, 30, BORDER_COLOR);
    } else {
        let tl_x = center_x - half;
        let tl_y = center_y - half;
        draw_rectangle_lines(
            tl_x - 1.0,
            tl_y - 1.0,
            diameter + 2.0,
            diameter + 2.0,
            3.0,
            BORDER_GLOW,
        );
        draw_rectangle_lines(tl_x, tl_y, diameter, diameter, 2.0, BORDER_COLOR);
    }
}

// ─── Maze layer (north-up, no rotation) ───────────────────────────────────────

fn draw_maze_layer(
    maze: &Maze,
    local_pos: Vec3,
    center_x: f32,
    center_y: f32,
    half: f32,
    diameter: f32,
    circular: bool,
) {
    let cell_size = maze.cell_size;
    let ppwu = diameter / (cell_size * 33.0); // 16 cells each side + center
    let texture = CELL_KINDS.with(|kinds| {
        let mut kinds = kinds.borrow_mut();
        let changed = kinds.sync(maze);
        layer::maze_texture(&kinds, changed)
    });
    let view = layer::View {
        center: vec2(center_x, center_y),
        half,
        player_cells: vec2(local_pos.x, local_pos.z) / cell_size,
        px_per_cell: cell_size * ppwu,
        circular,
        fade_start: FADE_START,
    };
    layer::draw_layer(&view, layer::Ground::Maze(&texture));
}

// ─── Lobby layer ──────────────────────────────────────────────────────────────

fn draw_lobby_layer(
    local_pos: Vec3,
    center_x: f32,
    center_y: f32,
    half: f32,
    diameter: f32,
    circular: bool,
) {
    let ppwu = diameter / (LOBBY_VIEW_RANGE * 2.0);

    // The lobby floor (-25..25) as 1x1 world-unit tiles with a grid line on
    // each tile's top and left edge (matching the in-game 1-unit grid), each
    // tile faded on its own so none overflows the circle.
    let view = layer::View {
        center: vec2(center_x, center_y),
        half,
        player_cells: vec2(local_pos.x, local_pos.z),
        px_per_cell: ppwu, // 1 world unit per tile
        circular,
        fade_start: FADE_START,
    };
    let ground = layer::Ground::Lobby {
        floor_half: 25.0,
        floor: LOBBY_FLOOR_COLOR,
        line: LOBBY_GRID_COLOR,
    };
    layer::draw_layer(&view, ground);

    // Landmark cubes
    let landmark_colors = [RED, GREEN, BLUE, YELLOW];
    for (i, (pos, size)) in LANDMARK_CUBES.iter().enumerate() {
        let sx = center_x + (pos.x - local_pos.x) * ppwu;
        let sy = center_y + (pos.z - local_pos.z) * ppwu;
        let alpha = fade_alpha(sx, sy, center_x, center_y, half, circular);
        if alpha <= 0.0 {
            continue;
        }
        let w = size.x * ppwu;
        let h = size.z * ppwu;
        let c = landmark_colors[i];
        let color = Color::new(c.r, c.g, c.b, alpha);
        draw_rectangle(sx - w / 2.0, sy - h / 2.0, w, h, color);
    }

    // Parkour cubes
    for (i, (pos, size)) in PARKOUR_CUBES.iter().enumerate() {
        let sx = center_x + (pos.x - local_pos.x) * ppwu;
        let sy = center_y + (pos.z - local_pos.z) * ppwu;
        let alpha = fade_alpha(sx, sy, center_x, center_y, half, circular);
        if alpha <= 0.0 {
            continue;
        }
        let t = i as f32 / (PARKOUR_CUBES.len() - 1) as f32;
        let c = Color::new(0.2 + 0.6 * t, 0.8 - 0.5 * t, 0.9 - 0.3 * t, alpha);
        let w = size.x * ppwu;
        let h = size.z * ppwu;
        draw_rectangle(sx - w / 2.0, sy - h / 2.0, w, h, c);
    }

    // Lobby console (orange accent icon on minimap)
    {
        let sx = center_x + (LOBBY_CONSOLE_POS.x - local_pos.x) * ppwu;
        let sy = center_y + (LOBBY_CONSOLE_POS.z - local_pos.z) * ppwu;
        let alpha = fade_alpha(sx, sy, center_x, center_y, half, circular);
        if alpha > 0.0 {
            let w = LOBBY_CONSOLE_SIZE.x * ppwu;
            let h = LOBBY_CONSOLE_SIZE.z * ppwu;
            // orange fill matching the UI accent
            let fill = Color::new(1.0, 0.42, 0.21, alpha * 0.9);
            draw_rectangle(sx - w / 2.0, sy - h / 2.0, w, h, fill);
            // bright border for visibility
            let border = Color::new(1.0, 0.55, 0.38, alpha);
            draw_rectangle_lines(
                sx - w / 2.0 - 1.0,
                sy - h / 2.0 - 1.0,
                w + 2.0,
                h + 2.0,
                1.0,
                border,
            );
        }
    }
}

// ─── Shared drawing helpers ───────────────────────────────────────────────────

fn pixels_per_world_unit(world: WorldId, maze: Option<&Maze>, diameter: f32) -> f32 {
    match world {
        WorldId::Maze => {
            if let Some(m) = maze {
                diameter / (m.cell_size * 33.0)
            } else {
                diameter / (LOBBY_VIEW_RANGE * 2.0)
            }
        }
        WorldId::Lobby => diameter / (LOBBY_VIEW_RANGE * 2.0),
    }
}

fn draw_other_players(
    other_players: &[OtherPlayer],
    current_world: WorldId,
    local_pos: Vec3,
    center_x: f32,
    center_y: f32,
    half: f32,
    circular: bool,
    ppwu: f32,
    local_team: u8,
    smoke_hunters: Option<&[u32]>,
) {
    for p in other_players {
        if p.world != current_world || !p.is_alive() {
            continue;
        }

        // If we're in the Maze and have a non-spectator team, only draw
        // teammates on the minimap so enemies are not revealed there.
        if current_world == WorldId::Maze && local_team != 0 {
            if p.team != local_team {
                continue;
            }
        }
        if let Some(hunters) = smoke_hunters { if !hunters.contains(&p.id) { continue; } }

        let sx = center_x + (p.smooth.x - local_pos.x) * ppwu;
        let sy = center_y + (p.smooth.z - local_pos.z) * ppwu;

        let alpha = fade_alpha(sx, sy, center_x, center_y, half, circular);
        if alpha <= 0.0 {
            continue;
        }

        let dot_size = (ppwu * 0.9).clamp(3.0, 6.0);
        let c = p.color;
        // a 3-6 px dot: 12 sides is visually a circle
        fill_disc(sx, sy, dot_size, 12, Color::new(c.r, c.g, c.b, c.a * alpha));
        ring(sx, sy, dot_size, 1.0, 12, Color::new(0.0, 0.0, 0.0, 0.5 * alpha));
    }
}

/// Draw the local player arrow at center, rotated by yaw.
fn draw_player_arrow(center_x: f32, center_y: f32, yaw: f32, ppwu: f32) {
    let nose = ppwu.clamp(5.0, 10.0) * 1.2;
    let wing = ppwu.clamp(3.0, 7.0) * 0.7;
    let tail = ppwu.clamp(2.5, 6.0) * 0.5;

    // Arrow points in the direction of yaw.
    // In the game: yaw=0 faces +X, yaw=PI/2 faces +Z.
    // On the minimap: +X is right, +Z is down.
    // So screen angle = yaw - PI/2 (rotate from "up" to the game yaw direction).
    let angle = yaw + std::f32::consts::FRAC_PI_2;
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    // Vertices relative to center (unrotated: pointing up = -Y)
    let raw_tip = (0.0, -nose);
    let raw_left = (-wing, tail);
    let raw_right = (wing, tail);
    let raw_notch = (0.0, tail * 0.15);

    let rot = |p: (f32, f32)| -> Vec2 {
        vec2(
            center_x + p.0 * cos_a - p.1 * sin_a,
            center_y + p.0 * sin_a + p.1 * cos_a,
        )
    };

    let tip = rot(raw_tip);
    let left = rot(raw_left);
    let right = rot(raw_right);
    let notch = rot(raw_notch);

    // Chevron with notch
    draw_triangle(tip, right, notch, LOCAL_PLAYER_COLOR);
    draw_triangle(tip, notch, left, LOCAL_PLAYER_COLOR);

    let outline = Color::new(0.0, 0.0, 0.0, 0.4);
    draw_line(tip.x, tip.y, right.x, right.y, 1.0, outline);
    draw_line(right.x, right.y, notch.x, notch.y, 1.0, outline);
    draw_line(notch.x, notch.y, left.x, left.y, 1.0, outline);
    draw_line(left.x, left.y, tip.x, tip.y, 1.0, outline);
}

// ─── Utilities ────────────────────────────────────────────────────────────────

fn fade_alpha(sx: f32, sy: f32, center_x: f32, center_y: f32, half: f32, circular: bool) -> f32 {
    if circular {
        let dist = ((sx - center_x).powi(2) + (sy - center_y).powi(2)).sqrt();
        smoothstep_fade(dist / half)
    } else {
        let tx = (sx - center_x).abs() / half;
        let ty = (sy - center_y).abs() / half;
        smoothstep_fade(tx.max(ty))
    }
}

/// Returns 1.0 for t <= FADE_START, smoothly fades to 0.0 at t >= 1.0.
fn smoothstep_fade(t: f32) -> f32 {
    if t >= 1.0 {
        0.0
    } else if t <= FADE_START {
        1.0
    } else {
        let f = (1.0 - t) / (1.0 - FADE_START);
        f * f * (3.0 - 2.0 * f)
    }
}
