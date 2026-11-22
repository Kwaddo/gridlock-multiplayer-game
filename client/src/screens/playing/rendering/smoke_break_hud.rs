/*
    the smoke break hud
    top centre: a pill with the round, the clock and coins left, the phase line under it, the golden coin banner for hiders
    bottom centre: your own pill, side, points and the golden key (hiders) or the multiplier (hunters)
    plus the hunter blind screen, the found card, the gold border while you're gold, the vote board and the tab board
*/

use macroquad::prelude::*;

use crate::net::protocols::{
    SmokeRound, SmokeScore, SMOKE_PHASE_BLIND, SMOKE_PHASE_COINS, SMOKE_PHASE_RELEASED, SMOKE_PHASE_RESULTS,
    SMOKE_SIDE_HIDER, SMOKE_SIDE_HUNTER,
};
use crate::ui::fonts::{measure_game_text, text_params};

use super::super::smoke_break::{FOUND_CARD_TIME, GOLD};
use super::super::PlayingState;

const HUNTER_COLOR: Color = Color::new(1.0, 0.45, 0.35, 1.0);
const HIDER_COLOR: Color = Color::new(0.45, 0.85, 1.0, 1.0);
const PANEL: Color = Color::new(0.03, 0.04, 0.08, 0.82);
const EDGE: Color = Color::new(0.30, 0.32, 0.45, 0.9);
const DIM: Color = Color::new(0.65, 0.67, 0.75, 1.0);
const BAD: Color = Color::new(1.0, 0.35, 0.3, 1.0);
const GOOD: Color = Color::new(0.5, 0.95, 0.55, 1.0);
const ON_GOLD: Color = Color::new(0.12, 0.08, 0.02, 1.0);
/// the clock goes red for this long at the end, same time golden locks
const LAST_SECONDS: f32 = 30.0;
/// how long the golden coin stays out, the banner fades in and out from it
const GOLD_COIN_LIFE: f32 = 30.0;

/// how wide the bottom pill is, the hider chat stays out of its way
pub(crate) const BOTTOM_PILL_W: f32 = 410.0;
const BOTTOM_PILL_H: f32 = 40.0;
const BOTTOM_PILL_GAP: f32 = 22.0;

impl PlayingState {
    pub(crate) fn draw_smoke_hud(&self, player_id: u32) {
        let Some(round) = self.smoke.round else { return };
        let hunting = self.smoke_hunting();

        // blind, the whole screen is black till we're let out, nothing else on top
        if hunting && round.phase == SMOKE_PHASE_BLIND {
            self.draw_blind_screen();
            return;
        }

        let gold = self.smoke_gold(player_id);
        if gold {
            draw_gold_edges();
        }
        self.draw_found_flash();

        self.draw_top_pill(round, hunting);
        let me = self.smoke.scores.iter().find(|s| s.id == player_id).copied();
        draw_bottom_pill(round, me, hunting, gold, self.smoke.gun_lock);

        if round.phase == SMOKE_PHASE_RESULTS {
            self.draw_smoke_results(round, player_id);
        }
        self.draw_found_card();
    }

