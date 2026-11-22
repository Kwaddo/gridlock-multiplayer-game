/*
    smoke break on the client, the server decides everything and this just shows it
    what the server told us (the round, coins, the golden coin, who's gold, hunter dots, scores),
    the q key (taunt, it was called golden), hunters only ever holding the deagle,
    and the world bits: coins, the golden coin and the gold glow on boxes
    the hud and the vote board are in rendering/smoke_break_hud.rs
*/

use macroquad::prelude::*;

use crate::audio::catalog::{COIN_PICKUP, TAUNT, TAUNT_LENGTH};
use crate::net::networking::ChatEvent;
use crate::net::protocols::{SmokeRound, SmokeScore, MODE_SMOKE_BREAK, SMOKE_PHASE_BLIND, SMOKE_SIDE_HIDER, SMOKE_SIDE_HUNTER};
use crate::types::world::WorldId;
use crate::weapons::stats::WeaponId;
use crate::world::render::cardboard_box::{BOX_D, BOX_H, BOX_W};

use super::{KillFeedEntry, PlayingState};

pub const GOLD: Color = Color::new(1.0, 0.78, 0.2, 1.0);
/// how long "FOUND by NAME" stays up before your hunter hud takes over
pub const FOUND_CARD_TIME: f32 = 3.0;
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
    /// q pressed, goes out with the next send
    pub pending_golden: bool,
    /// we just got found: by who and what it cost, the card counts down found_timer
    pub found: Option<(String, i16)>,
    pub found_timer: f32,
    /// our own player id, so our own taunt plays from where we are
    pub my_id: u32,
    /// (player id, seconds till their taunt plays again) for everyone gold right now
    pub taunts: Vec<(u32, f32)>,
    /// found hiders only: seconds the gun stays locked, counts down between updates
    pub gun_lock: f32,
}

fn v3(p: (f32, f32, f32)) -> Vec3 {
    vec3(p.0, p.1, p.2)
}

impl PlayingState {
    pub fn smoke_mode(&self) -> bool {
        self.match_mode == Some(MODE_SMOKE_BREAK) && self.current_world == WorldId::Maze
    }

    pub fn smoke_on(&self) -> bool {
        self.smoke_mode() && self.smoke.round.is_some()
    }

    pub(super) fn smoke_hunting(&self) -> bool {
        self.smoke.round.is_some_and(|r| r.side == SMOKE_SIDE_HUNTER)
    }

    /// a found hider whose gun is locked right now, no firing and no gun on screen
    pub(super) fn smoke_gun_locked(&self) -> bool {
        self.smoke_on() && self.smoke.round.is_some_and(|r| r.found) && self.smoke.gun_lock > 0.0
    }

    /// a hider not found yet, no gun and no shooting
    pub(super) fn smoke_hiding(&self) -> bool {
        self.smoke_on() && self.smoke.round.is_some_and(|r| r.side == SMOKE_SIDE_HIDER)
    }

    /// a hunter still blind at the start of the round
    pub(super) fn smoke_blind(&self) -> bool {
        self.smoke_on() && self.smoke.round.is_some_and(|r| r.side == SMOKE_SIDE_HUNTER && r.phase == SMOKE_PHASE_BLIND)
    }

    /// no firing at all right now: hiding, blind, or a found hider's gun locked
    pub(super) fn smoke_cant_fire(&self) -> bool {
        self.smoke_hiding() || self.smoke_blind() || self.smoke_gun_locked()
    }

