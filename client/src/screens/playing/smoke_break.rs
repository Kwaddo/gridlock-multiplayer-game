/*
    smoke break on the client, the server decides everything and this just shows it
    what the server told us (the round, coins, the golden coin, who's gold, hunter dots, scores),
    the keys (y / n to vote, q to go golden), hunters only ever holding the deagle,
    and the world bits: coins, the golden coin and the gold glow on boxes
    the hud and the vote board are in rendering/smoke_break_hud.rs
*/

use macroquad::prelude::*;

use crate::net::protocols::{SmokeRound, SmokeScore, MODE_SMOKE_BREAK, SMOKE_PHASE_VOTE, SMOKE_SIDE_HUNTER};
use crate::types::world::WorldId;
use crate::weapons::stats::WeaponId;
use crate::world::render::cardboard_box::{BOX_D, BOX_H, BOX_W};

use super::PlayingState;

pub const GOLD: Color = Color::new(1.0, 0.78, 0.2, 1.0);
/// the key that goes golden, g is already punch and q makes sense, i think its an ultimate key in some game. 
const GOLDEN_KEY: KeyCode = KeyCode::Q;

/// everything the server told us about the round
#[derive(Default)]
pub struct SmokeView {
    pub round: Option<SmokeRound>,
    /// counts down between server updates so the clock ticks smoothly
    pub timer: f32,
    pub coins: Vec<Vec3>,
    pub scores: Vec<SmokeScore>,
    /// hiders only: where the golden coin is and seconds left
    pub gold_coin: Option<(Vec3, f32)>,
    /// player ids gold right now
    pub golden: Vec<u32>,
    /// hunters only: (where, gold right now), false is a "last seen here" dot
    pub dots: Vec<(Vec3, bool)>,
    /// y or n pressed, goes out with the next send
    pub pending_vote: Option<bool>,
    /// q pressed, goes out with the next send
    pub pending_golden: bool,
    /// what we voted and in which round, so the vote board shows it
    pub voted: Option<(u16, bool)>,
}

fn v3(p: (f32, f32, f32)) -> Vec3 {
    vec3(p.0, p.1, p.2)
}

impl PlayingState {
    pub fn smoke_on(&self) -> bool {
        self.match_mode == Some(MODE_SMOKE_BREAK) && self.current_world == WorldId::Maze && self.smoke.round.is_some()
    }

    pub(super) fn smoke_hunting(&self) -> bool {
        self.smoke.round.is_some_and(|r| r.side == SMOKE_SIDE_HUNTER)
    }

    pub fn take_smoke_round(&mut self, round: SmokeRound) {
        self.smoke.timer = round.phase_timer;
        self.smoke.round = Some(round);
    }

    pub fn take_smoke_coins(&mut self, coins: Vec<(f32, f32, f32)>) {
        self.smoke.coins = coins.into_iter().map(v3).collect();
    }

    pub fn take_smoke_scores(&mut self, scores: Vec<SmokeScore>) {
        self.smoke.scores = scores;
    }

    pub fn take_smoke_gold_coin(&mut self, coin: Option<((f32, f32, f32), f32)>) {
        self.smoke.gold_coin = coin.map(|(at, left)| (v3(at), left));
    }

    pub fn take_smoke_golden(&mut self, ids: Vec<u32>) {
        self.smoke.golden = ids;
    }

    pub fn take_smoke_dots(&mut self, dots: Vec<((f32, f32, f32), bool)>) {
        self.smoke.dots = dots.into_iter().map(|(at, gold)| (v3(at), gold)).collect();
    }

    /// someone grabbed a coin, it's gone from our list straight away (the sound comes when we pick one)
    pub fn smoke_coin_taken(&mut self, at: (f32, f32, f32)) {
        let at = v3(at);
        self.smoke.coins.retain(|c| c.distance(at) > 0.05);
    }

    pub(super) fn update_smoke_break(&mut self, delta: f32) {
        if self.match_mode != Some(MODE_SMOKE_BREAK) || self.current_world != WorldId::Maze {
            // left the match, forget all of it so the next one starts clean
            if self.smoke.round.is_some() {
                self.smoke = SmokeView::default();
            }
            return;
        }
        let Some(round) = self.smoke.round else { return };
        self.smoke.timer = (self.smoke.timer - delta).max(0.0);
        if let Some((_, left)) = self.smoke.gold_coin.as_mut() {
            *left = (*left - delta).max(0.0);
        }
        if round.side == SMOKE_SIDE_HUNTER {
            self.hold_hunter_gun();
        }
        if self.chat.is_open() || self.lobby_console_open {
            return;
        }
        if round.phase == SMOKE_PHASE_VOTE {
            let pick = if is_key_pressed(KeyCode::Y) {
                Some(true)
            } else if is_key_pressed(KeyCode::N) {
                Some(false)
            } else {
                None
            };
            if let Some(stay) = pick {
                self.smoke.pending_vote = Some(stay);
                self.smoke.voted = Some((round.round, stay));
            }
        } else if round.golden_ready && is_key_pressed(GOLDEN_KEY) {
            self.smoke.pending_golden = true;
        }
    }