    /// round on the left, the clock big in the middle, coins on the right, the phase under it
    fn draw_top_pill(&self, round: SmokeRound, hunting: bool) {
        let (cell, mid) = (110.0, 120.0);
        let (w, h) = (cell * 2.0 + mid, 44.0);
        let (x, y) = ((screen_width() - w) / 2.0, 10.0);
        draw_rectangle(x, y, w, h, PANEL);
        draw_rectangle_lines(x, y, w, h, 1.0, EDGE);
        for dx in [cell, cell + mid] {
            draw_line(x + dx, y + 9.0, x + dx, y + h - 9.0, 1.0, EDGE);
        }

        centered_in(&format!("ROUND {}/{}", round.round, round.max_rounds), x, cell, y + 28.0, 16, DIM);
        let secs = self.smoke.timer.ceil() as u32;
        let late = round.phase == SMOKE_PHASE_COINS && self.smoke.timer <= LAST_SECONDS;
        let clock_color = if late {
            // red and pulsing for the last 30 s
            let pulse = 0.6 + 0.4 * (get_time() as f32 * 5.0).sin().abs();
            Color::new(BAD.r, BAD.g, BAD.b, pulse)
        } else {
            WHITE
        };
        centered_in(&format!("{}:{:02}", secs / 60, secs % 60), x + cell, mid, y + 33.0, 28, clock_color);
        if round.phase == SMOKE_PHASE_COINS {
            centered_in(&format!("{} coins", round.coins_left), x + cell + mid, cell, y + 28.0, 16, GOLD);
        }

        let (what, color) = match (round.phase, hunting) {
            (SMOKE_PHASE_BLIND, _) => ("HIDE  |  blend in with the boxes", HIDER_COLOR),
            (SMOKE_PHASE_RELEASED, false) => ("HUNTERS ARE OUT  |  coins are out soon", HIDER_COLOR),
            (SMOKE_PHASE_RELEASED, true) | (SMOKE_PHASE_COINS, true) => ("FIND THE HIDERS", HUNTER_COLOR),
            (SMOKE_PHASE_COINS, false) => ("COLLECT COINS AND STAY HIDDEN", GOLD),
            _ => ("", WHITE),
        };
        if !what.is_empty() {
            centered_shadowed(what, y + h + 22.0, 16, color);
        }

        // the golden coin, hiders only, slides in and fades out with its 30 s
        if let Some((_, left)) = self.smoke.gold_coin.filter(|_| !hunting) {
            let show = ((GOLD_COIN_LIFE - left) * 4.0).min(left * 4.0).clamp(0.0, 1.0);
            let text = format!("GOLDEN COIN  {}s  |  check your minimap", left.ceil() as u32);
            let bw = measure_game_text(&text, 16) + 40.0;
            let by = y + h + 34.0 - (1.0 - show) * 16.0;
            let bx = (screen_width() - bw) / 2.0;
            draw_rectangle(bx, by, bw, 28.0, Color::new(0.30, 0.22, 0.04, 0.88 * show));
            draw_rectangle_lines(bx, by, bw, 28.0, 1.5, Color::new(GOLD.r, GOLD.g, GOLD.b, show));
            centered_in(&text, bx, bw, by + 19.0, 16, Color::new(GOLD.r, GOLD.g, GOLD.b, show));
        }
    }

    /// hunters at the start: the countdown and the rules, the numbers match the server's constants
    fn draw_blind_screen(&self) {
        let (sw, sh) = (screen_width(), screen_height());
        draw_rectangle(0.0, 0.0, sw, sh, BLACK);
        let top = sh * 0.26;
        centered("YOU ARE HUNTING", top, 34, HUNTER_COLOR);
        centered(&format!("{}", self.smoke.timer.ceil() as u32), top + 110.0, 96, WHITE);

        let (w, h) = (440.0, 112.0);
        let (x, y) = ((sw - w) / 2.0, top + 150.0);
        draw_rectangle(x, y, w, h, PANEL);
        draw_rectangle_lines(x, y, w, h, 1.0, EDGE);
        let rows: [(&str, Color, &str); 3] = [
            ("+50", GOOD, "find a hider   (multiplier +0.2)"),
            ("-10", BAD, "shoot a decoy   (multiplier -0.1)"),
            ("", GOLD, "gold dot = a hider taunted"),
        ];
        for (i, (big, color, text)) in rows.iter().enumerate() {
            let ry = y + 34.0 + i as f32 * 30.0;
            if big.is_empty() {
                draw_circle(x + 40.0, ry - 6.0, 6.0, *color);
            } else {
                draw_text_ex(big, x + 24.0, ry, text_params(20, *color));
            }
            draw_text_ex(text, x + 80.0, ry, text_params(16, WHITE));
        }
    }

