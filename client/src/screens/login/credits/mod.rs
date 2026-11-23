//! The main menu's credits: CREDITS.md (compiled in, see doc.rs) as a
//! scrolling list. Clicking a creator, licence or model opens its page; the
//! website button opens the same credits on the site.

mod doc;

use macroquad::prelude::*;

use crate::ui::fonts::measure_game_text;
use crate::ui::text_batch;

use super::widgets::{button, outline_button};
use super::{in_rect, ACCENT, INPUT_BG, PANEL_BG, PANEL_BORDER, ROW_BG, ROW_HOVER, TEXT_DIM, TEXT_LABEL};
use doc::{Kind, Row};

/// The credits on the website. In the browser, the page next to the game's
/// (so a local or LAN build opens its own copy).
#[cfg(target_arch = "wasm32")]
const SITE_CREDITS: &str = "credits.html";
#[cfg(not(target_arch = "wasm32"))]
const SITE_CREDITS: &str = "https://gridlock.kwaddo.dev/credits.html";

const LINK: Color = Color::new(0.55, 0.72, 1.0, 1.0);
const NOTE: &str = "Click a name or title to open its page.   Esc: back";

const HEADING_SIZE: u16 = 18;
const HEADING_LINE: f32 = 24.0;
const HEADING_GAP: f32 = 18.0;
const TEXT_SIZE: u16 = 14;
const TEXT_LINE: f32 = 19.0;
const ENTRY_PAD: f32 = 4.0;
/// Below this list width an entry's label goes above its text instead of beside it.
const TWO_COLUMNS_MIN: f32 = 440.0;
const SCROLLBAR_W: f32 = 6.0;

/// One row laid out at the current width: its lines and where it sits in the list.
struct Item {
    row: usize,
    y: f32,
    h: f32,
    /// (x offset, y offset of the baseline, text, is the label)
    lines: Vec<(f32, f32, String, bool)>,
}

struct Layout {
    /// list width and DPI scale bits it was built for
    key: (u32, u32),
    items: Vec<Item>,
    height: f32,
}

/// The open credits screen: rows, scroll position and the layout cache.
pub(super) struct CreditsView {
    rows: Vec<Row>,
    scroll: f32,
    /// Dragging the scrollbar: (mouse y, scroll) where the drag began.
    drag: Option<(f32, f32)>,
    layout: Option<Layout>,
}

impl CreditsView {
    pub(super) fn new() -> Self {
        Self { rows: doc::parse(doc::CREDITS_MD), scroll: 0.0, drag: None, layout: None }
    }

    /// Draws the screen over the menu backdrop and handles its input; true once
    /// the player leaves it.
    pub(super) fn draw(&mut self) -> bool {
        let sw = screen_width();
        let sh = screen_height();
        let pw = 720.0_f32.min(sw - 32.0).max(240.0);
        let ph = 720.0_f32.min(sh - 64.0).max(220.0);
        let px = (sw - pw) / 2.0;
        let py = ((sh - ph) / 2.0).max(8.0);
        let pad = 24.0_f32.min(pw * 0.05).max(12.0);

        text_batch::rect(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.55));
        text_batch::rect(px, py, pw, ph, PANEL_BG);
        text_batch::rect_lines(px, py, pw, ph, 1.0, PANEL_BORDER);
        let title = "CREDITS";
        let tw = measure_game_text(title, 24);
        text_batch::text(title, px + (pw - tw) / 2.0, py + 38.0, 24, WHITE);

        // the list, between the title and the footer
        let lx = px + pad;
        let ly = py + 56.0;
        let lw = pw - pad * 2.0 - SCROLLBAR_W - 8.0;
        let lh = (ph - 56.0 - 72.0).max(40.0);
        self.relayout(lw);
        let content_h = self.layout.as_ref().map_or(0.0, |l| l.height);
        let max_scroll = (content_h - lh).max(0.0);

        self.scroll_input(px, py, pw, ph, lx, ly, lw, lh, content_h);
        self.scroll = self.scroll.clamp(0.0, max_scroll);

        let clicked = self.draw_list(lx, ly, lw, lh);
        if let Some(link) = clicked {
            platform::open_url(&link);
        }
        draw_scrollbar(lx + lw + 8.0, ly, lh, self.scroll, content_h);