    pub fn take_smoke_round(&mut self, round: SmokeRound) {
        // a new round: last round's found card, taunt repeats and an unsent taunt go,
        // the rest (coins, golden coin, dots, golden ids) the server resends itself
        if self.smoke.round.is_some_and(|r| r.round != round.round) {
            self.smoke.found = None;
            self.smoke.found_timer = 0.0;
            self.smoke.taunts.clear();
            self.smoke.pending_golden = false;
        }
        self.smoke.timer = round.phase_timer;
        self.smoke.gun_lock = round.gun_lock;
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

    pub fn take_smoke_golden(&mut self, ids: Vec<u32>, player_id: u32) {
        self.smoke.golden = ids;
        self.smoke.my_id = player_id;
    }

    pub fn take_smoke_dots(&mut self, dots: Vec<((f32, f32, f32), bool)>) {
        self.smoke.dots = dots.into_iter().map(|(at, gold)| (v3(at), gold)).collect();
    }

    /// a name from the player list, for the feed and the boards
    pub(super) fn smoke_name(&self, id: u32) -> String {
        self.connected_players.iter().find(|p| p.0 == id).map_or("?".to_string(), |p| p.1.clone())
    }

    /// box shots go in the kill feed: finds (gold tag when red handed) and decoy misses with what they cost
    pub fn take_smoke_feed(&mut self, feed: Vec<(u32, u32, bool, i16)>, player_id: u32) {
        for (shooter, target, red_handed, points) in feed {
            let shooter_name = self.smoke_name(shooter);
            let tag = if target == 0 {
                (points < 0).then(|| (points.to_string(), Color::new(1.0, 0.35, 0.3, 1.0)))
            } else if red_handed {
                Some(("RED-HANDED".to_string(), GOLD))
            } else {
                None
            };
            let victim_name = if target == 0 { "a decoy".to_string() } else { self.smoke_name(target) };
            if target == player_id {
                self.smoke.found = Some((shooter_name.clone(), points));
                self.smoke.found_timer = FOUND_CARD_TIME;
            }
            self.kill_feed.push(KillFeedEntry {
                killer_id: shooter,
                victim_id: target,
                killer_name: shooter_name,
                victim_name,
                killer_team: 0,
                victim_team: 0,
                weapon: "Deagle".to_string(),
                timer: 5.0,
                tag,
            });
        }
    }

    /// someone grabbed a coin (or the golden one): gone from our list, and everyone hears it from where it was
    pub fn smoke_coin_taken(&mut self, at: (f32, f32, f32)) {
        let at = v3(at);
        self.smoke.coins.retain(|c| c.distance(at) > 0.05);
        self.play_heard_everywhere(COIN_PICKUP, at);
    }

    /// everyone gold is taunting, the sound repeats from their box till they stop being gold
    fn play_taunts(&mut self, delta: f32) {
        let golden = self.smoke.golden.clone();
        self.smoke.taunts.retain(|(id, _)| golden.contains(id));
        for id in golden {
            if !self.smoke.taunts.iter().any(|(t, _)| *t == id) {
                // just went gold, play straight away
                self.smoke.taunts.push((id, 0.0));
            }
        }
        let mut due = Vec::new();
        for (id, left) in &mut self.smoke.taunts {
            *left -= delta;
            if *left <= 0.0 {
                *left += TAUNT_LENGTH;
                due.push(*id);
            }
        }
        for id in due {
            let at = if id == self.smoke.my_id {
                Some(self.position)
            } else {
                self.other_players.iter().find(|p| p.id == id).map(|p| vec3(p.render_x, p.render_y, p.render_z))
            };
            if let Some(at) = at {
                self.play_heard_everywhere(TAUNT, at);
            }
        }
    }

    pub(super) fn update_smoke_break(&mut self, delta: f32) {
        if self.match_mode != Some(MODE_SMOKE_BREAK) || self.current_world != WorldId::Maze {
            // left the match, forget all of it so the next one starts clean
            if self.smoke.round.is_some() {
                self.smoke = SmokeView::default();
            }
            return;
        }
        // "X joined" and the like go in chat as notices, the bottom left stays clear
        for (text, _) in self.announcements.drain(..) {
            self.chat.push(ChatEvent::Notice(text));
        }
        let Some(round) = self.smoke.round else { return };
        self.smoke.timer = (self.smoke.timer - delta).max(0.0);
        self.smoke.found_timer = (self.smoke.found_timer - delta).max(0.0);
        self.smoke.gun_lock = (self.smoke.gun_lock - delta).max(0.0);
        if let Some((_, left)) = self.smoke.gold_coin.as_mut() {
            *left = (*left - delta).max(0.0);
        }
        self.play_taunts(delta);
        if round.side == SMOKE_SIDE_HUNTER {
            self.hold_hunter_gun();
        }
        if self.chat.is_open() || self.lobby_console_open || self.is_dead {
            return;
        }
        // the server says when it's allowed (not used yet, not blind, not the last 30 s, not after the round)
        if round.golden_ready && is_key_pressed(GOLDEN_KEY) {
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
        // a hider gets no player dots at all, not even the hunters
        let hunters = if self.smoke_hiding() {
            Vec::new()
        } else {
            self.smoke.scores.iter().filter(|s| s.side == SMOKE_SIDE_HUNTER).map(|s| s.id).collect()
        };
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
