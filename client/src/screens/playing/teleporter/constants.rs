use macroquad::prelude::*;

// ── colour palette (matches login screen & settings) ──────────────────────────
pub(super) const MENU_BG_OVL: Color = Color::new(0.0, 0.0, 0.0, 0.72);
pub(super) const PANEL_BG: Color = Color::new(0.09, 0.11, 0.19, 0.97);
pub(super) const PANEL_BORDER: Color = Color::new(0.20, 0.24, 0.40, 0.7);
pub(super) const ACCENT: Color = Color::new(1.0, 0.42, 0.21, 1.0);
pub(super) const ACCENT_HOVER: Color = Color::new(1.0, 0.55, 0.38, 1.0);
pub(super) const ACCENT_DIM: Color = Color::new(1.0, 0.42, 0.21, 0.14);
pub(super) const TEXT_DIM: Color = Color::new(0.50, 0.52, 0.62, 1.0);
pub(super) const TEXT_LABEL: Color = Color::new(0.72, 0.74, 0.84, 1.0);
pub(super) const SEP: Color = Color::new(0.20, 0.24, 0.40, 0.5);
pub(super) const RED_TEAM: Color = Color::new(0.90, 0.40, 0.40, 1.0);
pub(super) const BLUE_TEAM: Color = Color::new(0.42, 0.62, 0.95, 1.0);
pub(super) const RED_PANEL: Color = Color::new(0.18, 0.07, 0.07, 0.95);
pub(super) const BLUE_PANEL: Color = Color::new(0.07, 0.09, 0.22, 0.95);
pub(super) const RED_BORDER: Color = Color::new(0.90, 0.40, 0.40, 0.35);
pub(super) const BLUE_BORDER: Color = Color::new(0.42, 0.62, 0.95, 0.35);
pub(super) const BTN_CLOSE_BG: Color = Color::new(0.12, 0.13, 0.22, 1.0);
pub(super) const BTN_CLOSE_HOVER: Color = Color::new(0.18, 0.20, 0.32, 1.0);
pub(super) const CONTROL_BG: Color = Color::new(0.12, 0.13, 0.22, 1.0);
pub(super) const CONTROL_HOVER: Color = Color::new(0.18, 0.20, 0.32, 1.0);

/// Height of each team's roster panel in the lobby console.
pub(super) const ROSTER_H: f32 = 118.0;
