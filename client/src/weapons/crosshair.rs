/*
    Runtime crosshair configuration
    did this so we can use change features like color, dot, line length, thickness etc without the need for a texture,
    eventually this will be editable in settings so players can customize their crosshair.
*/

use super::stats::WeaponCategory;
use macroquad::prelude::Color;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum CrosshairType {
    Cross,      // standard + crosshair lines
    FourCurves, // 4 curved arcs with gaps at cardinal directions
    Chevron,    // 4 L-bracket corners with bezier-chamfered diagonals
    Circle,
    Square,
    Triangle,
    AngleBrackets,
}

#[derive(Clone, PartialEq)]
pub struct CrosshairConfig {
    pub color: Color,
    pub dot_enabled: bool,
    pub dot_size: f32,       // % of screen height
    pub line_length: f32,    // % of screen height
    pub line_thickness: f32, // % of screen height
    pub center_gap: f32,     // % of screen height — base hip-fire gap, shrinks to 0 when ADS
    pub outline_enabled: bool,
    pub outline_thickness: f32, // % of screen height
    pub crosshair_type: CrosshairType,
}

impl CrosshairConfig {
    // default crosshair config per weapon category, player can override later in settings
    // visual values are unified — only crosshair_type and shotgun gap differ
    pub fn for_category(cat: WeaponCategory) -> Self {
        match cat {
            WeaponCategory::Shotgun => Self {
                color: Color::from_rgba(255, 255, 255, 255),
                dot_enabled: true,
                dot_size: 0.15,
                line_length: 1.0,
                line_thickness: 0.2,
                center_gap: 3.5,
                outline_enabled: true,
                outline_thickness: 0.1,
                crosshair_type: CrosshairType::FourCurves,
            },
            WeaponCategory::Sniper => Self {
                color: Color::from_rgba(255, 255, 255, 255),
                dot_enabled: false,
                dot_size: 0.0,
                line_length: 0.0,
                line_thickness: 0.0,
                center_gap: 0.0,
                outline_enabled: false,
                outline_thickness: 0.0,
                crosshair_type: CrosshairType::Cross,
            },
            // Pistol, SMG, AR, Other all share the same Cross defaults
            _ => Self {
                color: Color::from_rgba(255, 255, 255, 255),
                dot_enabled: true,
                dot_size: 0.15,
                line_length: 1.0,
                line_thickness: 0.2,
                center_gap: 0.5,
                outline_enabled: true,
                outline_thickness: 0.1,
                crosshair_type: CrosshairType::Cross,
            },
        }
    }
}
