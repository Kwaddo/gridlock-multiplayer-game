//! One bot's senses for one tick: hears the shots queued this tick and the
//! enemies sprinting or landing near it, and (every `SIGHT_CHECK_SECS`, not
//! every tick) looks at the enemies in its view cone, one line-of-sight
//! raycast each. Cheap filters (team, range, view cone) run before any ray.

use std::collections::HashMap;

use rand::Rng;

use super::model::{self, Alert, Boxed, Posture, SightInput};
use super::tuning::{self, *};
use super::{flat_dist, Awareness};
use crate::bot::ai::BotState;
use crate::bot::personality::difficulty_mods;
use crate::bot::sight::has_line_of_sight;
use crate::maze::MazeData;
use crate::net::{Outbox, PeerId};
use crate::protocols::{ServerMsg, BOT_ALERT_ENGAGED, BOT_ALERT_SUSPICIOUS};
use crate::player::{Player, WorldId};
use crate::server::game::MAZE_CELL_SIZE;
use crate::server::smoke_break::is_decoy;

/// A shot someone fired, heard by bots within `radius` of `pos`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Noise {
    pub shooter: PeerId,
    pub pos: (f32, f32, f32),
    pub radius: f32,
}

impl Noise {
    /// `shooter` (standing where `p` is) fired `weapon`.
    pub fn gunfire(shooter: PeerId, p: &Player, weapon: &str) -> Self {
        Noise { shooter, pos: p.position, radius: model::gunfire_radius(weapon) }
    }
}

/// Someone `bot` can sense as an enemy: alive, in its world, hostile, and not
/// a Smoke Break decoy (those are scenery the server walks around).
pub(crate) fn is_sensed(bot: &Player, peer: PeerId, p: &Player) -> bool {
    p.id != bot.id && p.is_alive && p.world == bot.world && bot.is_hostile_to(p) && !is_decoy(peer)
}

/// Every match bot's senses for one tick. Shooting-range bots (no
/// personality) have none; a dead or `frozen` bot forgets everything (a new
/// life, a new round). Smoke Break's decoys aren't bots here at all.
pub fn sense_bots(
    states: &mut HashMap<PeerId, BotState>,
    players: &HashMap<PeerId, Player>,
    maze: Option<&MazeData>,
    noises: &[Noise],
    dt: f32,
    frozen: impl Fn(&Player) -> bool,
) {
    for (addr, state) in states.iter_mut() {
        if state.personality_id.is_none() {
            continue;
        }
        let Some(bot) = players.get(addr) else { continue };
        if !bot.is_alive || bot.world != WorldId::Maze || frozen(bot) {
            state.awareness.reset();
            continue;
        }
        sense(bot, state, players, maze, noises, dt);
    }
}

/// Tell each spotted human what a bot's awareness of them just went up to
/// (`BotAlert`, issue #62). Only that player gets it; alerts about bots,
/// teammates or anyone gone are dropped.
pub fn send_alerts(states: &mut HashMap<PeerId, BotState>, players: &HashMap<PeerId, Player>, socket: &Outbox) {
    for (addr, state) in states.iter_mut() {
        let bot = players.get(addr);
        for (enemy_id, level) in state.awareness.take_alerts() {
            let level = match level {
                Alert::Engaged => BOT_ALERT_ENGAGED,
                Alert::Suspicious => BOT_ALERT_SUSPICIOUS,
                Alert::Unaware => continue,
            };
            let Some(bot) = bot else { continue };
            let spotted = |p: &Player| p.id == enemy_id && !p.is_bot && bot.is_hostile_to(p);
            if let Some((peer, _)) = players.iter().find(|(_, p)| spotted(p)) {
                let _ = socket.send_to(&ServerMsg::BotAlert { bot_id: bot.id, level }.encode(), *peer);
            }
        }
    }
}

