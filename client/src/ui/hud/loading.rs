use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;

/// A quiet note at the bottom of the screen while the rest of the game's
/// assets load in the background after joining (app::asset_loader::AssetQueue).
pub fn draw_background_loading(loaded: usize, total: usize) {
    if total == 0 || loaded >= total {
        return;
    }
    let text = format!("Loading assets {loaded} / {total}");
    let w = measure_game_text(&text, 13);
    draw_text_ex(
        &text,
        (screen_width() - w) / 2.0,
        screen_height() - 14.0,
        text_params(13, Color::from_rgba(150, 150, 150, 200)),
    );
}
