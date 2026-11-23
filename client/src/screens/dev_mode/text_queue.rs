// Deferred text for the dev overlay.
//
// macroquad starts a new draw call whenever the bound texture changes, and
// text (font atlas) interleaved with shapes (no texture) splits every widget
// into several batches. Widgets queue their text here and the overlay flushes
// it once after the shapes of the same layer, so each layer is ~2 batches.
// Strings live in one reused arena, so queued text does not allocate.

use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;
use std::cell::RefCell;
use std::fmt::{self, Write};

struct QueuedText {
    start: usize,
    end: usize,
    x: f32,
    y: f32,
    size: u16,
    color: Color,
}

#[derive(Default)]
struct TextQueue {
    arena: String,
    items: Vec<QueuedText>,
}

thread_local! {
    static QUEUE: RefCell<TextQueue> = RefCell::new(TextQueue::default());
    // lowest point any row or text reached since the last take_content_bottom()
    static CONTENT_BOTTOM: std::cell::Cell<f32> = const { std::cell::Cell::new(f32::NEG_INFINITY) };
}

/// Note that tab content reaches down to `bottom` (screen y).
pub(super) fn note_content_bottom(bottom: f32) {
    CONTENT_BOTTOM.with(|b| b.set(b.get().max(bottom)));
}

/// The lowest point content reached since the last call, if any; resets it.
pub(super) fn take_content_bottom() -> Option<f32> {
    let bottom = CONTENT_BOTTOM.with(|b| b.replace(f32::NEG_INFINITY));
    bottom.is_finite().then_some(bottom)
}

pub(super) fn queue_text(text: &str, x: f32, y: f32, size: u16, color: Color) {
    QUEUE.with(|q| {
        let q = &mut *q.borrow_mut();
        let start = q.arena.len();
        q.arena.push_str(text);
        let end = q.arena.len();
        q.items.push(QueuedText { start, end, x, y, size, color });
    });
    // y is the baseline; descenders reach a little below it
    note_content_bottom(y + size as f32 * 0.35);
}

pub(super) fn queue_fmt(args: fmt::Arguments, x: f32, y: f32, size: u16, color: Color) {
    queue_fmt_with(args, size, color, |_| (x, y));
}

// Formats `args` into the arena, then lets `place` pick the position from
// the measured width (for centred labels).
pub(super) fn queue_fmt_with(
    args: fmt::Arguments,
    size: u16,
    color: Color,
    place: impl FnOnce(f32) -> (f32, f32),
) {
    let y = QUEUE.with(|q| {
        let q = &mut *q.borrow_mut();
        let start = q.arena.len();
        let _ = q.arena.write_fmt(args);
        let end = q.arena.len();
        let (x, y) = place(measure_game_text(&q.arena[start..end], size));
        q.items.push(QueuedText { start, end, x, y, size, color });
        y
    });
    note_content_bottom(y + size as f32 * 0.35);
}

pub(super) fn flush_text() {
    QUEUE.with(|q| {
        let q = &mut *q.borrow_mut();
        for t in &q.items {
            draw_text_ex(&q.arena[t.start..t.end], t.x, t.y, text_params(t.size, t.color));
        }
        q.items.clear();
        q.arena.clear();
    });
}

pub fn queue_fmt_public(args: fmt::Arguments, x: f32, y: f32, size: u16, color: Color) {
    queue_fmt(args, x, y, size, color);
}
