use crate::physics::{ BotMovementState, PhysicsConfig, simulate_bot_physics };
use crate::bot::shooting::{ BotShootingConfig, BotShootingState };
use crate::player::Player;
use crate::bot::personality::{ BotArchetype, BotDifficulty, BotPersonality, BotPersonalityId };
use crate::maze::pathfinding::{ cell_to_world, world_to_cell };
use crate::server::game::MAZE_CELL_SIZE;
use glam::Vec3;

#[allow(dead_code)]
pub struct BotState {
    pub movement: BotMovementState,
    pub behavior: BotBehavior,
    pub can_shoot: bool,
    pub shooting: BotShootingState,
    pub shooting_config: BotShootingConfig,
    pub target_id: Option<u32>,
    pub reaction_timer: f32,  
    pub personality: BotPersonality,
    pub difficulty: BotDifficulty,
    pub archetype: BotArchetype,
    pub personality_id: Option<BotPersonalityId>,
    pub last_known_enemy_pos: Option<(f32, f32, f32)>,
    pub behavior_decision_timer: f32, // timer for how often the bot reevaluates its behavior
    pub stuck_timer: f32, // timer for how long the bot has been stuck in the same position
    pub idle_look_timer: f32, // accumulator for idle look-around yaw sweep

    pub current_yaw: f32, // current facing direction in radians
    pub current_pitch: f32, // current pitch angle for looking up/down
}

const PATH_ARRIVAL_THRESHOLD: f32 = 0.5; // distance threshold for considering a bot has arrived at a path node

#[derive(Clone)]
pub enum BotBehavior {
    Static,
    Patrol {
        waypoints: Vec<(f32, f32, f32)>,
        current: usize,
        arrival_threshold: f32,
    },
    Hunt {
        path: Vec<(u8, u8)>, // A* path
        path_idx: usize,
        target_player_id: u32,
        repath_timer: f32,
    },
    HoldAngle {
        face_direction: (f32, f32), // world direction to face (dx, dz)
        hold_timer: f32, // how long we've been holding
        max_hold_time: f32, // from personality.reposition_interval
    },
    Flank {
        path: Vec<(u8, u8)>,
        path_index: usize,
        target_player_id: u32,
        repath_timer: f32,
    },
    Retreat {
        path: Vec<(u8, u8)>,
        path_index: usize,
    },
    Investigate {
        path: Vec<(u8, u8)>,
        path_index: usize,
        give_up_timer: f32, // seconds before giving up
    },
}