/// Update `state.awareness` for one tick of `dt`. Every enemy whose alert level
/// went up gets an alert queued (`Awareness::take_alerts`), rate-limited.
pub fn sense(
    bot: &Player,
    state: &mut BotState,
    players: &HashMap<PeerId, Player>,
    maze: Option<&MazeData>,
    noises: &[Noise],
    dt: f32,
) {
    let perception = tuning::perception(&state.personality, state.difficulty);
    let hearing = tuning::hearing(&state.personality, state.difficulty);
    let fov = state.personality.base_fov_degrees.max(difficulty_mods(state.difficulty).fov_degrees);
    let half_fov = fov.clamp(1.0, 359.0).to_radians() * 0.5;
    let view_range = (state.personality.range_max * SIGHT_RANGE_MULT).max(SIGHT_MIN_RANGE);
    let fighting = state.target_id.is_some();
    let aw = &mut state.awareness;

    aw.sight_timer -= dt;
    aw.sight_elapsed += dt;
    let look = aw.sight_timer <= 0.0;
    let sight_dt = aw.sight_elapsed;
    if look {
        aw.sight_timer = SIGHT_CHECK_SECS;
        aw.sight_elapsed = 0.0;
    }

    for k in &mut aw.known {
        k.fresh = false;
    }
    let forward = (bot.yaw.cos(), bot.yaw.sin());
    let mut sensed_any = false;
    for (peer, p) in players {
        if !is_sensed(bot, *peer, p) {
            continue;
        }
        let d = flat_dist(bot.position, p.position);
        let k = aw.entry(p.id, p.position);
        k.fresh = true;
        let before = k.track.alert();
        k.track.age(dt);
        k.announced.1 += dt;

        // their shots this tick: the loudest one heard
        let shot = noises
            .iter()
            .filter(|n| n.shooter == *peer)
            .map(|n| (model::gunfire_floor(flat_dist(bot.position, n.pos), n.radius * hearing), n.pos))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((floor, at)) = shot.filter(|(f, _)| *f > 0.0) {
            k.track.raise_to(floor);
            k.last_pos = at;
        }

        // footsteps: sprinting, and landing from a jump or a fall
        let mut heard = 0.0;
        if !p.is_silent && !p.in_box {
            if p.is_sprinting {
                heard += SPRINT_NOISE_RATE * model::falloff(d, SPRINT_NOISE_RADIUS * hearing) * dt;
            }
            if k.was_airborne && !p.is_airborne {
                heard += LANDING_NOISE_BUMP * model::falloff(d, LANDING_NOISE_RADIUS * hearing);
            }
        }
        k.was_airborne = p.is_airborne;
        if heard > 0.0 {
            k.track.add(heard, HEARING_CAP);
            k.last_pos = p.position;
        }

        // sight, a few times a second
        if look {
            let prev = k.seen_pos.replace(p.position);
            let off_axis = off_axis(bot, p, forward, d, half_fov);
            if d <= view_range && off_axis <= 1.0 {
                let boxed = match prev {
                    _ if !p.in_box => Boxed::No,
                    Some(prev) if sight_dt > 0.0 && flat_dist(prev, p.position) / sight_dt > BOX_STILL_SPEED => {
                        Boxed::Moving
                    }
                    _ => Boxed::Still,
                };
                // a still box is scenery: no ray for it
                if boxed != Boxed::Still && has_line_of_sight(bot, p, maze) {
                    let input = SightInput {
                        distance: d,
                        view_range,
                        off_axis,
                        light: light_at(maze, p.position),
                        posture: posture(p),
                        boxed,
                    };
                    let cap = if p.in_box { BOX_AWARENESS_CAP } else { ENGAGED_AT };
                    k.track.add(model::sight_rate(&input, perception) * sight_dt, cap);
                    k.last_pos = p.position;
                }
            }
        }

        sensed_any |= k.track.since == 0.0;
        if let Some(up) = k.track.settle(dt, before) {
            aw.announce(p.id, up);
        }
    }
    aw.known.retain(|k| k.fresh);

    stall_hint(aw, bot, players, sensed_any || fighting, dt);
}

/// Anti-stall: a bot that has sensed nobody for `STALL_HINT_SECS` gets a
/// rough spot near the nearest enemy to head for.
fn stall_hint(aw: &mut Awareness, bot: &Player, players: &HashMap<PeerId, Player>, sensed: bool, dt: f32) {
    if let Some((_, left)) = &mut aw.hint {
        *left -= dt;
        if *left <= 0.0 {
            aw.hint = None;
        }
    }
    aw.stall = if sensed { 0.0 } else { aw.stall + dt };
    if sensed {
        aw.hints_given = 0;
    }
    if aw.stall < STALL_HINT_SECS {
        return;
    }
    aw.stall = 0.0;
    let nearest = players
        .iter()
        .filter(|(peer, p)| is_sensed(bot, **peer, p))
        .map(|(_, p)| p.position)
        .min_by(|a, b| flat_dist(*a, bot.position).total_cmp(&flat_dist(*b, bot.position)));
    if let Some(pos) = nearest {
        let mut rng = rand::thread_rng();
        let angle = rng.gen::<f32>() * std::f32::consts::TAU;
        let spread = (HINT_SPREAD * HINT_NARROWING.powi(aw.hints_given)).max(HINT_MIN_SPREAD);
        aw.hint = Some((rough_spot(pos, angle, rng.gen::<f32>().sqrt() * spread), HINT_SECS));
        aw.hints_given = aw.hints_given.saturating_add(1);
    }
}

/// A spot `r` from `pos` in direction `angle` (radians).
pub(crate) fn rough_spot(pos: (f32, f32, f32), angle: f32, r: f32) -> (f32, f32, f32) {
    (pos.0 + angle.cos() * r, pos.1, pos.2 + angle.sin() * r)
}

/// Angle between the bot's view and `p`, as a fraction of half its field of view.
fn off_axis(bot: &Player, p: &Player, forward: (f32, f32), d: f32, half_fov: f32) -> f32 {
    if d <= 1e-3 {
        return 0.0;
    }
    let dot = ((p.position.0 - bot.position.0) * forward.0 + (p.position.2 - bot.position.2) * forward.1) / d;
    dot.clamp(-1.0, 1.0).acos() / half_fov
}

fn posture(p: &Player) -> Posture {
    if p.is_prone {
        Posture::Prone
    } else if p.is_crouched {
        Posture::Crouched
    } else if p.is_sprinting {
        Posture::Sprinting
    } else {
        Posture::Standing
    }
}

/// How lit `pos` is by the maze's ceiling lights (fully lit with no maze).
fn light_at(maze: Option<&MazeData>, pos: (f32, f32, f32)) -> f32 {
    let Some(maze) = maze else {
        return 1.0;
    };
    let lights = maze
        .lights
        .iter()
        .map(|l| ((l.x as f32 + 0.5) * MAZE_CELL_SIZE, (l.y as f32 + 0.5) * MAZE_CELL_SIZE));
    model::light_level(lights, (pos.0, pos.2))
}
