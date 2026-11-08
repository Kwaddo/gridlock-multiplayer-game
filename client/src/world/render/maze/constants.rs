use macroquad::prelude::*;

// ------------------------------------------------------------------ colours

pub(super) const FLOOR_LIGHT: Color = Color::new(0.74, 0.72, 0.69, 1.0);
pub(super) const FLOOR_DARK: Color = Color::new(0.66, 0.64, 0.60, 1.0);
pub(super) const WALL_COLOR: Color = Color::new(0.80, 0.82, 0.84, 1.0);
pub(super) const WALL_BREAKABLE: Color = Color::new(0.85, 0.82, 0.74, 1.0);
pub(super) const BRACE_COLOR: Color = Color::new(0.86, 0.70, 0.22, 1.0);
pub(super) const ROOF_COLOR: Color = Color::new(0.23, 0.24, 0.26, 1.0);
pub(super) const CEILING_PANEL_LIGHT: Color = Color::new(0.30, 0.31, 0.34, 1.0);
pub(super) const CEILING_PANEL_DARK: Color = Color::new(0.26, 0.27, 0.30, 1.0);
pub(super) const PIPE_COLOR: Color = Color::new(0.50, 0.53, 0.56, 1.0);
pub(super) const PIPE_JOINT_COLOR: Color = Color::new(0.38, 0.40, 0.44, 1.0);
pub(super) const LIGHT_MOUNT_COLOR: Color = Color::new(0.72, 0.74, 0.78, 1.0);
pub(super) const LIGHT_STEM_COLOR: Color = Color::new(0.60, 0.62, 0.66, 1.0);
pub(super) const HOUSING_ON_COLOR: Color = Color::new(0.86, 0.86, 0.88, 1.0);
pub(super) const HOUSING_DEAD_COLOR: Color = Color::new(0.30, 0.30, 0.32, 1.0);
pub(super) const BULB_DEAD_COLOR: Color = Color::new(0.15, 0.14, 0.12, 1.0);
/// Shown where there is no maze yet.
pub(super) const PLACEHOLDER_COLOR: Color = GRAY;

// --------------------------------------------------------------- dimensions

pub(super) const ROOF_THICKNESS: f32 = 0.06;
pub(super) const ROOF_CLEARANCE: f32 = 0.05;
pub(super) const PANEL_INSET: f32 = 0.03;
pub(super) const PANEL_THICKNESS: f32 = 0.03;
pub(super) const PANEL_DROP: f32 = 0.018;
pub(super) const PIPE_ROW_SPACING: usize = 6;
pub(super) const PIPE_COL_SPACING: usize = 6;
/// Ceiling light position below the roof bottom.
pub(super) const LIGHT_DROP: f32 = 0.16;
pub(super) const LIGHT_INTENSITY: f32 = 0.38;

// ------------------------------------------------------------------ culling

/// Cells per chunk side; one chunk is one draw call.
pub(super) const CHUNK_CELLS: usize = 8;
/// Consecutive props (same chunk) baked into one draw call at most.
pub(super) const PROPS_PER_GROUP: usize = 16;
/// Frame time spent recolouring the maze after a light goes out; the rest
/// waits for later frames.
pub(super) const RELIGHT_BUDGET_SECS: f64 = 0.002;
/// The render distance never drops below this many cells.
pub(super) const MIN_RENDER_CELLS: f32 = 10.0;
/// Cells are drawn up to this many cells past the render distance.
pub(super) const RENDER_SLACK_CELLS: f32 = 1.5;
/// Mesh bounds grow by this much before the frustum test, so float error
/// can never cull something on screen.
pub(super) const FRUSTUM_MARGIN: f32 = 0.25;
/// Relative slack of the per-mesh distance pre-test (the exact per-cell test
/// runs on the GPU).
pub(super) const DISTANCE_SLACK: f32 = 1e-4;
/// Most vertices one draw call takes (16-bit indices).
pub(super) const MAX_MESH_VERTICES: usize = 65_532;

/// Colour of ceiling light `variant`.
pub(super) fn light_color(variant: u8) -> Vec3 {
    match variant % 3 {
        0 => vec3(0.95, 0.94, 0.88),
        1 => vec3(1.0, 0.95, 0.85),
        _ => vec3(0.88, 0.92, 0.98),
    }
}