    /// a short red flash the moment you get found
    fn draw_found_flash(&self) {
        let since = FOUND_CARD_TIME - self.smoke.found_timer;
        if self.smoke.found.is_some() && since < 0.5 {
            let alpha = (1.0 - since / 0.5) * 0.45;
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.9, 0.1, 0.1, alpha));
        }
    }

    /// "FOUND by NAME" for 3 s, then your hunter hud takes over
    fn draw_found_card(&self) {
        let Some((by, lost)) = &self.smoke.found else { return };
        if self.smoke.found_timer <= 0.0 {
            return;
        }
        let alpha = (self.smoke.found_timer / 0.4).min(1.0);
        let (w, h) = (440.0, 78.0);
        let (x, y) = ((screen_width() - w) / 2.0, screen_height() * 0.30);
        draw_rectangle(x, y, w, h, Color::new(PANEL.r, PANEL.g, PANEL.b, PANEL.a * alpha));
        draw_rectangle_lines(x, y, w, h, 1.5, Color::new(BAD.r, BAD.g, BAD.b, alpha));
        centered_in(&format!("FOUND by {by}"), x, w, y + 34.0, 24, Color::new(BAD.r, BAD.g, BAD.b, alpha));
        let sub = format!("-{} pts   |   you hunt now", lost.unsigned_abs());
        centered_in(&sub, x, w, y + 60.0, 16, Color::new(1.0, 1.0, 1.0, alpha));
    }

    /// the board between rounds: who won, everyone's points, and when the next one starts
    fn draw_smoke_results(&self, round: SmokeRound, player_id: u32) {
        let sw = screen_width();
        let sh = screen_height();
        let rows = self.smoke.scores.len().min(12);
        let (w, h) = (460.0, 150.0 + rows as f32 * 26.0);
        let (x, y) = ((sw - w) / 2.0, (sh - h) / 2.0);
        draw_rectangle(x, y, w, h, Color::new(0.03, 0.04, 0.08, 0.92));
        draw_rectangle_lines(x, y, w, h, 1.0, EDGE);
        centered_in(&format!("ROUND {} OVER", round.round), x, w, y + 34.0, 22, WHITE);
        let (won, won_color) = match round.winner {
            SMOKE_SIDE_HIDER => ("HIDERS WIN", HIDER_COLOR),
            SMOKE_SIDE_HUNTER => ("HUNTERS WIN", HUNTER_COLOR),
            _ => ("", WHITE),
        };
        centered_in(won, x, w, y + 58.0, 18, won_color);

        let head = y + 86.0;
        draw_text_ex("this round", x + w - 150.0, head, text_params(12, DIM));
        draw_text_ex("total", x + w - 60.0, head, text_params(12, DIM));
        for (i, s) in self.smoke.scores.iter().take(rows).enumerate() {
            let ry = head + 24.0 + i as f32 * 26.0;
            let color = if s.id == player_id { GOLD } else { WHITE };
            let (side, side_color) = side_label(s.side);
            draw_text_ex(&format!("{}.", i + 1), x + 20.0, ry, text_params(15, DIM));
            draw_text_ex(&self.smoke_name(s.id), x + 52.0, ry, text_params(15, color));
            draw_text_ex(side, x + 230.0, ry, text_params(13, side_color));
            let (gain, gain_color) = match s.round_points {
                p if p > 0 => (format!("+{p}"), GOOD),
                p if p < 0 => (p.to_string(), BAD),
                _ => ("0".to_string(), DIM),
            };
            draw_text_ex(&gain, x + w - 150.0, ry, text_params(15, gain_color));
            let pts = s.points.to_string();
            draw_text_ex(&pts, x + w - 20.0 - measure_game_text(&pts, 15), ry, text_params(15, color));
        }

        let secs = self.smoke.timer.ceil() as u32;
        let last = round.round >= round.max_rounds as u16;
        let foot = if last { format!("match over in {secs}s") } else { format!("next round in {secs}s") };
        centered_in(&foot, x, w, y + h - 24.0, 18, GOLD);
    }

    /// tab in smoke break: name, side, points, best first (the server sends them sorted)
    pub(crate) fn draw_smoke_tab(&self, player_id: u32) {
        let Some(round) = self.smoke.round else { return };
        let rows = self.smoke.scores.len().min(16);
        let (w, row_h) = (460.0, 28.0);
        let h = 78.0 + rows as f32 * row_h;
        let (x, y) = ((screen_width() - w) / 2.0, 120.0);
        draw_rectangle(x, y, w, h, Color::new(0.03, 0.04, 0.05, 0.88));
        draw_rectangle_lines(x, y, w, h, 2.0, Color::new(0.8, 0.8, 0.8, 0.25));
        draw_text_ex(&format!("Smoke Break  |  Round {}", round.round), x + 16.0, y + 30.0, text_params(22, GOLD));

        let head = y + 56.0;
        draw_text_ex("NAME", x + 16.0, head, text_params(13, DIM));
        draw_text_ex("SIDE", x + 260.0, head, text_params(13, DIM));
        draw_text_ex("PTS", x + w - 16.0 - measure_game_text("PTS", 13), head, text_params(13, DIM));
        for (i, s) in self.smoke.scores.iter().take(rows).enumerate() {
            let ry = head + 8.0 + i as f32 * row_h;
            let me = s.id == player_id;
            if me {
                draw_rectangle(x + 6.0, ry, w - 12.0, row_h - 2.0, Color::new(1.0, 1.0, 1.0, 0.08));
            }
            let color = if me { GOLD } else { WHITE };
            let (side, side_color) = side_label(s.side);
            draw_text_ex(&self.smoke_name(s.id), x + 16.0, ry + 19.0, text_params(16, color));
            draw_text_ex(side, x + 260.0, ry + 19.0, text_params(14, side_color));
            let pts = s.points.to_string();
            draw_text_ex(&pts, x + w - 16.0 - measure_game_text(&pts, 16), ry + 19.0, text_params(16, color));
        }
    }
}

