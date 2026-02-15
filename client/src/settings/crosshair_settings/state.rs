use crate::weapons::stats::WeaponKind;
use macroquad::prelude::*;

use super::color::rgb_to_hsv;

pub struct CrosshairSettingsPreview {
    pub preview_weapon: WeaponKind,
    pub preview_ads: bool,
    pub preview_light_bg: bool,
    pub shotgun_type_warning: bool,
    pub shotgun_warning_shown_this_visit: bool,
    // color picker HSV state
    pub hue: f32,           // 0..360
    pub sat: f32,           // 0..1
    pub val: f32,           // 0..1
    pub alpha_val: f32,     // 0..255
    pub color_synced: bool, // false = need to sync HSV from crosshair color
    // preset state
    pub selected_preset: usize,
    pub naming_mode: bool,
    pub name_input: String,
    // save prompt state
    pub show_save_prompt: bool,
    pub save_prompt_is_leave: bool,
    pub name_conflict_index: Option<usize>,
    pub name_error: String,
    pub pending_tab: Option<u8>, // 0=General, 1=Keybinds — stored as u8 to avoid circular dependency on SettingsTab
}

impl CrosshairSettingsPreview {
    pub fn new() -> Self {
        Self {
            preview_weapon: WeaponKind::Pistol,
            preview_ads: false,
            preview_light_bg: false,
            shotgun_type_warning: false,
            shotgun_warning_shown_this_visit: false,
            hue: 0.0,
            sat: 0.0,
            val: 1.0,
            alpha_val: 255.0,
            color_synced: false,
            selected_preset: 0,
            naming_mode: false,
            name_input: String::new(),
            show_save_prompt: false,
            save_prompt_is_leave: false,
            name_conflict_index: None,
            name_error: String::new(),
            pending_tab: None,
        }
    }

    pub fn reset_visit(&mut self) {
        self.shotgun_warning_shown_this_visit = false;
        self.color_synced = false;
        self.show_save_prompt = false;
        self.naming_mode = false;
        self.name_input.clear();
        self.name_conflict_index = None;
        self.name_error.clear();
        self.pending_tab = None;
    }

    pub fn sync_from_color(&mut self, color: Color) {
        let (h, s, v) = rgb_to_hsv(color.r, color.g, color.b);
        self.hue = h;
        self.sat = s;
        self.val = v;
        self.alpha_val = color.a * 255.0;
        self.color_synced = true;
    }
}
