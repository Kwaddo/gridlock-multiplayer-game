use macroquad::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

static GAME_FONT: OnceLock<Font> = OnceLock::new();

// Call once at startup to load the font.
pub async fn init_font() {
    let path = resolve_font_path("assets/fonts/Pixel Game.otf");
    if let Ok(font) = load_ttf_font(&path).await {
        let _ = GAME_FONT.set(font);
    }
}

/// In the browser, paths are URLs relative to the page; nothing to probe.
#[cfg(target_arch = "wasm32")]
fn resolve_font_path(rel: &str) -> String {
    rel.to_string()
}

#[cfg(not(target_arch = "wasm32"))]
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

// Widths of recently measured strings, one bucket per font size. macroquad's
// measure_text takes three mutex locks and three hash lookups per character,
// and the HUD re-measures the same strings every frame. The result depends on
// the text, the size, the DPI scale and which font is loaded, so the whole
// memo is dropped when the DPI scale or font changes, and a bucket is cleared
// once it grows past MEASURE_MEMO_BUCKET_CAP (timers, chat drafts).
const MEASURE_MEMO_BUCKET_CAP: usize = 512;

struct MeasureMemo {
    dpi_scale: f32,
    has_font: bool,
    buckets: Vec<(u16, HashMap<String, f32>)>,
}

thread_local! {
    static MEASURE_MEMO: RefCell<MeasureMemo> = RefCell::new(MeasureMemo {
        dpi_scale: 0.0,
        has_font: false,
        buckets: Vec::new(),
    });
}

// measure text width using the game font.
pub fn measure_game_text(text: &str, font_size: u16) -> f32 {
    let font = game_font();
    let dpi_scale = screen_dpi_scale();
    MEASURE_MEMO.with(|memo| {
        let mut memo = memo.borrow_mut();
        if memo.dpi_scale != dpi_scale || memo.has_font != font.is_some() {
            memo.dpi_scale = dpi_scale;
            memo.has_font = font.is_some();
            memo.buckets.clear();
        }
        let idx = match memo.buckets.iter().position(|(size, _)| *size == font_size) {
            Some(i) => i,
            None => {
                memo.buckets.push((font_size, HashMap::new()));
                memo.buckets.len() - 1
            }
        };
        let bucket = &mut memo.buckets[idx].1;
        if let Some(&w) = bucket.get(text) {
            return w;
        }
        let w = measure_game_text_uncached(text, font_size);
        if bucket.len() >= MEASURE_MEMO_BUCKET_CAP {
            bucket.clear();
        }
        bucket.insert(text.to_owned(), w);
        w
    })
}

/// measure_game_text without the memo, for strings measured once (word-wrap
/// candidates) that would only push the strings drawn every frame out of it.
pub fn measure_game_text_uncached(text: &str, font_size: u16) -> f32 {
    let atlas_size = font_size * FONT_OVERSAMPLE;
    let scale = 1.0 / FONT_OVERSAMPLE as f32;
    measure_text(text, game_font(), atlas_size, scale).width
}