        // footer: a hint, the website and back
        let note_size = if measure_game_text(NOTE, 12) > pw - pad * 2.0 { 10 } else { 12 };
        text_batch::text(NOTE, px + pad, py + ph - 52.0, note_size, TEXT_DIM);
        let bw = 170.0_f32.min((pw - pad * 2.0 - 16.0) / 2.0);
        let bh = 32.0;
        let by = py + ph - 42.0;
        let cx = px + pw / 2.0;
        if button("OPEN WEBSITE", 15, cx - bw - 8.0, by, bw, bh) {
            platform::open_url(SITE_CREDITS);
        }
        let back = outline_button("BACK", 15, cx + 8.0, by, bw, bh);
        back || is_key_pressed(KeyCode::Escape)
    }

    /// Wheel, keys and the scrollbar.
    #[allow(clippy::too_many_arguments)]
    fn scroll_input(&mut self, px: f32, py: f32, pw: f32, ph: f32, lx: f32, ly: f32, lw: f32, lh: f32, content_h: f32) {
        let mouse = mouse_position();
        if in_rect(mouse, px, py, pw, ph) {
            self.scroll -= wheel_pixels(mouse_wheel().1);
        }
        if is_key_pressed(KeyCode::Down) {
            self.scroll += TEXT_LINE * 2.0;
        }
        if is_key_pressed(KeyCode::Up) {
            self.scroll -= TEXT_LINE * 2.0;
        }
        if is_key_pressed(KeyCode::PageDown) || is_key_pressed(KeyCode::Space) {
            self.scroll += lh * 0.9;
        }
        if is_key_pressed(KeyCode::PageUp) {
            self.scroll -= lh * 0.9;
        }
        if is_key_pressed(KeyCode::Home) {
            self.scroll = 0.0;
        }
        if is_key_pressed(KeyCode::End) {
            self.scroll = content_h;
        }

        // the scrollbar: grab anywhere on its track (a little wider than drawn)
        let track_x = lx + lw + 2.0;
        if is_mouse_button_pressed(MouseButton::Left) && in_rect(mouse, track_x, ly, SCROLLBAR_W + 12.0, lh) {
            if content_h > lh {
                // jump so the thumb centres on the click, then drag from there
                let frac = ((mouse.1 - ly) / lh).clamp(0.0, 1.0);
                self.scroll = frac * content_h - lh / 2.0;
            }
            self.drag = Some((mouse.1, self.scroll));
        }
        if let Some((start_y, start_scroll)) = self.drag {
            if is_mouse_button_down(MouseButton::Left) {
                self.scroll = start_scroll + (mouse.1 - start_y) * content_h / lh;
            } else {
                self.drag = None;
            }
        }
    }

    /// Draws the visible lines; returns the link clicked this frame, if any.
    fn draw_list(&self, lx: f32, ly: f32, lw: f32, lh: f32) -> Option<String> {
        let layout = self.layout.as_ref()?;
        let mouse = mouse_position();
        let mouse_in_list = in_rect(mouse, lx, ly, lw, lh) && self.drag.is_none();
        let mut clicked = None;
        let mut entry_index = 0usize;
        for item in &layout.items {
            let row = &self.rows[item.row];
            if row.kind == Kind::Entry {
                entry_index += 1;
            }
            let top = ly + item.y - self.scroll;
            if top + item.h < ly || top > ly + lh {
                continue;
            }
            let hovered = row.link.is_some() && mouse_in_list && mouse.1 >= top && mouse.1 < top + item.h;

            // row background, cut to the list
            let bg = if hovered {
                Some(ROW_HOVER)
            } else if row.kind == Kind::Entry && entry_index % 2 == 1 {
                Some(ROW_BG)
            } else {
                None
            };
            if let Some(bg) = bg {
                let (y0, y1) = (top.max(ly), (top + item.h).min(ly + lh));
                if y1 > y0 {
                    text_batch::rect(lx, y0, lw, y1 - y0, bg);
                }
            }
            if row.kind == Kind::Heading {
                let y = top + item.h - 4.0;
                if y >= ly && y <= ly + lh {
                    text_batch::rect(lx, y, lw, 1.0, PANEL_BORDER);
                }
            }

            // lines only where they fit whole, so nothing spills past the list
            let size = if row.kind == Kind::Heading { HEADING_SIZE } else { TEXT_SIZE };
            for (x, base, line, is_label) in &item.lines {
                let y = top + base;
                if y - size as f32 * 0.9 < ly || y + 3.0 > ly + lh {
                    continue;
                }
                let color = match (row.kind, is_label) {
                    (_, true) => TEXT_LABEL,
                    (Kind::Heading, _) => ACCENT,
                    _ if hovered => WHITE,
                    _ if row.link.is_some() => LINK,
                    _ => TEXT_LABEL,
                };
                let w = text_batch::text(line, lx + x, y, size, color);
                if hovered && !is_label {
                    text_batch::rect(lx + x, y + 2.0, w, 1.0, color);
                }
            }
            if hovered && is_mouse_button_pressed(MouseButton::Left) {
                clicked = row.link.clone();
            }
        }
        clicked
    }

    /// Wraps every row to `width`; reuses the last layout when nothing changed.
    fn relayout(&mut self, width: f32) {
        let key = (width.to_bits(), screen_dpi_scale().to_bits());
        if self.layout.as_ref().is_some_and(|l| l.key == key) {
            return;
        }
        let two_columns = width >= TWO_COLUMNS_MIN;
        let label_w = if two_columns { (width * 0.34).min(220.0) } else { width };
        let mut items = Vec::with_capacity(self.rows.len());
        let mut y = 0.0;
        for (i, row) in self.rows.iter().enumerate() {
            let mut lines = Vec::new();
            let h = match row.kind {
                Kind::Title => continue, // the screen's own title stands in for it
                Kind::Heading => {
                    if !items.is_empty() {
                        y += HEADING_GAP;
                    }
                    let wrapped = wrap(&row.text, width, HEADING_SIZE);
                    for (n, line) in wrapped.iter().enumerate() {
                        lines.push((0.0, HEADING_LINE * (n as f32 + 1.0) - 6.0, line.clone(), false));
                    }
                    HEADING_LINE * wrapped.len() as f32 + 6.0
                }
                Kind::Text => {
                    let wrapped = wrap(&row.text, width, TEXT_SIZE);
                    for (n, line) in wrapped.iter().enumerate() {
                        lines.push((0.0, TEXT_LINE * (n as f32 + 1.0) - 4.0, line.clone(), false));
                    }
                    TEXT_LINE * wrapped.len() as f32 + 8.0
                }
                Kind::Entry => {
                    let inset = 8.0;
                    let label = wrap(&row.label, label_w - inset * 2.0, TEXT_SIZE);
                    let (text_x, text_top) = if two_columns {
                        (label_w + inset, 0.0)
                    } else {
                        (inset * 2.0, TEXT_LINE * label.len() as f32)
                    };
                    let text = wrap(&row.text, width - text_x - inset, TEXT_SIZE);
                    for (n, line) in label.iter().enumerate() {
                        lines.push((inset, ENTRY_PAD + TEXT_LINE * (n as f32 + 1.0) - 4.0, line.clone(), true));
                    }
                    for (n, line) in text.iter().enumerate() {
                        let base = ENTRY_PAD + text_top + TEXT_LINE * (n as f32 + 1.0) - 4.0;
                        lines.push((text_x, base, line.clone(), false));
                    }
                    let tall = if two_columns {
                        TEXT_LINE * label.len().max(text.len()) as f32
                    } else {
                        text_top + TEXT_LINE * text.len() as f32
                    };
                    tall + ENTRY_PAD * 2.0
                }
            };
            items.push(Item { row: i, y, h, lines });
            y += h;
        }
        self.layout = Some(Layout { key, items, height: y + 8.0 });
    }
}

