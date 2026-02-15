use crate::weapons::stats::WeaponKind;
use crate::weapons::crosshair::CrosshairType;
use macroquad::prelude::*;

pub(super) const CROSSHAIR_TYPES: &[CrosshairType] = &[
    CrosshairType::Cross,
    CrosshairType::FourCurves,
    CrosshairType::Chevron,
    CrosshairType::Circle,
    CrosshairType::Square,
    CrosshairType::Triangle,
    CrosshairType::AngleBrackets,
];

pub(super) fn crosshair_type_name(ct: CrosshairType) -> &'static str {
    match ct {
        CrosshairType::Cross => "Cross",
        CrosshairType::FourCurves => "4-Curves",
        CrosshairType::Chevron => "Chevron",
        CrosshairType::Circle => "Circle",
        CrosshairType::Square => "Square",
        CrosshairType::Triangle => "Triangle",
        CrosshairType::AngleBrackets => "Angle Brackets",
    }
}

pub(super) fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta < 0.0001 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };

    let s = if max < 0.0001 { 0.0 } else { delta / max };
    let v = max;
    (h, s, v)
}

pub(super) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (r1 + m, g1 + m, b1 + m)
}

pub struct CrosshairSettingsPreview {
    pub preview_weapon: WeaponKind,
    pub preview_ads: bool,
    pub preview_light_bg: bool,
    pub shotgun_type_warning: bool,
    pub shotgun_warning_shown_this_visit: bool,
    pub hue: f32,
    pub sat: f32,
    pub val: f32,
    pub alpha_val: f32,
    pub color_synced: bool,
    pub selected_preset: usize,
    pub naming_mode: bool,
    pub name_input: String,
    pub show_save_prompt: bool,
    pub save_prompt_is_leave: bool,
    pub name_conflict_index: Option<usize>,
    pub name_error: String,
    pub pending_tab: Option<u8>,
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

    pub(super) fn sync_from_color(&mut self, color: Color) {
        let (h, s, v) = rgb_to_hsv(color.r, color.g, color.b);
        self.hue = h;
        self.sat = s;
        self.val = v;
        self.alpha_val = color.a * 255.0;
        self.color_synced = true;
    }
}
