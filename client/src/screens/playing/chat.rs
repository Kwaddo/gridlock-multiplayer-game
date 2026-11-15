//! In-game text chat: the feed of player lines and server notices (joins,
//! leaves, match events) drawn bottom-left, and the line being typed.

use std::cell::RefCell;
use std::collections::VecDeque;

use macroquad::prelude::*;

pub use crate::net::networking::ChatEvent;
use crate::ui::fonts::{game_font, measure_game_text, measure_game_text_uncached, text_params};

/// Longest line a player can type (the server cuts anything longer).
pub const CHAT_MAX_CHARS: usize = 120;
/// Lines kept for scrollback while the chat is open.
const HISTORY: usize = 50;
/// Seconds a line stays on screen while the chat is closed (it fades out over the last second).
const LINE_LIFETIME: f32 = 10.0;
const ROWS_CLOSED: usize = 6;
const ROWS_OPEN: usize = 12;
const FONT: u16 = 15;
const ROW_H: f32 = 19.0;
const MAX_W: f32 = 460.0;
const LEFT: f32 = 20.0;
/// Bottom of the input line, above the health and stamina bars.
const BOTTOM_GAP: f32 = 140.0;

const NAME_COLOR: Color = Color::new(1.0, 0.62, 0.25, 1.0);
const TEXT_COLOR: Color = Color::new(0.94, 0.94, 0.94, 1.0);
const NOTICE_COLOR: Color = Color::new(0.98, 0.86, 0.45, 1.0);

struct FeedLine {
    /// The sender's `"name: "` label, `None` for a server notice.
    name_label: Option<String>,
    text: String,
    age: f32,
    /// The wrapped rows, kept until the width or the font scale changes.
    layout: RefCell<Option<LineLayout>>,
}

/// One feed line laid out for a given width.
struct LineLayout {
    /// (max_w bits, DPI scale bits, game font loaded): what the rows depend on
    key: (u32, u32, bool),
    name_w: f32,
    rows: Vec<String>,
}

impl FeedLine {
    /// The name width and wrapped rows for `max_w`, wrapped again only if
    /// the width or the font scale changed since the last frame.
    fn layout(&self, max_w: f32) -> std::cell::Ref<'_, LineLayout> {
        let key = (max_w.to_bits(), screen_dpi_scale().to_bits(), game_font().is_some());
        let stale = self.layout.borrow().as_ref().map_or(true, |l| l.key != key);
        if stale {
            let name_w = self.name_label.as_deref().map_or(0.0, |l| measure_game_text(l, FONT));
            let rows = wrap(&self.text, max_w - name_w, max_w);
            *self.layout.borrow_mut() = Some(LineLayout { key, name_w, rows });
        }
        std::cell::Ref::map(self.layout.borrow(), |l| l.as_ref().expect("layout just set"))
    }
}

#[derive(Default)]
pub struct Chat {
    lines: VecDeque<FeedLine>,
    open: bool,
    draft: String,
    /// `"> {draft}_"` as drawn, reused so typing does not allocate every frame.
    input_line: RefCell<String>,
    outgoing: Vec<String>,
    backspace_repeat: f32,
    /// Lines pushed since the connection began (drives the chat sound).
    received: u32,
}

impl Chat {
    pub fn push(&mut self, event: ChatEvent) {
        let (name_label, text) = match event {
            ChatEvent::Line { name, text } => (Some(format!("{}: ", name)), text),
            ChatEvent::Notice(text) => (None, text),
        };
        if self.lines.len() == HISTORY {
            self.lines.pop_front();
        }
        self.lines.push_back(FeedLine { name_label, text, age: 0.0, layout: RefCell::new(None) });
        self.received = self.received.wrapping_add(1);
    }

    /// How many lines have arrived; grows by one per line.
    pub fn received(&self) -> u32 {
        self.received
    }