    /// the deagle and nothing else, no knife either
    fn hold_hunter_gun(&mut self) {
        let w = &mut self.weapons;
        for slot in 0..w.weapons.len() {
            if w.weapons[slot].id != WeaponId::Deagle {
                w.replace_weapon(slot, WeaponId::Deagle);
            }
        }
        w.active_slot = 0;
        w.knife_active = false;
        w.set_locked_slot(Some(0));
    }

    /// the minimap in smoke break: only hunters get player dots (a hider never learns who the
    /// other humans are from it), plus the golden coin for hiders or the golden dots for hunters
    pub(super) fn smoke_minimap(&self) -> Option<(Vec<u32>, Vec<(Vec3, Color)>)> {
        if !self.smoke_on() {
            return None;
        }
        let hunters = self.smoke.scores.iter().filter(|s| s.side == SMOKE_SIDE_HUNTER).map(|s| s.id).collect();
        let mut markers = Vec::new();
        if let Some((at, _)) = self.smoke.gold_coin {
            markers.push((at, GOLD));
        }
        for (at, gold) in &self.smoke.dots {
            let color = if *gold { GOLD } else { Color::new(GOLD.r, GOLD.g, GOLD.b, 0.4) };
            markers.push((*at, color));
        }
        Some((hunters, markers))
    }

    /// same side, so their tag shows (a hider sees the other hiders' boxes up close)
    pub(super) fn smoke_mates(&self, me: u32, other: u32) -> bool {
        if !self.smoke_on() {
            return false;
        }
        let side = |id: u32| self.smoke.scores.iter().find(|s| s.id == id).map(|s| s.side);
        matches!((side(me), side(other)), (Some(a), Some(b)) if a == b)
    }

    /// the box this player is in is gold right now
    pub(super) fn smoke_gold(&self, player_id: u32) -> bool {
        self.smoke_on() && self.smoke.golden.contains(&player_id)
    }

    /// spinning coins and the bigger golden one, only hiders ever have any to draw
    pub(super) fn draw_smoke_coins(&self) {
        if !self.smoke_on() {
            return;
        }
        let t = get_time() as f32;
        let coins = self.smoke.coins.iter().map(|c| (*c, 0.17, 1.0));
        let golden = self.smoke.gold_coin.map(|(at, _)| (at, 0.3, 1.0 + (t * 6.0).sin() * 0.25));
        for (i, (at, radius, glow)) in coins.chain(golden).enumerate() {
            let bob = (t * 2.0 + i as f32).sin() * 0.05;
            let color = Color::new((GOLD.r * glow).min(1.0), (GOLD.g * glow).min(1.0), GOLD.b * glow, 1.0);
            push_turned(at + vec3(0.0, 0.45 + bob, 0.0), t * 2.5 + i as f32, std::f32::consts::FRAC_PI_2);
            draw_cylinder(vec3(0.0, -0.03, 0.0), radius, radius, 0.06, None, color);
            pop_turned();
        }
    }

    /// a see-through gold shell around a gold box, anyone who can see it sees it
    pub(super) fn draw_gold_glow(&self, at: Vec3, yaw: f32) {
        let pulse = 0.35 + (get_time() as f32 * 8.0).sin().abs() * 0.25;
        push_turned(at, -yaw, 0.0);
        let size = vec3(BOX_W, BOX_H, BOX_D) * 1.08;
        draw_cube(vec3(0.0, BOX_H / 2.0, 0.0), size, None, Color::new(GOLD.r, GOLD.g, GOLD.b, pulse));
        pop_turned();
    }
}

/// draw at 'at', turned 'yaw' around y and 'tip' around x, until pop_turned
fn push_turned(at: Vec3, yaw: f32, tip: f32) {
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.push_model_matrix(Mat4::from_translation(at) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(tip));
}

fn pop_turned() {
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.pop_model_matrix();
}