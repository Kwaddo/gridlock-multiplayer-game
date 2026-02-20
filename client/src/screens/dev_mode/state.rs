use crate::ui::fonts::text_params;
use macroquad::prelude::*;

const DEV_CODE: &str = "verso";
pub const HANDS_DEFAULT_SCALE: f32 = 1.93;
pub const HANDS_DEFAULT_OFFSET_X: f32 = 0.16;
pub const HANDS_DEFAULT_OFFSET_Y: f32 = -0.03;
pub const HANDS_DEFAULT_OFFSET_Z: f32 = -0.18;
pub const HANDS_DEFAULT_ROT_PITCH: f32 = -0.15;
pub const HANDS_DEFAULT_ROT_YAW: f32 = 0.65;
pub const HANDS_DEFAULT_ROT_ROLL: f32 = -0.05;
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
    pub infinite_ammo: bool,
    pub no_recoil: bool,
    pub show_hitboxes: bool,
    pub god_mode: bool,
    pub show_health_bars: bool,
    pub code_input: String,
    pub code_verified: bool,
    pub show_code_prompt: bool,
    // hands tuning (viewmodel tab)
    pub hands_scale: f32,
    pub hands_offset_x: f32,
    pub hands_offset_y: f32,
    pub hands_offset_z: f32,
    pub hands_rot_pitch: f32,
    pub hands_rot_yaw: f32,
    pub hands_rot_roll: f32,
    pub hands_anim_request: Option<HandsAnimRequest>,
}

impl DevModeState {
    pub fn new() -> Self {
        Self {
            active_tab: 0,
            scroll_offset: 0.0,
            infinite_ammo: false,
            no_recoil: false,
            show_hitboxes: false,
            god_mode: false,
            show_health_bars: false,
            code_input: String::new(),
            code_verified: false,
            show_code_prompt: false,
            hands_scale: HANDS_DEFAULT_SCALE,
            hands_offset_x: HANDS_DEFAULT_OFFSET_X,
            hands_offset_y: HANDS_DEFAULT_OFFSET_Y,
            hands_offset_z: HANDS_DEFAULT_OFFSET_Z,
            hands_rot_pitch: HANDS_DEFAULT_ROT_PITCH,
            hands_rot_yaw: HANDS_DEFAULT_ROT_YAW,
            hands_rot_roll: HANDS_DEFAULT_ROT_ROLL,
            hands_anim_request: None,
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