pub fn tick_bot(
    bot: &mut Player,
    state: &mut BotState,
    dt: f32,
    physics: &PhysicsConfig,
    maze_cells: Option<(&[u8], u8, u8, &[crate::maze::types::Prop])>,
) {
    let mut desired_dir = Vec3::ZERO; // movement we intend to apply every tick
    let want_jump = false; // whether we want to jump this tick
    let speed: f32 = physics.walk_speed;

    match &mut state.behavior {
        BotBehavior::Static => {}

        BotBehavior::Patrol { waypoints, current, arrival_threshold } => {
            if !waypoints.is_empty() {
                let waypoint = waypoints[*current];
                desired_dir = Vec3::new(
                    waypoint.0 - bot.position.0,
                    0.0,
                    waypoint.2 - bot.position.2
                );

                if desired_dir.length() < *arrival_threshold {
                    // if we're close enough to the waypoint, switch to the next one
                    *current = (*current + 1) % waypoints.len();
                } else {
                    desired_dir = desired_dir.normalize();
                }
            }
        }
        // path-following behaviors: Hunt, Flank, Retreat, Investigate
        // all share the same core logic: walk toward the next cell in the A* path
        /*
            for each of the 4 hunting bots
            follow A* cell by cell so we can brain it later on
            retreat has no timer cause they just escape but hunt and flank do to know when to change ur path or give up hunting a player
            Investigate as well but stops because there will be nothjing to investigate in a while
            Hold angle doesnt move because we will call it when we find a good angle in step 5
        */
        BotBehavior::Hunt { path, path_idx, repath_timer, .. } => {
            // .. means ignore remaining field because its not relevant to target player
            *repath_timer -= dt;
            if let Some(&cell) = path.get(*path_idx) {
                let (wx, wz) = cell_to_world(cell.0, cell.1, MAZE_CELL_SIZE);
                desired_dir = Vec3::new(wx - bot.position.0, 0.0, wz - bot.position.2);
                if desired_dir.length() < PATH_ARRIVAL_THRESHOLD {
                    *path_idx += 1; // advance to next cell
                } else {
                    desired_dir = desired_dir.normalize();
                }
            } else {
                *repath_timer = 0.0;
            }
        }

        BotBehavior::Flank { path, path_index, repath_timer, .. } => {
            *repath_timer -= dt;
            if let Some(&cell) = path.get(*path_index) {
                let (wx, wz) = cell_to_world(cell.0, cell.1, MAZE_CELL_SIZE);
                desired_dir = Vec3::new(wx - bot.position.0, 0.0, wz - bot.position.2);
                if desired_dir.length() < PATH_ARRIVAL_THRESHOLD {
                    *path_index += 1;
                } else {
                    desired_dir = desired_dir.normalize();
                }
            } else {
                *repath_timer = 0.0;
            }
        }

        BotBehavior::Retreat { path, path_index } => {
            if let Some(&cell) = path.get(*path_index) {
                let (wx, wz) = cell_to_world(cell.0, cell.1, MAZE_CELL_SIZE);
                desired_dir = Vec3::new(wx - bot.position.0, 0.0, wz - bot.position.2);
                if desired_dir.length() < PATH_ARRIVAL_THRESHOLD {
                    *path_index += 1;
                } else {
                    desired_dir = desired_dir.normalize();
                }
            }
        }

        BotBehavior::Investigate { path, path_index, give_up_timer } => {
            *give_up_timer -= dt;
            if let Some(&cell) = path.get(*path_index) {
                let (wx, wz) = cell_to_world(cell.0, cell.1, MAZE_CELL_SIZE);
                desired_dir = Vec3::new(wx - bot.position.0, 0.0, wz - bot.position.2);
                if desired_dir.length() < PATH_ARRIVAL_THRESHOLD {
                    *path_index += 1;
                } else {
                    desired_dir = desired_dir.normalize();
                }
            } else {
                *give_up_timer = 0.0;
            }
        }

        // HoldAngle: stay put, face the specified direction
        BotBehavior::HoldAngle { hold_timer, .. } => {
            *hold_timer += dt;
            // no movement — desired_dir stays ZERO
        }
    }

    // save old position for collision rollback in case we walk into a wall
    let old_pos = bot.position;

    let (evt_jump_start, evt_jump_land) = simulate_bot_physics(
        &mut state.movement,
        &mut bot.position,
        desired_dir,
        speed,
        want_jump,
        dt,
        physics
    );

    // axis-separated wall + prop sliding with radius-based corner checks
    if let Some((cells, width, height, props)) = maze_cells {
        // Direct cell lookup — no fallback search so walls are always detected.
        let cell_is_wall = |x: f32, z: f32| -> bool {
            let cx = (x / MAZE_CELL_SIZE).floor() as i16;
            let cz = (z / MAZE_CELL_SIZE).floor() as i16;
            if cx < 0 || cz < 0 || cx >= width as i16 || cz >= height as i16 {
                return true;
            }
            cells[(cz as usize) * (width as usize) + (cx as usize)] == 0
        };

        // Check all 4 corners of the bot's footprint so body width is respected.
        const BOT_RADIUS: f32 = 0.28;

        let hits_prop = |x: f32, z: f32| -> bool {
            for prop in props {
                let px = prop.x as f32 * MAZE_CELL_SIZE;
                let pz = prop.y as f32 * MAZE_CELL_SIZE;
                if x + BOT_RADIUS > px && x - BOT_RADIUS < px + MAZE_CELL_SIZE
                    && z + BOT_RADIUS > pz && z - BOT_RADIUS < pz + MAZE_CELL_SIZE
                {
                    return true;
                }
            }
            false
        };

        let hits_wall = |x: f32, z: f32| -> bool {
            cell_is_wall(x - BOT_RADIUS, z - BOT_RADIUS)
                || cell_is_wall(x + BOT_RADIUS, z - BOT_RADIUS)
                || cell_is_wall(x - BOT_RADIUS, z + BOT_RADIUS)
                || cell_is_wall(x + BOT_RADIUS, z + BOT_RADIUS)
                || hits_prop(x, z)
        };

        if hits_wall(bot.position.0, bot.position.2) {
            // revert X, keep Z — try sliding along Z axis
            bot.position.0 = old_pos.0;
            state.movement.velocity.x = 0.0;

            // if Z alone also puts us in a wall/prop it's a corner — full stop
            if hits_wall(bot.position.0, bot.position.2) {
                bot.position.2 = old_pos.2;
                state.movement.velocity.z = 0.0;
            }
        }
    }

    /*
        Anything below runs regardless of bot behavior
        Animations, facing direction, and other movement-related state is updated here based on the final desired_dir 
        after behavior logic runs
    */

    // update replicated movement flags for client animation state
    bot.is_airborne = !state.movement.is_grounded;
    // Kept until the next state broadcast sends them (see Server::tick).
    bot.evt_jump_start |= evt_jump_start;
    bot.evt_landed |= evt_jump_land;
    bot.is_sprinting = false;
    bot.is_crouched = false;
    bot.is_prone = false;
    bot.is_sliding = false;
    bot.is_ads = state.target_id.is_some();

    // Face movement direction by default.
    if desired_dir.length_squared() > 0.0001 {
        bot.yaw = desired_dir.z.atan2(desired_dir.x);
        state.idle_look_timer = 0.0;
    } else if state.target_id.is_none() {
        // Idle look-around: slowly sweep yaw when not moving and no target
        state.idle_look_timer += dt;
        let sweep_speed = 0.4;
        bot.yaw += (state.idle_look_timer * sweep_speed).sin() * 0.3 * dt;
        bot.pitch = (state.idle_look_timer * 0.25).sin() * 0.08;
    }

    // HoldAngle overrides yaw to face the strategic direction
    if let BotBehavior::HoldAngle { face_direction, .. } = &state.behavior {
        bot.yaw = face_direction.1.atan2(face_direction.0);
    }

    // stuck detection
    // track consecutive time without meaningful movement
    let dominated_by_movement = !matches!(
        state.behavior,
        BotBehavior::Static | BotBehavior::HoldAngle { .. }
    );
    let dx = bot.position.0 - old_pos.0;
    let dz = bot.position.2 - old_pos.2;
    let moved = (dx * dx + dz * dz).sqrt();
    if dominated_by_movement && moved < 0.01 {
        state.stuck_timer += dt;
    } else {
        state.stuck_timer = 0.0;
    }

    // Unstick: if stuck for >1s, teleport to nearest adjacent open cell
    if state.stuck_timer > 2.5 {
        if let Some((cells, width, height, _props)) = maze_cells {
            if let Some(cell) = world_to_cell(
                bot.position.0, bot.position.2,
                MAZE_CELL_SIZE, cells, width, height,
            ) {
                let neighbors: [(i8, i8); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
                for &(ndx, ndz) in &neighbors {
                    let nx = cell.0 as i16 + ndx as i16;
                    let nz = cell.1 as i16 + ndz as i16;
                    if nx >= 0 && nz >= 0
                        && (nx as usize) < width as usize
                        && (nz as usize) < height as usize
                    {
                        let ni = (nz as usize) * (width as usize) + (nx as usize);
                        if cells[ni] != 0 {
                            let (wx, wz) = cell_to_world(nx as u8, nz as u8, MAZE_CELL_SIZE);
                            bot.position.0 = wx;
                            bot.position.2 = wz;
                            state.stuck_timer = 0.0;
                            state.movement.velocity = Vec3::ZERO;
                            break;
                        }
                    }
                }
            }
        }
    }
}