/// side | points | golden key or multiplier, the whole pill goes gold while you're gold
fn draw_bottom_pill(round: SmokeRound, me: Option<SmokeScore>, hunting: bool, gold: bool, gun_lock: f32) {
    let (side_w, pts_w) = (120.0, 140.0);
    let key_w = BOTTOM_PILL_W - side_w - pts_w;
    let (w, h) = (BOTTOM_PILL_W, BOTTOM_PILL_H);
    let (x, y) = ((screen_width() - w) / 2.0, screen_height() - h - BOTTOM_PILL_GAP);
    let (fill, edge) = if gold { (Color::new(GOLD.r, GOLD.g, GOLD.b, 0.9), GOLD) } else { (PANEL, EDGE) };
    draw_rectangle(x, y, w, h, fill);
    draw_rectangle_lines(x, y, w, h, 1.0, edge);
    let line = if gold { ON_GOLD } else { EDGE };
    for dx in [side_w, side_w + pts_w] {
        draw_line(x + dx, y + 8.0, x + dx, y + h - 8.0, 1.0, line);
    }

    let (side, side_color) = if round.found {
        ("FOUND", BAD)
    } else if hunting {
        ("HUNTER", HUNTER_COLOR)
    } else {
        ("HIDER", HIDER_COLOR)
    };
    centered_in(side, x, side_w, y + 26.0, 17, if gold { ON_GOLD } else { side_color });
    let points = me.map_or(0, |s| s.points);
    centered_in(&format!("{points} pts"), x + side_w, pts_w, y + 28.0, 22, if gold { ON_GOLD } else { WHITE });

    let (key, key_color) = if round.found {
        // found hiders can't gain points so the multiplier means nothing, the gun is what matters
        if gun_lock > 0.0 {
            (format!("LOCKED {}s", gun_lock.ceil() as u32), BAD)
        } else {
            ("READY".to_string(), GOOD)
        }
    } else if hunting {
        let tenths = me.map_or(10, |s| s.mult_tenths);
        let color = match tenths {
            t if t < 10 => BAD,
            t if t > 10 => GOOD,
            _ => WHITE,
        };
        (format!("x{}.{}", tenths / 10, tenths % 10), color)
    } else if gold {
        ("TAUNTING".to_string(), ON_GOLD)
    } else if round.golden_ready {
        ("[Q] TAUNT".to_string(), GOLD)
    } else {
        // used, or locked in the last 30 s
        ("[Q] TAUNT".to_string(), Color::new(0.45, 0.45, 0.5, 1.0))
    };
    centered_in(&key, x + side_w + pts_w, key_w, y + 26.0, 17, key_color);
}

/// a thin gold border pulsing round the screen while you're gold
fn draw_gold_edges() {
    let (sw, sh) = (screen_width(), screen_height());
    let pulse = 0.35 + 0.35 * (get_time() as f32 * 6.0).sin().abs();
    let c = Color::new(GOLD.r, GOLD.g, GOLD.b, pulse);
    let t = 6.0;
    draw_rectangle(0.0, 0.0, sw, t, c);
    draw_rectangle(0.0, sh - t, sw, t, c);
    draw_rectangle(0.0, t, t, sh - t * 2.0, c);
    draw_rectangle(sw - t, t, t, sh - t * 2.0, c);
}

fn side_label(side: u8) -> (&'static str, Color) {
    if side == SMOKE_SIDE_HUNTER { ("hunter", HUNTER_COLOR) } else { ("hider", HIDER_COLOR) }
}

fn centered(text: &str, y: f32, size: u16, color: Color) {
    centered_in(text, 0.0, screen_width(), y, size, color);
}

/// centred inside the strip from `x` that is `w` wide
fn centered_in(text: &str, x: f32, w: f32, y: f32, size: u16, color: Color) {
    let tw = measure_game_text(text, size);
    draw_text_ex(text, x + (w - tw) / 2.0, y, text_params(size, color));
}

/// centred with a drop shadow, for text over the world
fn centered_shadowed(text: &str, y: f32, size: u16, color: Color) {
    let x = (screen_width() - measure_game_text(text, size)) / 2.0;
    draw_text_ex(text, x + 1.0, y + 1.0, text_params(size, Color::new(0.0, 0.0, 0.0, 0.7 * color.a)));
    draw_text_ex(text, x, y, text_params(size, color));
}
