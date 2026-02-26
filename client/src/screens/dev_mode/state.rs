use crate::ui::fonts::text_params;
use macroquad::prelude::*;

const DEV_CODE: &str = "verso";
pub(super) const TAB_NAMES: &[&str] = &[
    "Weapon",
    "Crosshair",
    "Movement",
    "Viewmodel",
    "Debug",
    "Bots",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandsAnimRequest {
    IdleLoop,
    ReloadOnce,
    Stop,
}

pub struct DevModeState {
    pub active_tab: usize,
    pub scroll_offset: f32,
    pub content_height: f32, // height of last-drawn tab content, used to clamp scroll
    pub infinite_ammo: bool,
    pub no_recoil: bool,
    pub show_hitboxes: bool,
    pub god_mode: bool,
    pub show_health_bars: bool,
    pub code_input: String,
    pub code_verified: bool,
    pub show_code_prompt: bool,
    pub hands_anim_request: Option<HandsAnimRequest>,
    pub precision_mode: bool, // viewmodel: shrinks all step sizes by 10x for micro-adjustments
    pub force_ads: bool,      // simulate ADS being held for viewmodel preview
    pub viewport_drag_active: bool, // viewmodel: drag on viewport to adjust pos/rot
    pub viewport_drag_prev_mouse: (f32, f32), // last frame mouse pos for delta calc
    pub ruler_active: bool,
    pub ruler_start: Option<Vec3>,
    pub ruler_end: Option<Vec3>,
    pub ruler_set_start_requested: bool,
    pub ruler_set_end_requested: bool,
    // world-weapon adjustment knobs (local space)
    pub weapon_pos_offset: (f32, f32, f32),
    pub weapon_rot_offset: (f32, f32, f32),
    // additional yaw tweak applied when moving (walk/run)
    pub weapon_yaw_adjust_move: f32,
    // yaw tweak while aiming down sights
    pub weapon_yaw_adjust_ads: f32,
}

impl DevModeState {
    pub fn new() -> Self {
        Self {
            active_tab: 0,
            scroll_offset: 0.0,
            content_height: 9999.0,
            infinite_ammo: false,
            no_recoil: false,
            show_hitboxes: false,
            god_mode: false,
            show_health_bars: false,
            code_input: String::new(),
            code_verified: false,
            show_code_prompt: false,
            hands_anim_request: None,
            precision_mode: false,
            force_ads: false,
            viewport_drag_active: false,
            viewport_drag_prev_mouse: (0.0, 0.0),
            ruler_active: false,
            ruler_start: None,
            ruler_end: None,
            ruler_set_start_requested: false,
            ruler_set_end_requested: false,
            // default offset: mirror the baseline adjustment used when dev mode is locked
            // (X right/left, Y up/down, Z forward/back) – note that Z sign flips
            // depending on movement/ADS state so the slider value is the magnitude.
            weapon_pos_offset: (-0.14, -0.07, -0.01),
            weapon_rot_offset: (0.0, 0.0, 0.0),
            weapon_yaw_adjust_move: std::f32::consts::PI / 2.0,
            weapon_yaw_adjust_ads: 0.0,
        }
    }

    pub fn request_open(&mut self) {
        if self.code_verified {
            self.show_code_prompt = false;
        } else {
            self.code_input.clear();
            self.show_code_prompt = true;
            while get_char_pressed().is_some() {}
        }
    }

    pub fn is_unlocked(&self) -> bool {
        self.code_verified && !self.show_code_prompt
    }

    pub fn update_code_prompt(state: &mut DevModeState) -> bool {
        if !state.show_code_prompt {
            state.code_input.clear();
            return false;
        }

        while let Some(c) = get_char_pressed() {
            if c.is_alphanumeric() {
                state.code_input.push(c);
            }
        }

        if state.code_input.len() > 20 {
            state.code_input.truncate(20);
        }

        if is_key_pressed(KeyCode::Backspace) {
            state.code_input.pop();
        }

        if is_key_pressed(KeyCode::Escape) {
            state.show_code_prompt = false;
            return false;
        }

        if is_key_pressed(KeyCode::Enter) {
            if state.code_input == DEV_CODE {
                state.code_verified = true;
                state.show_code_prompt = false;
                return true;
            } else {
                state.code_input.clear();
            }
        }
        false
    }

    pub fn draw_code_prompt(state: &DevModeState) {
        if !state.show_code_prompt {
            return;
        }

        let sw = screen_width();
        let sh = screen_height();
        draw_rectangle(0.0, 0.0, sw, sh, Color::from_rgba(0, 0, 0, 180));

        let bw = 360.0;
        let bh = 180.0;
        let bx = (sw - bw) / 2.0;
        let by = (sh - bh) / 2.0;

        draw_rectangle(bx, by, bw, bh, Color::from_rgba(30, 30, 30, 255));
        draw_rectangle_lines(bx, by, bw, bh, 2.0, Color::from_rgba(80, 150, 220, 255));

        draw_text_ex("Enter Code", bx + 90.0, by + 35.0, text_params(22, WHITE));

        let ix = bx + 30.0;
        let iy = by + 60.0;
        let iw = bw - 60.0;
        let ih = 36.0;
        draw_rectangle(ix, iy, iw, ih, Color::from_rgba(15, 15, 15, 255));
        draw_rectangle_lines(ix, iy, iw, ih, 1.0, GRAY);

        let dots: String = std::iter::repeat('*')
            .take(state.code_input.len())
            .collect();
        draw_text_ex(&dots, ix + 10.0, iy + 24.0, text_params(22, WHITE));

        draw_text_ex(
            "Press ENTER to SUBMIT, ESC to CANCEL",
            bx + 40.0,
            by + 130.0,
            text_params(14, GRAY),
        );
    }
}
