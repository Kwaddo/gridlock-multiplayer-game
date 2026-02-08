use crate::types::maze::Maze;
use crate::world::lobby::{LANDMARK_CUBES, PARKOUR_CUBES};
use macroquad::prelude::*;

pub const PLAYER_RADIUS: f32 = 0.25;

/// Iterator over all collidable cubes (landmarks + parkour)
pub fn all_cubes() -> impl Iterator<Item = &'static (Vec3, Vec3)> {
    LANDMARK_CUBES.iter().chain(PARKOUR_CUBES.iter())
}

/// Resolve vertical collision: land on cube tops, bump head on cube bottoms, land on ground.
pub fn resolve_vertical(
    position: &mut Vec3,
    velocity: &mut Vec3,
    player_height: f32,
    is_grounded: &mut bool,
) {
    let px = position.x;
    let py = position.y; // foot Y
    let pz = position.z;
    let head_y = py + player_height;
    let mut best_ground = 0.0_f32;

    for (center, size) in all_cubes() {
        let half = *size * 0.5;
        let cube_min = *center - half;
        let cube_max = *center + half;

        // Check XZ overlap (player circle vs cube AABB)
        let closest_x = px.clamp(cube_min.x, cube_max.x);
        let closest_z = pz.clamp(cube_min.z, cube_max.z);
        let dx = px - closest_x;
        let dz = pz - closest_z;
        if dx * dx + dz * dz > PLAYER_RADIUS * PLAYER_RADIUS {
            continue;
        }

        // Landing on top
        if py >= cube_max.y - 0.1 && velocity.y <= 0.0 && cube_max.y > best_ground {
            best_ground = cube_max.y;
        }

        // Head bump
        if velocity.y > 0.0 && head_y >= cube_min.y && py < cube_min.y {
            position.y = cube_min.y - player_height;
            velocity.y = 0.0;
        }
    }

    // Apply ground landing
    if position.y <= best_ground {
        position.y = best_ground;
        *is_grounded = true;
        velocity.y = 0.0;
    }
}

/// Resolve horizontal collision: push player out of cube side walls.
pub fn resolve_horizontal(position: &mut Vec3, velocity: &mut Vec3, player_height: f32) {
    let py = position.y;
    let head_y = py + player_height;

    for (center, size) in all_cubes() {
        let half = *size * 0.5;
        let cube_min = *center - half;
        let cube_max = *center + half;

        // Only collide with sides if there's vertical overlap
        if head_y <= cube_min.y || py >= cube_max.y {
            continue;
        }

        // Find closest point on cube XZ to player
        let closest_x = position.x.clamp(cube_min.x, cube_max.x);
        let closest_z = position.z.clamp(cube_min.z, cube_max.z);
        let dx = position.x - closest_x;
        let dz = position.z - closest_z;
        let dist_sq = dx * dx + dz * dz;

        if dist_sq < PLAYER_RADIUS * PLAYER_RADIUS && dist_sq > 0.0001 {
            // Push player out
            let dist = dist_sq.sqrt();
            let overlap = PLAYER_RADIUS - dist;
            let nx = dx / dist;
            let nz = dz / dist;
            position.x += nx * overlap;
            position.z += nz * overlap;

            // Kill velocity into the wall
            let dot = velocity.x * nx + velocity.z * nz;
            if dot < 0.0 {
                velocity.x -= dot * nx;
                velocity.z -= dot * nz;
            }
        } else if dist_sq < 0.0001 {
            // Player center is inside cube — push out on shortest axis
            let push = vec4(
                position.x - cube_min.x,
                cube_max.x - position.x,
                position.z - cube_min.z,
                cube_max.z - position.z,
            );
            let min_push = push.x.min(push.y).min(push.z).min(push.w);

            if min_push == push.x {
                position.x = cube_min.x - PLAYER_RADIUS;
                velocity.x = velocity.x.min(0.0);
            } else if min_push == push.y {
                position.x = cube_max.x + PLAYER_RADIUS;
                velocity.x = velocity.x.max(0.0);
            } else if min_push == push.z {
                position.z = cube_min.z - PLAYER_RADIUS;
                velocity.z = velocity.z.min(0.0);
            } else {
                position.z = cube_max.z + PLAYER_RADIUS;
                velocity.z = velocity.z.max(0.0);
            }
        }
    }
}

/// Resolve maze wall collision using maze's built-in collision detection
pub fn resolve_maze(position: &mut Vec3, velocity: &mut Vec3, maze: &Maze) {
    if !maze.is_blocked(*position, PLAYER_RADIUS) {
        return;
    }

    // Try sliding along walls by reverting one axis at a time
    let original = *position;

    // Try keeping X, reverting Z
    position.z -= velocity.z * 0.016;
    if !maze.is_blocked(*position, PLAYER_RADIUS) {
        velocity.z = 0.0;
        return;
    }

    // Try keeping Z, reverting X
    *position = original;
    position.x -= velocity.x * 0.016;
    if !maze.is_blocked(*position, PLAYER_RADIUS) {
        velocity.x = 0.0;
        return;
    }

    // Both failed, revert both
    position.x -= velocity.x * 0.016;
    position.z -= velocity.z * 0.016;
    velocity.x = 0.0;
    velocity.z = 0.0;
}
