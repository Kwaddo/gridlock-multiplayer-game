use macroquad::prelude::*;

use super::constants::{ACCENT, CONTROL_BG, CONTROL_HOVER, PANEL_BORDER, TEXT_DIM, TEXT_LABEL};
use super::PlayingState;

impl PlayingState {
    /// "BOT DIFFICULTY" filter: a summary of the enabled difficulties and a
    /// dropdown of checkboxes, `w` wide from (`x`, `y`).
    pub(super) fn draw_bot_difficulty(&mut self, x: f32, y: f32, w: f32, mouse: (f32, f32), clicked: bool) {
        use crate::ui::fonts::{measure_game_text, text_params};

        let h = 22.0;
        let hov = mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h;
        draw_rectangle(x, y, w, h, if hov || self.autofill_dropdown_open { CONTROL_HOVER } else { CONTROL_BG });
        draw_rectangle_lines(x, y, w, h, 1.0, if self.autofill_dropdown_open { ACCENT } else { PANEL_BORDER });
        if hov && clicked {
            self.autofill_dropdown_open = !self.autofill_dropdown_open;
        }

        draw_text_ex("DIFFICULTY", x + 8.0, y + 15.0, text_params(12, TEXT_LABEL));

        let mut active: Vec<&str> = Vec::new();
        if self.autofill_diff_recruit { active.push("Rec"); }
        if self.autofill_diff_easy { active.push("Easy"); }
        if self.autofill_diff_normal { active.push("Norm"); }
        if self.autofill_diff_hard { active.push("Hard"); }
        if self.autofill_diff_nightmare { active.push("NM"); }
        let summary = if active.len() == 5 {
            "All".to_string()
        } else if active.is_empty() {
            "None!".to_string()
        } else {
            active.join(", ")
        };
        let arrow = if self.autofill_dropdown_open { "^" } else { "v" };
        let aw = measure_game_text(arrow, 12);
        draw_text_ex(arrow, x + w - aw - 8.0, y + 15.0, text_params(12, TEXT_LABEL));
        draw_text_ex(
            &summary,
            x + 92.0,
            y + 15.0,
            text_params(12, if active.is_empty() { Color::new(1.0, 0.3, 0.3, 1.0) } else { TEXT_DIM }),
        );

        if !self.autofill_dropdown_open {
            return;
        }

        // dropdown panel, drawn over whatever is below
        let panel_x = x;
        let panel_y = y + h + 2.0;
        let panel_w = w.max(300.0);
        let row_h = 24.0;
        let panel_h = row_h * 5.0 + 8.0;
        draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.08, 0.09, 0.16, 0.98));
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, PANEL_BORDER);

        let difficulties: [(&str, &str); 5] = [
            ("Recruit", "Slowest reaction, worst aim"),
            ("Easy", "Below average skill"),
            ("Normal", "Average players"),
            ("Hard", "Skilled opponents"),
            ("Nightmare", "Near-perfect aim & reactions"),
        ];
        let flags: [&mut bool; 5] = [
            &mut self.autofill_diff_recruit,
            &mut self.autofill_diff_easy,
            &mut self.autofill_diff_normal,
            &mut self.autofill_diff_hard,
            &mut self.autofill_diff_nightmare,
        ];
        for (i, ((name, desc), flag)) in difficulties.iter().zip(flags).enumerate() {
            let row_y = panel_y + 4.0 + i as f32 * row_h;
            let cb_x = panel_x + 8.0;
            let cb = 16.0;
            let row_hov = mouse.0 >= panel_x
                && mouse.0 <= panel_x + panel_w
                && mouse.1 >= row_y
                && mouse.1 <= row_y + row_h;
            if row_hov {
                draw_rectangle(panel_x + 1.0, row_y, panel_w - 2.0, row_h, Color::new(0.15, 0.17, 0.28, 0.8));
            }
            draw_rectangle(cb_x, row_y + 4.0, cb, cb, CONTROL_BG);
            draw_rectangle_lines(cb_x, row_y + 4.0, cb, cb, 1.0, if *flag { ACCENT } else { PANEL_BORDER });
            if *flag {
                draw_rectangle(cb_x + 3.0, row_y + 7.0, cb - 6.0, cb - 6.0, ACCENT);
            }
            draw_text_ex(name, cb_x + cb + 8.0, row_y + 16.0, text_params(13, if *flag { WHITE } else { TEXT_DIM }));
            draw_text_ex(desc, cb_x + cb + 90.0, row_y + 16.0, text_params(11, Color::new(0.45, 0.47, 0.58, 0.8)));
            if row_hov && clicked {
                *flag = !*flag;
            }
        }
    }
}

/// A `- N BOTS +` stepper filling `w` x `h`, clamped to `0..=max`. `accent`
/// colours the buttons and count; `border` frames it.
#[allow(clippy::too_many_arguments)]
pub(super) fn bot_stepper(
    value: &mut u8,
    max: u8,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    accent: Color,
    border: Color,
    mouse: (f32, f32),
    clicked: bool,
) {
    use crate::ui::fonts::{measure_game_text, text_params};

    draw_rectangle(x, y, w, h, CONTROL_BG);
    draw_rectangle_lines(x, y, w, h, 1.0, border);

    let inside = |bx: f32| mouse.0 >= bx && mouse.0 <= bx + h && mouse.1 >= y && mouse.1 <= y + h;
    let button = |bx: f32, label: &str, enabled: bool| -> bool {
        let hov = enabled && inside(bx);
        let bg = if !enabled {
            Color::new(accent.r, accent.g, accent.b, 0.10)
        } else if hov {
            Color::new(accent.r, accent.g, accent.b, 0.45)
        } else {
            Color::new(accent.r, accent.g, accent.b, 0.22)
        };
        draw_rectangle(bx + 3.0, y + 3.0, h - 6.0, h - 6.0, bg);
        let lw = measure_game_text(label, 18);
        let lc = if enabled { WHITE } else { TEXT_DIM };
        draw_text_ex(label, bx + (h - lw) / 2.0, y + h / 2.0 + 6.0, text_params(18, lc));
        hov && clicked
    };
    if button(x, "-", *value > 0) {
        *value -= 1;
    }
    if button(x + w - h, "+", *value < max) {
        *value += 1;
    }

    let label = match *value {
        0 => "NO BOTS".to_string(),
        1 => "1 BOT".to_string(),
        n => format!("{} BOTS", n),
    };
    let lw = measure_game_text(&label, 15);
    let lc = if *value == 0 { TEXT_DIM } else { accent };
    draw_text_ex(&label, x + (w - lw) / 2.0, y + h / 2.0 + 5.0, text_params(15, lc));
}

/// A small outlined button; returns true when clicked.
pub(super) fn small_button(label: &str, x: f32, y: f32, w: f32, h: f32, mouse: (f32, f32), clicked: bool) -> bool {
    use crate::ui::fonts::{measure_game_text, text_params};

    let hov = mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h;
    draw_rectangle(x, y, w, h, if hov { CONTROL_HOVER } else { CONTROL_BG });
    draw_rectangle_lines(x, y, w, h, 1.0, if hov { ACCENT } else { PANEL_BORDER });
    let lw = measure_game_text(label, 12);
    draw_text_ex(label, x + (w - lw) / 2.0, y + h / 2.0 + 4.0, text_params(12, if hov { WHITE } else { TEXT_LABEL }));
    hov && clicked
}
