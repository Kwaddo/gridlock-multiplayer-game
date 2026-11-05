use crate::ui::fonts::{measure_game_text, text_params};

use macroquad::prelude::*;

use super::{in_rect, LoginScreen, ACCENT, ACCENT_HOVER, INPUT_BG, INPUT_BORDER, TEXT_DIM, TEXT_LABEL};
use crate::core::game_state::LoginField;

pub(super) const FIELD_H: f32 = 34.0;
const BUTTON_TEXT: Color = Color::new(0.05, 0.05, 0.05, 1.0);

impl LoginScreen {
    /// Labelled text input bound to `field`; clicking it focuses it.
    pub(super) fn text_field(&mut self, field: LoginField, label: &str, placeholder: &str, x: f32, y: f32, w: f32) {
        let active = self.active_field == field;
        draw_text_ex(label, x, y, text_params(13, if active { ACCENT } else { TEXT_LABEL }));

        let fy = y + 10.0;
        draw_rectangle(x, fy, w, FIELD_H, INPUT_BG);
        draw_rectangle_lines(x, fy, w, FIELD_H, if active { 2.0 } else { 1.0 }, if active { ACCENT } else { INPUT_BORDER });

        let text = self.field_text(field);
        if text.is_empty() && !active {
            draw_text_ex(placeholder, x + 10.0, fy + 23.0, text_params(17, Color::new(0.30, 0.32, 0.42, 0.5)));
        } else {
            draw_text_ex(text, x + 10.0, fy + 23.0, text_params(17, WHITE));
            if active && (self.anim_time * 2.5) as i32 % 2 == 0 {
                let tw = measure_game_text(text, 17);
                draw_rectangle(x + 10.0 + tw + 2.0, fy + 6.0, 2.0, FIELD_H - 12.0, ACCENT);
            }
        }

        if is_mouse_button_pressed(MouseButton::Left) && in_rect(mouse_position(), x, fy, w, FIELD_H) {
            self.active_field = field;
        }
    }

    pub(super) fn field_text(&self, field: LoginField) -> &str {
        match field {
            LoginField::Username => &self.username_input,
            LoginField::Address => &self.address_input,
            LoginField::ServerName => &self.server_name_input,
        }
    }

    pub(super) fn field_text_mut(&mut self, field: LoginField) -> &mut String {
        match field {
            LoginField::Username => &mut self.username_input,
            LoginField::Address => &mut self.address_input,
            LoginField::ServerName => &mut self.server_name_input,
        }
    }
}

/// Filled accent button; returns true on the frame it is clicked.
pub(super) fn button(label: &str, size: u16, x: f32, y: f32, w: f32, h: f32) -> bool {
    let hover = in_rect(mouse_position(), x, y, w, h);
    draw_rectangle(x, y, w, h, if hover { ACCENT_HOVER } else { ACCENT });
    let tw = measure_game_text(label, size);
    draw_text_ex(label, x + (w - tw) / 2.0, y + h / 2.0 + size as f32 * 0.35, text_params(size, BUTTON_TEXT));
    hover && is_mouse_button_pressed(MouseButton::Left)
}

/// Checkbox with a label; flips `value` when clicked unless `enabled` is false.
pub(super) fn checkbox(value: &mut bool, enabled: bool, label: &str, x: f32, y: f32) {
    let size = 18.0;
    let label_w = measure_game_text(label, 14);
    let hover = enabled && in_rect(mouse_position(), x, y, size + 10.0 + label_w, size);
    if hover && is_mouse_button_pressed(MouseButton::Left) {
        *value = !*value;
    }
    draw_rectangle(x, y, size, size, INPUT_BG);
    draw_rectangle_lines(x, y, size, size, 1.0, if hover { ACCENT } else { INPUT_BORDER });
    if *value && enabled {
        draw_rectangle(x + 4.0, y + 4.0, size - 8.0, size - 8.0, ACCENT);
    }
    let color = if enabled { TEXT_LABEL } else { TEXT_DIM };
    draw_text_ex(label, x + size + 10.0, y + 14.0, text_params(14, color));
}
