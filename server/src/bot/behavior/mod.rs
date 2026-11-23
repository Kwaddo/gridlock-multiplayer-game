use std::collections::HashMap;
use crate::net::PeerId;
use crate::player::Player;
use crate::maze::MazeData;
use crate::maze::analysis::MazeAnalysis;
use crate::bot::ai::{ BotState, BotBehavior };
use crate::bot::personality::BotArchetype;
use crate::maze::pathfinding::world_to_cell;
use crate::bot::awareness::tuning::INVESTIGATE_SECS;
use crate::server::game::MAZE_CELL_SIZE;
use rand::Rng;

mod routing;
mod tactics;

use routing::{dist2, find_enemy, is_enemy, route, route_to_player, route_to_pos};
use tactics::{
    flank_behind_enemy, hold_here, pick_hold_angle, retreat_from_enemy, roam_to_random_point, stance, Stance,
};

/// Seconds a wounded bot holds an angle where it got away to, while its
/// health comes back, before it falls back somewhere else.
const WOUNDED_HOLD_SECS: f32 = 8.0;

/*
    The "brain" of the bot. Called every tick from manager.rs before tick_bot (the "legs").
    Decides which BotBehavior the bot should be in based on:
      - its archetype and personality weights
      - whether it can see an enemy
      - its health vs retreat threshold
      - timers (repath, hold, give_up)

    Returns Some(new_behavior) to switch, or None to keep current behavior.
*/

