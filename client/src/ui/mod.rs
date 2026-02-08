use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

pub mod fonts;
pub mod hud;
pub use crate::types::player::OtherPlayer;

pub fn is_button_clicked(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    color: Color,
    hover_color: Color,
) -> bool {
    let mouse = mouse_position();
    let hover = mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h;
    draw_rectangle(x, y, w, h, if hover { hover_color } else { color });
    draw_rectangle_lines(x, y, w, h, 1.0, WHITE);
    let tw = measure_game_text(label, 20);
    draw_text_ex(label, x + (w - tw) / 2.0, y + 26.0, text_params(20, WHITE));
    hover && is_mouse_button_pressed(MouseButton::Left)
}
