use macroquad::prelude::*;

// ── colour palette (matches login screen, settings & lobby console) ──────────
pub(super) const OVERLAY: Color = Color::new(0.0, 0.0, 0.0, 0.35);
pub(super) const PANEL_BG: Color = Color::new(0.09, 0.11, 0.19, 0.92);
pub(super) const PANEL_BORDER: Color = Color::new(0.20, 0.24, 0.40, 0.7);
pub(super) const ACCENT: Color = Color::new(1.0, 0.42, 0.21, 1.0);
pub(super) const ACCENT_DIM: Color = Color::new(1.0, 0.42, 0.21, 0.22);
pub(super) const TEXT_DIM: Color = Color::new(0.50, 0.52, 0.62, 1.0);
pub(super) const TEXT_LABEL: Color = Color::new(0.72, 0.74, 0.84, 1.0);

// ── geometry (at gui scale 1.0) ───────────────────────────────────────────────
pub(super) const RING_INNER: f32 = 72.0;
pub(super) const RING_OUTER: f32 = 168.0;
pub(super) const HUB_RADIUS: f32 = 60.0;
/// Angle left empty between neighbouring slices (radians).
pub(super) const SLICE_GAP: f32 = 0.035;
/// How far the hovered slice slides outwards.
pub(super) const HOVER_PUSH: f32 = 8.0;
pub(super) const ARC_SEGMENTS: u32 = 24;
pub(super) const LABEL_FONT: u16 = 16;
pub(super) const HUB_TITLE_FONT: u16 = 12;
pub(super) const HUB_NAME_FONT: u16 = 18;

// ── input ─────────────────────────────────────────────────────────────────────
/// Cursor distance below which releasing cancels.
pub(super) const DEADZONE: f32 = 30.0;
pub(super) const CURSOR_MAX: f32 = 120.0;

// ── animation ─────────────────────────────────────────────────────────────────
/// Seconds to open fully (closing runs the same curve backwards).
pub(super) const OPEN_SECS: f32 = 0.14;
/// Wheel scale when fully closed; it grows to 1.0 as it opens.
pub(super) const CLOSED_SCALE: f32 = 0.86;
/// How fast a slice's highlight follows the cursor (higher = snappier).
pub(super) const HOVER_SPEED: f32 = 18.0;
