/* 
    This file contains the game configuration, such as keybinds and settings. 
    did this so we can easily have settings menu in the future where players can change these values.
*/

use macroquad::prelude::*;

pub struct GameConfig {
    // Mouse (UI value 10-100, default 60)
    pub sensitivity: f32,

    // Keybinds
    pub key_forward: KeyCode,
    pub key_backward: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_sprint: KeyCode,
    pub key_crouch: KeyCode,
    pub key_jump: KeyCode,
    pub key_settings: KeyCode,

    // Camera (UI value 40-120, default 90)
    pub fov: f32,

    // Player
    pub standing_height: f32,
    pub crouch_height: f32,
}

impl GameConfig {
    pub fn defaults() -> Self {
        Self {
            // Mouse - UI value, converted to internal with raw_sensitivity()
            sensitivity: 60.0,

            // Keybinds
            key_forward: KeyCode::W,
            key_backward: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_sprint: KeyCode::LeftShift,
            key_crouch: KeyCode::LeftControl,
            key_jump: KeyCode::Space,
            key_settings: KeyCode::Escape,

            // Camera - UI value, converted to internal with raw_fov()
            fov: 90.0,

            // Player
            standing_height: 1.0,
            crouch_height: 0.5,
        }
    }

    pub fn new() -> Self {
        Self::defaults()
    }

    pub fn reset(&mut self) {
        *self = Self::defaults();
    }

    // Convert UI sensitivity (10-100) to the actual mouse multiplier
    // UI 60 = old 0.002
    pub fn raw_sensitivity(&self) -> f32 {
        self.sensitivity * (0.002 / 60.0)
    }

    // Convert UI FOV (40-120) to the internal camera fovy
    // UI 90 = old 45.0 (half the UI value)
    pub fn raw_fov(&self) -> f32 {
        self.fov / 2.0
    }
}

