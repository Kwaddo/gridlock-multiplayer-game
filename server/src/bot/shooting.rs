use std::collections::HashMap;
use crate::net::PeerId;
use rand::Rng;
use crate::bot::personality::difficulty_mods;
use crate::maze::MazeData;
use crate::player::{Player, WorldId};
use crate::server::game::MAZE_CELL_SIZE;
use crate::bot::ai::BotState;
use crate::weapons_stats::get_weapon_stats;

/// Weapons with a shorter range than this are melee: no hit beyond their reach.
const MELEE_RANGE_CUTOFF: f32 = 5.0;


#[derive(Clone, Copy)]
pub struct BotShootingConfig {
    pub fire_rate: f32, // seconds between bot-triggered shots
    pub angular_offset: f32, // max angle offset in degrees for inaccuracy
    pub reaction_time: f32, // time in seconds before bot starts shooting after seeing target
    pub detection_range: Option<f32>, // override weapon range for target detection (None = use weapon range)
    pub aim_threshold: f32, // max angular error before bot is allowed to fire
}

pub struct BotShootingState {
    pub fire_rate: f32, // time until next shot can be fired
    pub has_target: bool, // whether bot currently has a valid target in sight
    pub reaction_remaining: f32, // time remaining before bot will start shooting at target
}

pub struct ShotDecision {
    pub target_id: u32,
    pub damage: Option<u32>,
}

pub fn try_shoot(
    bot_addr: PeerId,
    players: &HashMap<PeerId, Player>,
    bot_state: &mut BotState,
    maze: Option<&MazeData>,
    dt: f32,
) -> Option<ShotDecision> {

    // timers run every tick, whatever else happens
    bot_state.shooting.fire_rate = (bot_state.shooting.fire_rate - dt).max(0.0);
    bot_state.shooting.reaction_remaining = (bot_state.shooting.reaction_remaining - dt).max(0.0);
    if let Some((_, since)) = &mut bot_state.tactics.lost_target {
        *since += dt;
    }

    // resolve bot player from players using bot_addr
    let bot = match players.get(&bot_addr) {
        Some(p) => p,
        None => return None, // bot player not found (shouldn't happen)
    };
    let weapon_stats = get_weapon_stats(&bot.current_weapon);
    let mods = difficulty_mods(bot_state.difficulty);

    // find target: who can we see and shoot at?
    let runtime_fov = bot_state.personality.base_fov_degrees.max(mods.fov_degrees);
    let range = bot_state.shooting_config.detection_range.unwrap_or(weapon_stats.range);
    let can_engage = |p: &Player| {
        p.id != bot.id
            && p.is_alive
            && p.world == bot.world
            && distance(bot.position, p.position) <= range
            && is_in_fov(bot, p, runtime_fov)
            && has_line_of_sight(bot, p, maze)
            && (if bot.world == WorldId::Lobby {
                !p.is_bot
            } else {
                bot.is_hostile_to(p)
            })
    };
    let current = bot_state
        .target_id
        .and_then(|id| players.values().find(|p| p.id == id))
        .filter(|p| can_engage(p));
    let nearest = players.values().filter(|p| can_engage(p)).min_by(|a, b| {
        let da = distance(bot.position, a.position);
        let db = distance(bot.position, b.position);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    });
    // Stick with the enemy we are fighting: switching costs a new reaction
    // time, and flipping between two at similar range meant never firing.
    // Only a clearly closer one (more so for better bots) takes over.
    let target = match (current, nearest) {
        (Some(c), Some(n))
            if n.id != c.id
                && distance(bot.position, n.position)
                    < distance(bot.position, c.position) * (1.0 - mods.target_stickiness * 0.6) =>
        {
            Some(n)
        }
        (Some(c), _) => Some(c),
        (None, n) => n,
    };

    let Some(target) = target else {
        if let Some(id) = bot_state.target_id.take() {
            bot_state.tactics.lost_target = Some((id, 0.0));
        }
        bot_state.shooting.has_target = false;
        return None; // no valid targets
    };

    // handle target change
    let new_target = target.id;

    if bot_state.target_id != Some(new_target) {
        if let Some(old) = bot_state.target_id {
            bot_state.tactics.lost_target = Some((old, 0.0));
        }
        // Someone we only just lost sight of (behind a corner, say) needs no
        // fresh reaction; a new face does.
        let seen_moments_ago = matches!(
            bot_state.tactics.lost_target,
            Some((id, since)) if id == new_target && since <= mods.wall_read_memory_time
        );
        bot_state.target_id = Some(new_target);
        bot_state.shooting.has_target = true;
        if !seen_moments_ago {
            bot_state.shooting.reaction_remaining = bot_state.shooting_config.reaction_time;
            return None; // just acquired target, start reaction timer
        }
    }

    // has target and reaction time elapsed, attempt to shoot
    if bot_state.shooting.reaction_remaining > 0.0 {
        return None; // still reacting, can't shoot yet
    }

        // gate fire on aim alignment — bot must actually be looking at the target, this is to guard against bot firing when target is not visually in sight
        {
            let dx = target.position.0 - bot.position.0;
            let dy = target.position.1 - bot.position.1;
            let dz = target.position.2 - bot.position.2;
            let horiz = (dx * dx + dz * dz).sqrt();
            let target_yaw = dz.atan2(dx);
            let target_pitch = -(dy / horiz.max(0.001)).atan();

            let yaw_err = {
                let d = target_yaw - bot_state.current_yaw;
                ((d + std::f32::consts::PI).rem_euclid(2.0 * std::f32::consts::PI)
                    - std::f32::consts::PI).abs()
            };
            let pitch_err = (target_pitch - bot_state.current_pitch).abs();

            if yaw_err > bot_state.shooting_config.aim_threshold
                || pitch_err > bot_state.shooting_config.aim_threshold * 1.5
            {
                return None; // still tracking, not on target yet
            }
        }

        if bot_state.shooting.fire_rate > 0.0 {
            return None; // still cooling down, can't shoot
        }

        // Melee (Gun Game's final Punch level) only lands within arm's reach;
        // the bot still tracks the target from afar and closes in.
        if weapon_stats.range < MELEE_RANGE_CUTOFF
            && distance(bot.position, target.position) > weapon_stats.range
        {
            return None;
        }

        let weapon_min_interval = if weapon_stats.shots_per_sec > 0.0 {
            1.0 / weapon_stats.shots_per_sec
        } else {
            bot_state.shooting_config.fire_rate
        };
        let mut rng = rand::thread_rng();
        let jitter = rng.gen_range(0.85f32..1.15f32);
        let interval = bot_state.shooting_config.fire_rate.max(weapon_min_interval) * jitter;
        bot_state.shooting.fire_rate = interval;

        // Simple probabilistic hit check driven by bot accuracy + distance.
        let target_dist = distance(bot.position, target.position);
        let max_range = bot_state
            .shooting_config
            .detection_range
            .unwrap_or(weapon_stats.range)
            .max(0.001);
        let distance_penalty = (target_dist / max_range).clamp(0.0, 1.0);
        let accuracy_from_offset = (1.0 - bot_state.shooting_config.angular_offset * 1.8).clamp(0.05, 0.98);
        let hit_chance = (accuracy_from_offset - 0.35 * distance_penalty).clamp(0.05, 0.98);

        // Roll each pellet independently so shotgun damage naturally falls off with distance.
        let total_damage: u32 = (0..weapon_stats.pellet_count)
            .filter(|_| rng.gen::<f32>() <= hit_chance)
            .map(|_| weapon_stats.damage)
            .sum();

        Some(ShotDecision {
            target_id: new_target,
            damage: if total_damage > 0 { Some(total_damage) } else { None },
        })
}


