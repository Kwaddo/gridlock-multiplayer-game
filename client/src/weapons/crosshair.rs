/*
    Runtime crosshair configuration 
    did this so we can use change features like color, dot, line length, thickness etc without the need for a texture,
    eventually this will be editable in settings so players can customize their crosshair.
*/

use macroquad::prelude::Color;
use super::stats::WeaponKind;

pub struct CrosshairConfig {
    pub color: Color,
    pub dot_enabled: bool,
    pub dot_size: f32, // % of screen height
    pub line_length: f32, // % of screen height
    pub line_thickness: f32, // % of screen height
    pub center_gap: f32, // % of screen height — base hip-fire gap, shrinks to 0 when ADS
    pub outline_enabled: bool,
    pub outline_thickness: f32, // % of screen height

    // texture crosshair (sniper scope texture if we add one later)
    pub hip_texture_key: Option<String>,
    pub ads_texture_key: Option<String>,
    pub texture_base_size: f32, // % of screen height
    pub texture_ads_size: f32, // % of screen height

    pub kick_per_shot: f32, //  how much gap to add per shot (% of screen height)
    pub kick_decay_rate: f32, // how quickly the gap recovers back to normal after shooting (units per second)
    pub kick_max: f32,
    pub is_4curves: bool, // 4 curved arcs with gaps at cardinal directions (shotgun style)
}

impl CrosshairConfig {
    // default crosshair config per weapon, player can override later in settings
    pub fn for_weapon(kind: WeaponKind) -> Self {
        match kind {
            WeaponKind::Pistol =>
                Self {
                    color: Color::from_rgba(255, 255, 255, 255),
                    dot_enabled: true,
                    dot_size: 0.15,
                    line_length: 1.0,
                    line_thickness: 0.2,
                    center_gap: 0.5,
                    outline_enabled: true,
                    outline_thickness: 0.1,
                    hip_texture_key: None,
                    ads_texture_key: None,
                    texture_base_size: 0.0,
                    texture_ads_size: 0.0,
                    kick_per_shot: 1.5,
                    kick_decay_rate: 8.0,
                    kick_max: 3.0,
                    is_4curves: false,
                },
            WeaponKind::Smg =>
                Self {
                    color: Color::from_rgba(255, 255, 255, 255),
                    dot_enabled: true,
                    dot_size: 0.15,
                    line_length: 1.2,
                    line_thickness: 0.25,
                    center_gap: 0.7,
                    outline_enabled: true,
                    outline_thickness: 0.1,
                    hip_texture_key: None,
                    ads_texture_key: None,
                    texture_base_size: 0.0,
                    texture_ads_size: 0.0,
                    kick_per_shot: 0.8,
                    kick_decay_rate: 6.0,
                    kick_max: 2.5,
                    is_4curves: false,
                },
            WeaponKind::Shotgun =>
                Self {
                    color: Color::from_rgba(255, 255, 255, 255),
                    dot_enabled: true, 
                    dot_size: 0.15,
                    line_length: 1.8, 
                    line_thickness: 0.25,
                    center_gap: 2.5,
                    outline_enabled: true,
                    outline_thickness: 0.1,
                    hip_texture_key: None, 
                    ads_texture_key: None,
                    texture_base_size: 0.0,
                    texture_ads_size: 0.0,
                    kick_per_shot: 2.5,
                    kick_decay_rate: 8.0,
                    kick_max: 4.0,
                    is_4curves: true,
                },
            WeaponKind::AssaultRifle =>
                Self {
                    color: Color::from_rgba(255, 255, 255, 255),
                    dot_enabled: true,
                    dot_size: 0.15,
                    line_length: 1.1,
                    line_thickness: 0.25,
                    center_gap: 0.6,
                    outline_enabled: true,
                    outline_thickness: 0.1,
                    hip_texture_key: None,
                    ads_texture_key: None,
                    texture_base_size: 0.0,
                    texture_ads_size: 0.0,
                    kick_per_shot: 1.2,
                    kick_decay_rate: 5.0,
                    kick_max: 3.0,
                    is_4curves: false,
                },
            WeaponKind::Sniper =>
                Self {
                    color: Color::from_rgba(255, 255, 255, 255),
                    dot_enabled: false,
                    dot_size: 0.0,
                    line_length: 0.0,
                    line_thickness: 0.0,
                    center_gap: 0.0,
                    outline_enabled: false,
                    outline_thickness: 0.0,
                    // None for now — procedural scope. can swap to Some("scope_texture") later
                    hip_texture_key: None,
                    ads_texture_key: None,
                    texture_base_size: 0.0,
                    texture_ads_size: 0.0,
                    // N/A for sniper
                    kick_per_shot: 0.0,
                    kick_decay_rate: 0.0,
                    kick_max: 0.0,
                    is_4curves: false,
                },
        }
    }
}
