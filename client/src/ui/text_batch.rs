//! Deferred text for UI panels that interleave shapes and text.
//!
//! macroquad ends its current draw call every time the bound texture changes,
//! so a widget list that goes rect, text, rect, text... costs one draw call per
//! switch. Inside `batched`, `text` only queues the string; shapes drawn
//! through `rect`/`rect_lines`/`line` (or announced with `guard`) go straight
//! to the batcher, and the queued text is drawn in one go at the end. Picture
//! order is kept: a shape whose bounds touch any queued text flushes the queue
//! first, so text never ends up on top of a shape that used to cover it.
//! Outside `batched`, `text` draws immediately.

use std::cell::RefCell;
use std::ops::Range;

use macroquad::prelude::*;

use crate::ui::fonts::{measure_game_text, text_params};

struct Queued {
    text: Range<usize>,
    x: f32,
    y: f32,
    font_size: u16,
    color: Color,
    // conservative glyph bounds: min x, min y, max x, max y
    bounds: [f32; 4],
}

struct Batch {
    depth: u32,
    arena: String,
    items: Vec<Queued>,
}

thread_local! {
    static BATCH: RefCell<Batch> =
        const { RefCell::new(Batch { depth: 0, arena: String::new(), items: Vec::new() }) };
}

/// Runs `draw` with text deferred, then draws the queued text.
pub fn batched<R>(draw: impl FnOnce() -> R) -> R {
    BATCH.with_borrow_mut(|b| b.depth += 1);
    let out = draw();
    let outer = BATCH.with_borrow_mut(|b| {
        b.depth -= 1;
        b.depth == 0
    });
    if outer {
        flush();
    }
    out
}

/// Draws every queued string now, in the order it was queued.
pub fn flush() {
    BATCH.with_borrow_mut(|b| {
        for q in &b.items {
            draw_text_ex(&b.arena[q.text.clone()], q.x, q.y, text_params(q.font_size, q.color));
        }
        b.items.clear();
        b.arena.clear();
    });
}

/// `draw_text_ex(s, x, y, text_params(font_size, color))`, queued when batching.
pub fn text(s: &str, x: f32, y: f32, font_size: u16, color: Color) {
    let batching = BATCH.with_borrow(|b| b.depth > 0);
    if !batching {
        draw_text_ex(s, x, y, text_params(font_size, color));
        return;
    }
    let width = measure_game_text(s, font_size);
    let fs = font_size as f32;
    // y is the baseline; allow for tall ascenders, descenders and side bearings
    let bounds = [x - fs * 0.5, y - fs * 1.5, x + width + fs * 0.5, y + fs * 0.6];
    BATCH.with_borrow_mut(|b| {
        let start = b.arena.len();
        b.arena.push_str(s);
        let end = b.arena.len();
        b.items.push(Queued { text: start..end, x, y, font_size, color, bounds });
    });
}

/// Call before drawing a shape that covers (x, y, w, h): flushes the queued
/// text first if the shape would overlap any of it.
pub fn guard(x: f32, y: f32, w: f32, h: f32) {
    let hit = BATCH.with_borrow(|b| {
        let (x0, x1) = (x.min(x + w), x.max(x + w));
        let (y0, y1) = (y.min(y + h), y.max(y + h));
        b.items.iter().any(|q| {
            let [bx0, by0, bx1, by1] = q.bounds;
            x0 <= bx1 && x1 >= bx0 && y0 <= by1 && y1 >= by0
        })
    });
    if hit {
        flush();
    }
}

/// `draw_rectangle`, after `guard`.
pub fn rect(x: f32, y: f32, w: f32, h: f32, color: Color) {
    guard(x, y, w, h);
    draw_rectangle(x, y, w, h, color);
}

/// `draw_rectangle_lines`, after `guard`.
pub fn rect_lines(x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Color) {
    guard(x - thickness, y - thickness, w + thickness * 2.0, h + thickness * 2.0);
    draw_rectangle_lines(x, y, w, h, thickness, color);
}

/// `draw_line`, after `guard`.
pub fn line(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
    guard(
        x1.min(x2) - thickness,
        y1.min(y2) - thickness,
        (x2 - x1).abs() + thickness * 2.0,
        (y2 - y1).abs() + thickness * 2.0,
    );
    draw_line(x1, y1, x2, y2, thickness, color);
}