    /// Forget everything (a new connection starts with an empty chat).
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self) {
        self.open = true;
        self.draft.clear();
        // Keys pressed while playing (and the key that opened the chat) also
        // queue up as typed characters; they are not part of the message.
        while get_char_pressed().is_some() {}
    }

    pub fn close(&mut self) {
        self.open = false;
        self.draft.clear();
    }

    /// Lines typed since the last call, to send to the server.
    pub fn take_outgoing(&mut self) -> Vec<String> {
        std::mem::take(&mut self.outgoing)
    }

    pub fn tick(&mut self, dt: f32) {
        for line in &mut self.lines {
            line.age += dt;
        }
    }

    /// Typing, while the chat is open: Enter sends, Escape cancels.
    pub fn update_input(&mut self) {
        if is_key_pressed(KeyCode::Escape) {
            self.close();
            return;
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            let text = self.draft.trim().to_string();
            if !text.is_empty() {
                self.outgoing.push(text);
            }
            self.close();
            return;
        }
        if is_key_pressed(KeyCode::Backspace) {
            self.draft.pop();
            self.backspace_repeat = 0.45;
        } else if is_key_down(KeyCode::Backspace) {
            self.backspace_repeat -= get_frame_time();
            if self.backspace_repeat <= 0.0 {
                self.draft.pop();
                self.backspace_repeat = 0.035;
            }
        }
        while let Some(c) = get_char_pressed() {
            // Backspace and Enter arrive here as characters too (macOS sends
            // Backspace as DEL, 0x7f); the key checks above handled them.
            if !c.is_control() && self.draft.chars().count() < CHAT_MAX_CHARS {
                self.draft.push(c);
            }
        }
    }

    pub fn draw(&self) {
        let bottom = screen_height() - BOTTOM_GAP;
        let max_w = MAX_W.min(screen_width() - LEFT * 2.0);

        let mut y = bottom;
        if self.open {
            draw_rectangle(
                LEFT - 6.0,
                bottom - ROW_H * (ROWS_OPEN as f32 + 1.0) - 6.0,
                max_w + 12.0,
                ROW_H * (ROWS_OPEN as f32 + 1.0) + 12.0,
                Color::new(0.0, 0.0, 0.0, 0.35),
            );
            draw_rectangle(LEFT - 6.0, bottom - ROW_H + 2.0, max_w + 12.0, ROW_H + 4.0, Color::new(0.0, 0.0, 0.0, 0.55));
            let caret = if (get_time() * 2.0) as i64 % 2 == 0 { "_" } else { " " };
            let mut input = self.input_line.borrow_mut();
            input.clear();
            input.push_str("> ");
            input.push_str(&self.draft);
            input.push_str(caret);
            // keep the end of a long draft in view
            let shown = tail_fitting(&input, max_w);
            draw_text_ex(shown, LEFT, bottom - 4.0, text_params(FONT, WHITE));
            y -= ROW_H;
        }

        let max_rows = if self.open { ROWS_OPEN } else { ROWS_CLOSED };
        let mut rows_left = max_rows;
        for line in self.lines.iter().rev() {
            if rows_left == 0 {
                break;
            }
            let alpha = if self.open {
                1.0
            } else {
                (LINE_LIFETIME - line.age).clamp(0.0, 1.0)
            };
            if alpha <= 0.0 {
                break; // older lines are older still
            }

            let layout = line.layout(max_w);
            let name_w = layout.name_w;
            let rows = &layout.rows;
            // draw this entry bottom-up so wrapped rows read top to bottom
            let take = rows.len().min(rows_left);
            let first_shown = rows.len() - take;
            for (i, row) in rows.iter().enumerate().skip(first_shown).rev() {
                let mut x = LEFT;
                if i == 0 {
                    if let Some(label) = &line.name_label {
                        draw_shadowed(label, x, y - 4.0, with_alpha(NAME_COLOR, alpha));
                        x += name_w;
                    }
                }
                let color = if line.name_label.is_some() { TEXT_COLOR } else { NOTICE_COLOR };
                draw_shadowed(row, x, y - 4.0, with_alpha(color, alpha));
                y -= ROW_H;
            }
            rows_left -= take;
        }
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * a)
}

fn draw_shadowed(text: &str, x: f32, y: f32, color: Color) {
    draw_text_ex(text, x + 1.0, y + 1.0, text_params(FONT, Color::new(0.0, 0.0, 0.0, 0.7 * color.a)));
    draw_text_ex(text, x, y, text_params(FONT, color));
}

/// The longest end of `text` that fits in `max_w` (at least its last character).
fn tail_fitting(text: &str, max_w: f32) -> &str {
    if measure_game_text(text, FONT) <= max_w {
        return text;
    }
    // One right-to-left pass adding up character widths finds the cut ...
    let mut start = text.char_indices().next_back().map_or(0, |(i, _)| i);
    let mut w = 0.0;
    let mut buf = [0u8; 4];
    for (i, c) in text.char_indices().rev() {
        w += measure_game_text(c.encode_utf8(&mut buf), FONT);
        if w > max_w {
            break;
        }
        start = i;
    }
    // ... then the whole-suffix measure settles it, as a sum of single
    // characters can round differently at the boundary.
    while let Some((prev, _)) = text[..start].char_indices().next_back() {
        if measure_game_text(&text[prev..], FONT) > max_w {
            break;
        }
        start = prev;
    }
    while measure_game_text(&text[start..], FONT) > max_w {
        let next = start + text[start..].chars().next().map_or(0, char::len_utf8);
        if next >= text.len() {
            break;
        }
        start = next;
    }
    &text[start..]
}

/// Greedy word wrap: the first row is `first_w` wide, the rest `rest_w`.
/// Words wider than a whole row are broken between characters.
fn wrap(text: &str, first_w: f32, rest_w: f32) -> Vec<String> {
    let mut rows = Vec::new();
    let mut row = String::new();
    let width = |rows: &Vec<String>| if rows.is_empty() { first_w } else { rest_w };
    for word in text.split_whitespace() {
        let candidate = if row.is_empty() { word.to_string() } else { format!("{} {}", row, word) };
        if measure_game_text_uncached(&candidate, FONT) <= width(&rows) {
            row = candidate;
            continue;
        }
        if !row.is_empty() {
            rows.push(std::mem::take(&mut row));
        }
        for c in word.chars() {
            row.push(c);
            if measure_game_text_uncached(&row, FONT) > width(&rows) && row.chars().count() > 1 {
                row.pop();
                rows.push(std::mem::take(&mut row));
                row.push(c);
            }
        }
    }
    if !row.is_empty() || rows.is_empty() {
        rows.push(row);
    }
    rows
}