/// Mouse wheel steps to pixels: the browser reports pixels already, native
/// platforms report lines (fractions of one on trackpads).
fn wheel_pixels(wheel: f32) -> f32 {
    if cfg!(target_arch = "wasm32") {
        wheel
    } else {
        wheel * 40.0
    }
}

fn draw_scrollbar(x: f32, y: f32, h: f32, scroll: f32, content_h: f32) {
    if content_h <= h {
        return;
    }
    text_batch::rect(x, y, SCROLLBAR_W, h, INPUT_BG);
    let thumb_h = (h * h / content_h).max(24.0).min(h);
    let max_scroll = content_h - h;
    let thumb_y = y + (h - thumb_h) * (scroll / max_scroll).clamp(0.0, 1.0);
    text_batch::rect(x, thumb_y, SCROLLBAR_W, thumb_h, TEXT_DIM);
}

/// Greedy word wrap to `width` at `size`. A word wider than a line gets a line of its own.
fn wrap(text: &str, width: f32, size: u16) -> Vec<String> {
    wrap_with(text, width, |s| measure_game_text(s, size))
}

fn wrap_with(text: &str, width: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.is_empty() {
            line.push_str(word);
            continue;
        }
        let candidate = format!("{line} {word}");
        if measure(&candidate) <= width {
            line = candidate;
        } else {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::wrap_with;

    // 10 px per character
    fn measure(s: &str) -> f32 {
        s.len() as f32 * 10.0
    }

    #[test]
    fn wraps_on_words() {
        assert_eq!(wrap_with("aa bb cc dd", 50.0, measure), ["aa bb", "cc dd"]);
        assert_eq!(wrap_with("one", 500.0, measure), ["one"]);
        // too long for any line: alone on its own, never dropped
        assert_eq!(wrap_with("a verylongword b", 50.0, measure), ["a", "verylongword", "b"]);
        assert_eq!(wrap_with("", 50.0, measure), [""]);
    }
}
