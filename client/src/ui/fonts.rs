use macroquad::prelude::*;
use std::path::Path;
use std::sync::OnceLock;

static GAME_FONT: OnceLock<Font> = OnceLock::new();

// Call once at startup to load the font.
pub async fn init_font() {
    let path = resolve_font_path("assets/fonts/Pixel Game.otf");
    if let Ok(font) = load_ttf_font(&path).await {
        let _ = GAME_FONT.set(font);
    }
}

fn resolve_font_path(rel: &str) -> String {
    if Path::new(rel).exists() {
        return rel.to_string();
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(rel);
            if candidate.exists() {
                return candidate.to_string_lossy().into_owned();
            }
        }
    }
    let client_path = format!("client/{}", rel);
    if Path::new(&client_path).exists() {
        return client_path;
    }
    rel.to_string()
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

// Oversample factor: render glyphs at Nx the requested size, then scale
// down when drawing. This gives the font atlas much more pixel data to
// work with and eliminates the blurriness macroquad gets at small sizes.
const FONT_OVERSAMPLE: u16 = 2;

// build text params with font so we can easily change font settings in one place
pub fn text_params(font_size: u16, color: Color) -> TextParams<'static> {
    let atlas_size = font_size * FONT_OVERSAMPLE;
    let scale = 1.0 / FONT_OVERSAMPLE as f32;
    TextParams {
        font: game_font(),
        font_size: atlas_size,
        font_scale: scale,
        font_scale_aspect: 1.0,
        color,
        ..Default::default()
    }
}

// measure text width using the game font.
pub fn measure_game_text(text: &str, font_size: u16) -> f32 {
    let atlas_size = font_size * FONT_OVERSAMPLE;
    let scale = 1.0 / FONT_OVERSAMPLE as f32;
    measure_text(text, game_font(), atlas_size, scale).width
}
