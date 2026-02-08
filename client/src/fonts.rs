use macroquad::prelude::*;
use std::sync::OnceLock;

static GAME_FONT: OnceLock<Font> = OnceLock::new();

// Call once at startup to load the font.
pub async fn init_font() {
    if let Ok(font) = load_ttf_font("assets/fonts/Pixel Game.otf").await {
        let _ = GAME_FONT.set(font);
    }
}

// pub async fn init_font() {
//     if let Ok(font) = load_ttf_font("assets/fonts/TrajanPro-Bold.otf").await {
//         let _ = GAME_FONT.set(font);
//     }
// }


// load font or panic if it fails (since the game won't look right without it)
pub fn game_font() -> Option<&'static Font> {
    GAME_FONT.get()
}

// build text params with font so we can easily change font settings in one place
pub fn text_params(font_size: u16, color: Color) -> TextParams<'static> {
    TextParams {
        font: game_font(),
        font_size,
        font_scale: 1.0,
        font_scale_aspect: 1.0,
        color,
        ..Default::default()
    }
}

// measure text width using the game font.
pub fn measure_game_text(text: &str, font_size: u16) -> f32 {
    measure_text(text, game_font(), font_size, 1.0).width
}
