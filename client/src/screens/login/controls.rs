//! The main menu's controls reference: every key the game uses, with the
//! player's current bindings, and a way into the key binding settings.

use crate::core::GameConfig;
use crate::ui::fonts::{measure_game_text, text_params};

use macroquad::prelude::*;

use super::widgets::{button, outline_button};
use super::{LoginScreen, PANEL_BG, PANEL_BORDER, ROW_BG, TEXT_DIM, TEXT_LABEL};

/// What the player chose on the controls overlay.
pub enum ControlsAction {
    Close,
    /// Open the settings on the key bindings tab.
    EditKeys,
}

/// Keys with a fixed job, not in the key bindings.
const FIXED_KEYS: &[(&str, &str)] = &[
    ("Pause / back", "Esc"),
    ("Chat", "Enter"),
    ("Scoreboard (hold)", "Tab"),
    ("Use lobby console", "E"),
    ("Respawn (lobby)", "Space"),
];

const ROW_H: f32 = 25.0;

impl LoginScreen {
    /// Draws the overlay over the menu backdrop; `Some` once the player leaves it.
    pub fn draw_controls(&self, config: &GameConfig) -> Option<ControlsAction> {
        let sw = screen_width();
        let sh = screen_height();

        let mut rows: Vec<(String, String)> = config
            .all_bindings()
            .into_iter()
            .map(|(action, binding)| {
                let mode = match action {
                    "Crouch" if config.crouch_toggle => " (toggle)",
                    "Slow Walk" if config.slow_walk_toggle => " (toggle)",
                    "Prone" if config.prone_toggle => " (toggle)",
                    _ => "",
                };
                (format!("{action}{mode}"), pretty_key(&binding.display_name()))
            })
            .collect();
        rows.extend(FIXED_KEYS.iter().map(|&(a, k)| (a.to_string(), k.to_string())));

        let per_col = rows.len().div_ceil(2);
        let pw = 760.0_f32.min(sw - 32.0);
        let ph = 70.0 + per_col as f32 * ROW_H + 72.0;
        let px = (sw - pw) / 2.0;
        let py = ((sh - ph) / 2.0).max(8.0);
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.55));
        draw_rectangle(px, py, pw, ph, PANEL_BG);
        draw_rectangle_lines(px, py, pw, ph, 1.0, PANEL_BORDER);

        let title = "CONTROLS";
        let tw = measure_game_text(title, 24);
        draw_text_ex(title, px + (pw - tw) / 2.0, py + 40.0, text_params(24, WHITE));

        let pad = 24.0;
        let gap = 20.0;
        let col_w = (pw - pad * 2.0 - gap) / 2.0;
        let top = py + 62.0;
        for (i, (action, key)) in rows.iter().enumerate() {
            let col = i / per_col;
            let row = i % per_col;
            let x = px + pad + col as f32 * (col_w + gap);
            let y = top + row as f32 * ROW_H;
            if row % 2 == 0 {
                draw_rectangle(x, y, col_w, ROW_H, ROW_BG);
            }
            draw_text_ex(action, x + 10.0, y + 17.0, text_params(15, TEXT_LABEL));
            let kw = measure_game_text(key, 15);
            draw_text_ex(key, x + col_w - kw - 10.0, y + 17.0, text_params(15, WHITE));
        }

        let note = "Esc always pauses too. In a browser, the first click into a game goes fullscreen so keys like Ctrl+W stay in the game.";
        let note_size = if measure_game_text(note, 12) > pw - pad * 2.0 { 10 } else { 12 };
        draw_text_ex(note, px + pad, py + ph - 52.0, text_params(note_size, TEXT_DIM));

        let bw = 150.0;
        let bh = 32.0;
        let by = py + ph - 42.0;
        let edit = button("CHANGE KEYS", 15, px + pw / 2.0 - bw - 8.0, by, bw, bh);
        let close = outline_button("CLOSE", 15, px + pw / 2.0 + 8.0, by, bw, bh);

        if edit {
            Some(ControlsAction::EditKeys)
        } else if close || is_key_pressed(KeyCode::Escape) {
            Some(ControlsAction::Close)
        } else {
            None
        }
    }
}

/// "LeftShift" -> "Left Shift", "Key1" -> "1", "Mouse Left" unchanged.
fn pretty_key(name: &str) -> String {
    let name = match name.strip_prefix("Key") {
        Some(rest) if rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() => rest,
        _ => name,
    };
    let mut out = String::with_capacity(name.len() + 4);
    let mut prev_lower = false;
    for c in name.chars() {
        if c.is_ascii_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_ascii_lowercase();
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::pretty_key;

    #[test]
    fn key_names_read_naturally() {
        assert_eq!(pretty_key("LeftShift"), "Left Shift");
        assert_eq!(pretty_key("Key1"), "1");
        assert_eq!(pretty_key("C"), "C");
        assert_eq!(pretty_key("Mouse Left"), "Mouse Left");
        assert_eq!(pretty_key("F11"), "F11");
    }
}