fn distance(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dz = a.2 - b.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn is_in_fov(bot: &Player, target: &Player, fov_degrees: f32) -> bool {
    let dx = target.position.0 - bot.position.0;
    let dz = target.position.2 - bot.position.2;
    let len_sq = dx * dx + dz * dz;
    if len_sq <= 1e-6 {
        return true;
    }

    let fov = fov_degrees.clamp(1.0, 359.0);
    let half_fov_cos = (fov.to_radians() * 0.5).cos();
    let inv_len = len_sq.sqrt().recip();
    let to_target = (dx * inv_len, dz * inv_len);
    let forward = (bot.yaw.cos(), bot.yaw.sin());
    let dot = to_target.0 * forward.0 + to_target.1 * forward.1;
    dot >= half_fov_cos
}

fn has_line_of_sight(bot: &Player, target: &Player, maze: Option<&MazeData>) -> bool {
    // Lobby has no server-side wall grid representation; only maze mode is occluded here.
    if bot.world != WorldId::Maze {
        return true;
    }
    let Some(maze) = maze else {
        return true;
    };

    let x0 = bot.position.0;
    let z0 = bot.position.2;
    let dx = target.position.0 - x0;
    let dz = target.position.2 - z0;
    let dist = (dx * dx + dz * dz).sqrt();
    if dist <= 1e-5 {
        return true;
    }

    // Sample several points per cell along the segment and reject as soon as it enters a wall cell.
    let steps = ((dist / (MAZE_CELL_SIZE * 0.2)).ceil() as i32).max(1);
    for i in 1..steps {
        let t = i as f32 / steps as f32;
        let x = x0 + dx * t;
        let z = z0 + dz * t;
        let cx = (x / MAZE_CELL_SIZE).floor() as i32;
        let cy = (z / MAZE_CELL_SIZE).floor() as i32;

        if cx < 0 || cy < 0 || cx >= maze.width as i32 || cy >= maze.height as i32 {
            return false;
        }

        let idx = cy as usize * maze.width as usize + cx as usize;
        if maze.cells.get(idx).copied().unwrap_or(0) == 0 {
            return false;
        }
    }

    true
}
