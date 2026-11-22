/*
    the smoke break hud. 
    Rounds, the clock up top, your side, points, and golden ability at the bottom left,
    the golden coin countdown for hiders, the black blind screen for hunters, and the vote board
*/

use macroquad::prelude::*;

use crate::net::protocols::{
    SmokeRound, SMOKE_PHASE_BLIND, SMOKE_PHASE_COINS, SMOKE_PHASE_RELEASED, SMOKE_PHASE_VOTE, SMOKE_SIDE_HUNTER,
};
use crate::ui::fonts::{measure_game_text, text_params};

use super::super::smoke_break::GOLD;
use super::super::PlayingState;

const HUNTER_COLOR: Color = Color::new(1.0, 0.45, 0.35, 1.0);
const HIDER_COLOR: Color = Color::new(0.45, 0.85, 1.0, 1.0);

impl PlayingState {
    pub(crate) fn draw_smoke_hud(&self, player_id: u32) {
        let Some(round) = self.smoke.round else { return };
        let sh = screen_height();
        let hunting = self.smoke_hunting();

        // blind, the whole screen is black till we're let out
        if hunting && round.phase == SMOKE_PHASE_BLIND {
            draw_rectangle(0.0, 0.0, screen_width(), sh, BLACK);
            centered("YOU ARE HUNTING", sh * 0.40, 40, HUNTER_COLOR);
            centered(&format!("released in {}", self.smoke.timer.ceil() as u32), sh * 0.40 + 46.0, 22, WHITE);
            centered("find the humans hiding among the boxes, every decoy you shoot costs points", sh * 0.40 + 84.0, 15, LIGHTGRAY);
        }

        // top centre: round, what's happening, the clock
        let (what, color) = match (round.phase, hunting) {
            (SMOKE_PHASE_BLIND, false) => ("HIDE  -  blend in with the boxes", HIDER_COLOR),
            (SMOKE_PHASE_BLIND, true) => ("WAIT", HUNTER_COLOR),
            (SMOKE_PHASE_RELEASED, false) => ("HUNTERS ARE OUT  -  coins soon", HIDER_COLOR),
            (SMOKE_PHASE_RELEASED, true) | (SMOKE_PHASE_COINS, true) => ("FIND THE HUMANS", HUNTER_COLOR),
            (SMOKE_PHASE_COINS, false) => ("COLLECT COINS  -  or just survive", GOLD),
            _ => ("ROUND OVER", WHITE),
        };
        let secs = self.smoke.timer.ceil() as u32;
        centered(&format!("ROUND {}   {}:{:02}", round.round, secs / 60, secs % 60), 30.0, 22, WHITE);
        centered(what, 54.0, 16, color);
        if round.phase == SMOKE_PHASE_COINS {
            centered(&format!("{} coins left", round.coins_left), 74.0, 14, GOLD);
        }
        if let Some((_, left)) = self.smoke.gold_coin {
            centered(&format!("GOLDEN COIN  -  {}s  (on your minimap)", left.ceil() as u32), 94.0, 15, GOLD);
        }

        // bottom left above the health bar: side, points, golden
        let mine = self.smoke.scores.iter().find(|s| s.id == player_id).map_or(0, |s| s.points);
        let (side, side_color) = if hunting { ("HUNTER", HUNTER_COLOR) } else { ("HIDER", HIDER_COLOR) };
        draw_text_ex(side, 16.0, sh - 140.0, text_params(18, side_color));
        draw_text_ex(&format!("{} pts", mine), 16.0, sh - 118.0, text_params(16, WHITE));
        if self.smoke_gold(player_id) {
            draw_text_ex("YOU'RE GOLD  -  hunters can see you", 16.0, sh - 98.0, text_params(15, GOLD));
        } else if round.golden_ready {
            draw_text_ex("[Q]  GO GOLDEN  (5 pts/s, hunters see you)", 16.0, sh - 98.0, text_params(14, GOLD));
        }

        if round.phase == SMOKE_PHASE_VOTE {
            self.draw_smoke_vote(round, player_id);
        }
    }

    fn draw_smoke_vote(&self, round: SmokeRound, player_id: u32) {
        let sw = screen_width();
        let sh = screen_height();
        let rows = self.smoke.scores.len().min(12);
        let (w, h) = (420.0, 150.0 + rows as f32 * 26.0);
        let (x, y) = ((sw - w) / 2.0, (sh - h) / 2.0);
        draw_rectangle(x, y, w, h, Color::new(0.03, 0.04, 0.08, 0.92));
        draw_rectangle_lines(x, y, w, h, 1.0, Color::new(0.3, 0.32, 0.45, 1.0));
        centered(&format!("ROUND {} OVER", round.round), y + 34.0, 22, WHITE);

        for (i, s) in self.smoke.scores.iter().take(rows).enumerate() {
            let ry = y + 70.0 + i as f32 * 26.0;
            let name = self.connected_players.iter().find(|p| p.0 == s.id).map_or("?".to_string(), |p| p.1.clone());
            let color = if s.id == player_id { GOLD } else { WHITE };
            let (side, side_color) = if s.side == SMOKE_SIDE_HUNTER { ("hunter", HUNTER_COLOR) } else { ("hider", HIDER_COLOR) };
            draw_text_ex(&format!("{}.", i + 1), x + 20.0, ry, text_params(15, LIGHTGRAY));
            draw_text_ex(&name, x + 52.0, ry, text_params(15, color));
            draw_text_ex(side, x + 250.0, ry, text_params(13, side_color));
            let pts = s.points.to_string();
            draw_text_ex(&pts, x + w - 20.0 - measure_game_text(&pts, 15), ry, text_params(15, color));
        }

        let foot = y + h - 46.0;
        let mine = self.smoke.voted.filter(|(r, _)| *r == round.round).map(|(_, stay)| stay);
        let prompt = match mine {
            Some(true) => "you're staying   [N] leave instead",
            Some(false) => "you're leaving   [Y] stay instead",
            None => "[Y] STAY        [N] LEAVE",
        };
        centered(prompt, foot, 18, GOLD);
        let secs = self.smoke.timer.ceil() as u32;
        let count = format!("{} of {} staying   -   {}s   (no vote = leave)", round.votes_stay, round.voters, secs);
        centered(&count, foot + 26.0, 13, LIGHTGRAY);
    }
}

fn centered(text: &str, y: f32, size: u16, color: Color) {
    let w = measure_game_text(text, size);
    draw_text_ex(text, (screen_width() - w) / 2.0, y, text_params(size, color));
}