pub fn decide_behavior(
    bot: &Player,
    state: &BotState,
    players: &HashMap<PeerId, Player>,
    maze: &MazeData,
    analysis: &MazeAnalysis
) -> Option<BotBehavior> {
    if !bot.is_alive {
        return None;
    }

    // Navigation grid: maze cells with prop positions marked impassable, so
    // bots route around props rather than through them. Cached on the maze.
    let nav_cells = maze.nav_cells();

    let bot_cell = match
        world_to_cell(
            bot.position.0,
            bot.position.2,
            MAZE_CELL_SIZE,
            &maze.cells,
            maze.width,
            maze.height
        )
    {
        Some(c) => c,
        None => {
            // Bot is in an invalid position — find any random walkable cell as fallback
            let mut rng = rand::thread_rng();
            let mut fallback = None;
            for _ in 0..50 {
                let x = rng.gen_range(1..maze.width - 1);
                let y = rng.gen_range(1..maze.height - 1);
                let idx = (y as usize) * (maze.width as usize) + (x as usize);
                if maze.cells[idx] == 1 {
                    fallback = Some((x, y));
                    break;
                }
            }
            match fallback {
                Some(c) => c,
                None => {
                    return None;
                }
            }
        }
    };

    // What the bot knows (bot::awareness): the enemy it can see (shooting
    // picked it), whoever shot it unseen, the nearest enemy it is engaged
    // with and still tracking, and where it noticed something it hasn't
    // made out yet. It no longer knows where everyone is.
    let aware = &state.awareness;
    let visible = state.target_id.and_then(|id| find_enemy(players, bot, id));
    let alerted = state.tactics.alert.map(|(id, pos, _)| (find_enemy(players, bot, id), pos));
    // an enemy the bot is fighting, sensed moments ago
    let known = |id: u32| find_enemy(players, bot, id).filter(|e| aware.tracks(e.id));
    let nearest_enemy = players
        .values()
        .filter(|p| is_enemy(bot, p) && aware.tracks(p.id))
        .min_by(|a, b| {
            let da = dist2(a.position, bot.position);
            let db = dist2(b.position, bot.position);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
    let threatened = visible.is_some() || alerted.is_some();
    let suspicion = aware.suspicion(bot.position);

    // Retreat when hurt. Health comes back out of combat (as it does for
    // players), so this ends: until then the bot falls back, holds, falls back.
    let health_ratio = (bot.health as f32) / (bot.max_health.max(1) as f32);
    let wounded = health_ratio < state.personality.retreat_health_threshold;
    if wounded {
        match &state.behavior {
            BotBehavior::Retreat { path, path_index } if *path_index < path.len() => {
                return None; // keep retreating
            }
            // got away: hold an angle back toward them while health comes back
            BotBehavior::Retreat { .. } => {
                return Some(hold_here(bot, visible.or(nearest_enemy), WOUNDED_HOLD_SECS));
            }
            // holding: the hold ends as usual below
            BotBehavior::HoldAngle { .. } => {}
            _ => {
                if let Some(b) = retreat_from_enemy(bot_cell, visible.or(nearest_enemy), maze, &nav_cells) {
                    return Some(b);
                }
            }
        }
    }

    // keep doing what we are doing unless it is finished or something more
    // urgent came up
    match &state.behavior {
        BotBehavior::Hunt { repath_timer, target_player_id, path, path_idx } => {
            let path_done = *path_idx >= path.len();
            match known(*target_player_id) {
                Some(enemy) if *repath_timer <= 0.0 || path_done => {
                    // the target moved on: path to where they are now
                    if let Some(new_path) = route_to_player(&nav_cells, maze, bot_cell, enemy) {
                        return Some(BotBehavior::Hunt {
                            path: new_path,
                            path_idx: 0,
                            target_player_id: enemy.id,
                            repath_timer: state.personality.base_repath_interval,
                        });
                    }
                }
                Some(_) => return None, // still on the way
                None => {} // target dead or gone: pick something new
            }
        }

        BotBehavior::Flank { repath_timer, target_player_id, path, path_index } => {
            let path_done = *path_index >= path.len();
            match known(*target_player_id) {
                Some(enemy) if *repath_timer <= 0.0 || path_done => {
                    if path_done {
                        // made it round the back: go straight for them
                        if let Some(new_path) = route_to_player(&nav_cells, maze, bot_cell, enemy) {
                            return Some(BotBehavior::Hunt {
                                path: new_path,
                                path_idx: 0,
                                target_player_id: enemy.id,
                                repath_timer: state.personality.base_repath_interval,
                            });
                        }
                    }
                    return flank_behind_enemy(bot_cell, enemy, state, maze, &nav_cells);
                }
                Some(_) => return None,
                None => {}
            }
        }

        BotBehavior::HoldAngle { hold_timer, max_hold_time, .. } => {
            let holding = *hold_timer < *max_hold_time;
            if let Some(enemy) = visible {
                if dist2(enemy.position, bot.position).sqrt() <= state.personality.range_max {
                    return None; // they walked into our angle: fight from here
                }
                // seen but out of reach: go after them, unless set on holding
                if holding && stance(state) == Stance::Hold {
                    return None;
                }
            } else if alerted.is_none() && holding && aware.hint(bot.position).is_none() {
                return None; // keep holding (turning toward anything it notices)
            }
            // shot from somewhere unseen, or held long enough: move
        }

        BotBehavior::Investigate { give_up_timer, path, path_index } => {
            if !threatened && *give_up_timer > 0.0 && *path_index < path.len() {
                return None; // still investigating
            }
        }

        BotBehavior::Retreat { path, path_index } => {
            if *path_index < path.len() {
                return None; // still retreating
            }
        }

        BotBehavior::Reposition { path, path_index, hold_face } => {
            let destination = path.last().copied();
            if *path_index < path.len() {
                // set on holding, an enemy in sight doesn't turn us from the spot
                let keep_going = hold_face.is_some() && stance(state) == Stance::Hold && alerted.is_none();
                // roaming and noticed something, or given a hint: go and look instead
                let curious = (hold_face.is_none() && suspicion.is_some()) || aware.hint(bot.position).is_some();
                if (!threatened && !curious) || keep_going {
                    return None; // still on the way
                }
            } else if threatened {
                // something came up on the way: deal with it below
            } else if destination.is_some_and(|d| d != bot_cell) {
                // the path was dropped (blocked or pushed off it): plan again
                if let Some(path) = destination.and_then(|d| route(&nav_cells, maze, bot_cell, d)) {
                    return Some(BotBehavior::Reposition { path, path_index: 0, hold_face: *hold_face });
                }
            } else if let Some(face) = hold_face {
                // arrived: hold the angle we came for
                return Some(BotBehavior::HoldAngle {
                    face_direction: *face,
                    hold_timer: 0.0,
                    max_hold_time: state.personality.reposition_interval,
                });
            }
        }

        BotBehavior::Static => {}

        // Patrol is only used in shooting range, don't override
        BotBehavior::Patrol { .. } => {
            return None;
        }
    }

    // --- Pick a new behavior ---

    // still hurt (a hold to recover ended, say): fall back somewhere else
    if wounded {
        if let Some(b) = retreat_from_enemy(bot_cell, visible.or(nearest_enemy), maze, &nav_cells) {
            return Some(b);
        }
    }

    // someone we can't see shot us: go and find them
    if visible.is_none() {
        if let Some((attacker, pos)) = alerted {
            if let Some(enemy) = attacker {
                if let Some(path) = route_to_player(&nav_cells, maze, bot_cell, enemy) {
                    return Some(BotBehavior::Hunt {
                        path,
                        path_idx: 0,
                        target_player_id: enemy.id,
                        repath_timer: state.personality.base_repath_interval,
                    });
                }
            }
            if let Some(path) = route_to_pos(&nav_cells, maze, bot_cell, pos) {
                return Some(BotBehavior::Investigate { path, path_index: 0, give_up_timer: 5.0 });
            }
        }
    }

    // The stance is rolled every few seconds (tick_bot), not per decision, so
    // the bot doesn't flip between pushing and holding. Whatever it can't do
    // from here, it does one of the others instead of standing still.
    if let Some(enemy) = visible.or(nearest_enemy) {
        // HUNT — A* directly to the enemy
        let push = || route_to_player(&nav_cells, maze, bot_cell, enemy).map(|path| BotBehavior::Hunt {
            path,
            path_idx: 0,
            target_player_id: enemy.id,
            repath_timer: state.personality.base_repath_interval,
        });
        // FLANK — path to cell behind the enemy
        let flank = || flank_behind_enemy(bot_cell, enemy, state, maze, &nav_cells);
        // HOLD ANGLE — find a strategic position
        let hold = || pick_hold_angle(bot_cell, Some(enemy), analysis, maze, state, &nav_cells);
        let picked = match stance(state) {
            Stance::Push => push().or_else(flank).or_else(hold),
            Stance::Flank => flank().or_else(push).or_else(hold),
            Stance::Hold => hold().or_else(push).or_else(flank),
        };
        if picked.is_some() {
            return picked;
        }
    }

    // noticed something (a shot, footsteps, a glimpse): go and look, warily
    // (see tick_bot). Or, nobody sensed in a long while: head for the rough
    // area the anti-stall hint gives.
    if let Some(pos) = suspicion.or_else(|| aware.hint(bot.position)) {
        if let Some(path) = route_to_pos(&nav_cells, maze, bot_cell, pos) {
            return Some(BotBehavior::Investigate { path, path_index: 0, give_up_timer: INVESTIGATE_SECS });
        }
    }

    // no enemy — check last known position for investigation
    if let Some(last_pos) = state.last_known_enemy_pos {
        let roll = rand::thread_rng().gen::<f32>();
        if roll < state.personality.noise_investigation_weight {
            if let Some(path) = route_to_pos(&nav_cells, maze, bot_cell, last_pos) {
                return Some(BotBehavior::Investigate {
                    path,
                    path_index: 0,
                    give_up_timer: 5.0,
                });
            }
        }
    }

    // idle behavior based on archetype, else the other one
    let hold = || pick_hold_angle(bot_cell, None, analysis, maze, state, &nav_cells);
    let roam = || roam_to_random_point(bot_cell, analysis, maze, &nav_cells);
    match state.personality.archetype {
        BotArchetype::Camper | BotArchetype::Sniper => hold().or_else(roam),
        _ => roam().or_else(hold),
    }
}
