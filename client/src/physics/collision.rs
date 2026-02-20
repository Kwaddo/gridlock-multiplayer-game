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
///
/// Special-case: when the player is grounded and very close to a cube's top surface
/// we prefer keeping them "on top" (clamp to the top area) instead of pushing them
/// sideways and accidentally dropping them off small platforms.
pub fn resolve_horizontal(
    position: &mut Vec3,
    velocity: &mut Vec3,
    player_height: f32,
    is_grounded: bool,
) {
    let py = position.y;
    let head_y = py + player_height;
    const TOP_EPS: f32 = 0.06; // tolerance for "on-top" detection
    const STICKY_OVERLAP: f32 = 0.06; // small pushes below this will not knock you off a platform

    // If we're standing on a cube top, remember that cube so we can avoid
    // tiny side-nudges from *other* cubes pushing us off the top.
    let mut supporting_cube: Option<(Vec3, Vec3)> = None;
    if is_grounded {
        for (c_center, c_size) in all_cubes() {
            let half = *c_size * 0.5;
            let c_min = *c_center - half;
            let c_max = *c_center + half;
            if py >= c_max.y - TOP_EPS && py <= c_max.y + TOP_EPS {
                if position.x >= c_min.x - PLAYER_RADIUS
                    && position.x <= c_max.x + PLAYER_RADIUS
                    && position.z >= c_min.z - PLAYER_RADIUS
                    && position.z <= c_max.z + PLAYER_RADIUS
                {
                    supporting_cube = Some((c_min, c_max));
                    break;
                }
            }
        }
    }

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

        // If player is grounded and very close to the cube top, prefer keeping them on top
        if is_grounded && py >= cube_max.y - TOP_EPS && py <= cube_max.y + TOP_EPS {
            if position.x >= cube_min.x - PLAYER_RADIUS
                && position.x <= cube_max.x + PLAYER_RADIUS
                && position.z >= cube_min.z - PLAYER_RADIUS
                && position.z <= cube_max.z + PLAYER_RADIUS
            {
                // Clamp into the top-face area so small nudges don't push player off
                position.x = position
                    .x
                    .clamp(cube_min.x + PLAYER_RADIUS, cube_max.x - PLAYER_RADIUS);
                position.z = position
                    .z
                    .clamp(cube_min.z + PLAYER_RADIUS, cube_max.z - PLAYER_RADIUS);

                // Kill velocity into the walls so the player doesn't immediately slide off
                if position.x <= cube_min.x + PLAYER_RADIUS + 0.001 && velocity.x < 0.0 {
                    velocity.x = 0.0;
                }
                if position.x >= cube_max.x - PLAYER_RADIUS - 0.001 && velocity.x > 0.0 {
                    velocity.x = 0.0;
                }
                if position.z <= cube_min.z + PLAYER_RADIUS + 0.001 && velocity.z < 0.0 {
                    velocity.z = 0.0;
                }
                if position.z >= cube_max.z - PLAYER_RADIUS - 0.001 && velocity.z > 0.0 {
                    velocity.z = 0.0;
                }

                continue;
            }
        }

        if dist_sq < PLAYER_RADIUS * PLAYER_RADIUS && dist_sq > 0.0001 {
            // Push player out
            let dist = dist_sq.sqrt();
            let overlap = PLAYER_RADIUS - dist;
            let nx = dx / dist;
            let nz = dz / dist;

            // If we're standing on a supporting cube, ignore very small pushes
            // from *other* cubes that would move us off the support area.
            if let Some((sup_min, sup_max)) = supporting_cube {
                // If this cube is the support cube itself, allow normal handling
                if !(sup_min == cube_min && sup_max == cube_max) {
                    let new_x = position.x + nx * overlap;
                    let new_z = position.z + nz * overlap;

                    let outside_x = new_x < (sup_min.x + PLAYER_RADIUS - 0.001)
                        || new_x > (sup_max.x - PLAYER_RADIUS + 0.001);
                    let outside_z = new_z < (sup_min.z + PLAYER_RADIUS - 0.001)
                        || new_z > (sup_max.z - PLAYER_RADIUS + 0.001);

                    if (outside_x || outside_z) && overlap < STICKY_OVERLAP {
                        // treat it as non-colliding (sticky top)
                        continue;
                    }
                }
            }

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
