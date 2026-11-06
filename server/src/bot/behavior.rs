use std::collections::HashMap;
use crate::net::PeerId;
use crate::player::{Player, Team, WorldId};
use crate::maze::MazeData;
use crate::maze::analysis::MazeAnalysis;
use crate::bot::ai::{ BotState, BotBehavior };
use crate::bot::personality::BotArchetype;
use crate::maze::pathfinding::{ astar, world_to_cell };
use crate::server::game::MAZE_CELL_SIZE;
use rand::Rng;

/// Build a navigation grid from maze cells with prop positions marked impassable.
/// Bots route around props rather than through them.
fn make_nav_cells(maze: &MazeData) -> Vec<u8> {
    let mut nav = maze.cells.clone();
    for prop in &maze.props {
        let idx = (prop.y as usize) * (maze.width as usize) + (prop.x as usize);
        if idx < nav.len() {
            nav[idx] = 0;
        }
    }
    nav
}

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

    let nav_cells = make_nav_cells(maze);

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

    // What the bot knows: the enemy it can see (shooting picked it), whoever
    // shot it unseen, and where the nearest enemy is (bots always roughly know,
    // which keeps them heading for the fight).
    let visible = state.target_id.and_then(|id| find_enemy(players, bot, id));
    let alerted = state.tactics.alert.map(|(id, pos, _)| (find_enemy(players, bot, id), pos));
    let nearest_enemy = players
        .values()
        .filter(|p| is_enemy(bot, p))
        .min_by(|a, b| {
            let da = dist2(a.position, bot.position);
            let db = dist2(b.position, bot.position);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
    let threatened = visible.is_some() || alerted.is_some();

    // retreat when hurt
    let health_ratio = (bot.health as f32) / (bot.max_health.max(1) as f32);
    if health_ratio < state.personality.retreat_health_threshold {
        if let BotBehavior::Retreat { path, path_index } = &state.behavior {
            if *path_index < path.len() {
                return None; // keep retreating
            }
            // got away: hold an angle back toward them
            return pick_hold_angle(bot_cell, visible.or(nearest_enemy), analysis, maze, state, &nav_cells);
        }
        if !matches!(state.behavior, BotBehavior::HoldAngle { .. }) {
            return retreat_from_enemy(bot_cell, visible.or(nearest_enemy), maze, &nav_cells);
        }
    }

    // keep doing what we are doing unless it is finished or something more
    // urgent came up
    match &state.behavior {
        BotBehavior::Hunt { repath_timer, target_player_id, path, path_idx } => {
            let path_done = *path_idx >= path.len();
            match find_enemy(players, bot, *target_player_id) {
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
            match find_enemy(players, bot, *target_player_id) {
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
            if let Some(enemy) = visible {
                if dist2(enemy.position, bot.position).sqrt() <= state.personality.range_max {
                    return None; // they walked into our angle: fight from here
                }
                // seen but out of reach: go after them
            } else if alerted.is_none() && *hold_timer < *max_hold_time {
                return None; // keep holding
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
                if !threatened {
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

    if let Some(enemy) = visible.or(nearest_enemy) {
        let roll = rand::thread_rng().gen::<f32>();
        let total = (state.personality.push_weight
            + state.personality.flank_weight
            + state.personality.hold_angle_weight)
            .max(0.001);

        let push_chance = state.personality.push_weight / total;
        let flank_chance = state.personality.flank_weight / total;

        if roll < push_chance {
            // HUNT — A* directly to the enemy
            let path = route_to_player(&nav_cells, maze, bot_cell, enemy)?;
            return Some(BotBehavior::Hunt {
                path,
                path_idx: 0,
                target_player_id: enemy.id,
                repath_timer: state.personality.base_repath_interval,
            });
        } else if roll < push_chance + flank_chance {
            // FLANK — path to cell behind the enemy
            return flank_behind_enemy(bot_cell, enemy, state, maze, &nav_cells);
        } else {
            // HOLD ANGLE — find a strategic position
            return pick_hold_angle(bot_cell, Some(enemy), analysis, maze, state, &nav_cells);
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

    // idle behavior based on archetype
    match state.personality.archetype {
        BotArchetype::Camper | BotArchetype::Sniper => {
            pick_hold_angle(bot_cell, None, analysis, maze, state, &nav_cells)
        }
        _ => { roam_to_random_point(bot_cell, analysis, maze, &nav_cells) }
    }
}

//  Helper functions

fn flank_behind_enemy(
    bot_cell: (u8, u8),
    enemy: &Player,
    state: &BotState,
    maze: &MazeData,
    nav_cells: &[u8],
) -> Option<BotBehavior> {
    let behind_x = enemy.position.0 - enemy.yaw.cos() * MAZE_CELL_SIZE * 2.0;
    let behind_z = enemy.position.2 - enemy.yaw.sin() * MAZE_CELL_SIZE * 2.0;
    let behind_cell = world_to_cell(
        behind_x,
        behind_z,
        MAZE_CELL_SIZE,
        &maze.cells,
        maze.width,
        maze.height
    );
    let target_cell = behind_cell.or_else(||
        world_to_cell(
            enemy.position.0,
            enemy.position.2,
            MAZE_CELL_SIZE,
            &maze.cells,
            maze.width,
            maze.height
        )
    )?;
    let path = route(nav_cells, maze, bot_cell, target_cell)?;
    Some(BotBehavior::Flank {
        path,
        path_index: 0,
        target_player_id: enemy.id,
        repath_timer: state.personality.base_repath_interval,
    })
}

fn retreat_from_enemy(
    bot_cell: (u8, u8),
    nearest_enemy: Option<&Player>,
    maze: &MazeData,
    nav_cells: &[u8],
) -> Option<BotBehavior> {
    // retreat away from enemy - pick a cell in the opposite direction
    let (retreat_cx, retreat_cz) = if let Some(enemy) = nearest_enemy {
        let enemy_cx = enemy.position.0 / MAZE_CELL_SIZE;
        let enemy_cz = enemy.position.2 / MAZE_CELL_SIZE;
        let dx = (bot_cell.0 as f32) - enemy_cx;
        let dz = (bot_cell.1 as f32) - enemy_cz;
        let len = (dx * dx + dz * dz).sqrt().max(0.01);
        // project 8 cells away from enemy
        let tx = ((bot_cell.0 as f32) + (dx / len) * 8.0).clamp(1.0, (maze.width - 2) as f32);
        let tz = ((bot_cell.1 as f32) + (dz / len) * 8.0).clamp(1.0, (maze.height - 2) as f32);
        (tx, tz)
    } else {
        ((maze.width as f32) / 2.0, (maze.height as f32) / 2.0)
    };

    let retreat_cell = world_to_cell(
        retreat_cx * MAZE_CELL_SIZE,
        retreat_cz * MAZE_CELL_SIZE,
        MAZE_CELL_SIZE,
        &maze.cells,
        maze.width,
        maze.height
    )?;
    let path = route(nav_cells, maze, bot_cell, retreat_cell)?;
    Some(BotBehavior::Retreat {
        path,
        path_index: 0,
    })
}

fn pick_hold_angle(
    bot_cell: (u8, u8),
    nearest_enemy: Option<&Player>,
    analysis: &MazeAnalysis,
    maze: &MazeData,
    state: &BotState,
    nav_cells: &[u8],
) -> Option<BotBehavior> {
    let mut rng = rand::thread_rng();

    // pick strategic positions based on archetype
    let candidates: &[(u8, u8)] = match state.personality.archetype {
        BotArchetype::Sniper => {
            if !analysis.long_sightlines.is_empty() {
                &analysis.long_sightlines
            } else if !analysis.chokepoints.is_empty() {
                &analysis.chokepoints
            } else {
                &analysis.intersections
            }
        }
        BotArchetype::Camper => {
            if !analysis.room_entrances.is_empty() {
                &analysis.room_entrances
            } else if !analysis.chokepoints.is_empty() {
                &analysis.chokepoints
            } else {
                &analysis.intersections
            }
        }
        _ => {
            if !analysis.intersections.is_empty() {
                &analysis.intersections
            } else if !analysis.chokepoints.is_empty() {
                &analysis.chokepoints
            } else {
                &analysis.long_sightlines
            }
        }
    };

    if candidates.is_empty() {
        return None;
    }

    let target = candidates[rng.gen_range(0..candidates.len())];

    // face direction: toward enemy if we have one, otherwise toward map center
    let (face_dx, face_dz) = if let Some(enemy) = nearest_enemy {
        let dx = enemy.position.0 - ((target.0 as f32) + 0.5) * MAZE_CELL_SIZE;
        let dz = enemy.position.2 - ((target.1 as f32) + 0.5) * MAZE_CELL_SIZE;
        let len = (dx * dx + dz * dz).sqrt().max(0.01);
        (dx / len, dz / len)
    } else {
        let cx = (maze.width as f32) / 2.0 - (target.0 as f32);
        let cz = (maze.height as f32) / 2.0 - (target.1 as f32);
        let len = (cx * cx + cz * cz).sqrt().max(0.01);
        (cx / len, cz / len)
    };

    if target == bot_cell {
        // already at the position — hold immediately
        return Some(BotBehavior::HoldAngle {
            face_direction: (face_dx, face_dz),
            hold_timer: 0.0,
            max_hold_time: state.personality.reposition_interval,
        });
    }

    // walk to the hold position; on arrival decide_behavior switches to HoldAngle
    let path = route(nav_cells, maze, bot_cell, target)?;
    Some(BotBehavior::Reposition { path, path_index: 0, hold_face: Some((face_dx, face_dz)) })
}

fn roam_to_random_point(
    bot_cell: (u8, u8),
    analysis: &MazeAnalysis,
    maze: &MazeData,
    nav_cells: &[u8],
) -> Option<BotBehavior> {
    let mut rng = rand::thread_rng();

    // prefer intersections as roam destinations
    let target = if !analysis.intersections.is_empty() {
        analysis.intersections[rng.gen_range(0..analysis.intersections.len())]
    } else {
        // random walkable cell
        let mut attempts = 0;
        loop {
            let x = rng.gen_range(1..maze.width - 1);
            let y = rng.gen_range(1..maze.height - 1);
            let idx = (y as usize) * (maze.width as usize) + (x as usize);
            if maze.cells[idx] == 1 {
                break (x, y);
            }
            attempts += 1;
            if attempts > 100 {
                return None;
            }
        }
    };

    if target == bot_cell {
        return None;
    }

    let path = route(nav_cells, maze, bot_cell, target)?;
    Some(BotBehavior::Reposition { path, path_index: 0, hold_face: None })
}

/// `p` is someone `bot` fights: alive, in the maze, on the other team.
fn is_enemy(bot: &Player, p: &Player) -> bool {
    p.id != bot.id
        && p.is_alive
        && p.world == WorldId::Maze
        && p.team != bot.team
        && p.team != Team::Spectator
}

fn find_enemy<'a>(players: &'a HashMap<PeerId, Player>, bot: &Player, id: u32) -> Option<&'a Player> {
    players.values().find(|p| p.id == id && is_enemy(bot, p))
}

/// A* from `from` to `to`, without `from` itself: the bot is already in that
/// cell, and walking back to its centre first makes it stutter on every repath.
fn route(nav_cells: &[u8], maze: &MazeData, from: (u8, u8), to: (u8, u8)) -> Option<Vec<(u8, u8)>> {
    let mut path = astar(nav_cells, maze.width, maze.height, from, to)?;
    if path.len() > 1 {
        path.remove(0);
    }
    Some(path)
}

fn route_to_pos(
    nav_cells: &[u8],
    maze: &MazeData,
    from: (u8, u8),
    pos: (f32, f32, f32),
) -> Option<Vec<(u8, u8)>> {
    let goal = world_to_cell(pos.0, pos.2, MAZE_CELL_SIZE, &maze.cells, maze.width, maze.height)?;
    route(nav_cells, maze, from, goal)
}

fn route_to_player(
    nav_cells: &[u8],
    maze: &MazeData,
    from: (u8, u8),
    player: &Player,
) -> Option<Vec<(u8, u8)>> {
    route_to_pos(nav_cells, maze, from, player.position)
}

fn dist2(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dz = a.2 - b.2;
    dx * dx + dz * dz
}
