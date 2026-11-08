use crate::player::Player;
use crate::maze::MazeData;
use crate::maze::analysis::MazeAnalysis;
use crate::bot::ai::{ BotState, BotBehavior };
use crate::bot::personality::BotArchetype;
use crate::maze::pathfinding::world_to_cell;
use crate::server::game::MAZE_CELL_SIZE;
use rand::Rng;

use super::routing::route;

//  Helper functions

pub(super) fn flank_behind_enemy(
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

pub(super) fn retreat_from_enemy(
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

pub(super) fn pick_hold_angle(
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

pub(super) fn roam_to_random_point(
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
