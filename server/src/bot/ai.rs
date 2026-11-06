use crate::physics::{ BotMovementState, PhysicsConfig, simulate_bot_physics };
use crate::bot::shooting::{ BotShootingConfig, BotShootingState };
use crate::player::Player;
use crate::bot::personality::{ BotArchetype, BotDifficulty, BotPersonality, BotPersonalityId };
use crate::maze::pathfinding::{ cell_to_world, world_to_cell };
use crate::server::game::MAZE_CELL_SIZE;
use glam::Vec3;
use rand::Rng;

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
    pub tactics: BotTactics,
}

/// Short-lived combat state: who shot us, which way we strafe, whom we just lost sight of.
#[derive(Clone, Debug, Default)]
pub struct BotTactics {
    /// Someone we can't see shot us: (attacker id, where they were, seconds left to care).
    pub alert: Option<(u32, (f32, f32, f32), f32)>,
    /// +1 or -1: which side we strafe to while fighting.
    pub strafe_sign: f32,
    /// Seconds until we pick a strafe side again.
    pub strafe_timer: f32,
    /// The target we last lost sight of and how long ago; seeing them again
    /// soon after needs no fresh reaction time.
    pub lost_target: Option<(u32, f32)>,
}

/// Seconds a bot keeps turning toward / hunting someone who shot it unseen.
pub const ALERT_SECS: f32 = 3.0;

impl BotState {
    /// We were shot by `attacker_id` standing at `pos`.
    pub fn alert(&mut self, attacker_id: u32, pos: (f32, f32, f32)) {
        if self.target_id == Some(attacker_id) {
            return; // already fighting them
        }
        self.tactics.alert = Some((attacker_id, pos, ALERT_SECS));
        self.last_known_enemy_pos = Some(pos);
    }
}

/// An enemy the bot can see and is fighting.
#[derive(Clone, Copy, Debug)]
pub struct Engagement {
    pub target_pos: (f32, f32, f32),
    pub distance: f32,
}

const PATH_ARRIVAL_THRESHOLD: f32 = 0.5; // distance threshold for considering a bot has arrived at a path node
/// Seconds without moving along a path before the bot plans a new one.
const REPLAN_WHEN_STUCK_SECS: f32 = 0.7;

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
    /// Walk to a spot: then hold an angle there facing `hold_face`, or (None,
    /// roaming) pick something new.
    Reposition {
        path: Vec<(u8, u8)>,
        path_index: usize,
        hold_face: Option<(f32, f32)>,
    },
}

impl BotBehavior {
    /// The A* path this behavior walks, and how far along it is.
    fn path_mut(&mut self) -> Option<(&[(u8, u8)], &mut usize)> {
        match self {
            BotBehavior::Hunt { path, path_idx, .. } => Some((path, path_idx)),
            BotBehavior::Flank { path, path_index, .. }
            | BotBehavior::Retreat { path, path_index }
            | BotBehavior::Investigate { path, path_index, .. }
            | BotBehavior::Reposition { path, path_index, .. } => Some((path, path_index)),
            _ => None,
        }
    }
}

/// Where to go next along a path.
struct PathStep {
    dir: Vec3,
    /// Distance to where the path next changes direction (or ends): the bot
    /// has to have slowed down by then or momentum carries it into a wall.
    to_turn: f32,
}

/// The next step along `path`, advancing past cells reached. `None` once the
/// path is done, or when the bot is no longer next to it (pushed off, or
/// carried past a corner): the path is then dropped (`idx` set to its end) so
/// the next decision plans a new one from where the bot is.
fn follow_path(path: &[(u8, u8)], idx: &mut usize, pos: (f32, f32, f32)) -> Option<PathStep> {
    let next = loop {
        let &cell = path.get(*idx)?;
        let (wx, wz) = cell_to_world(cell.0, cell.1, MAZE_CELL_SIZE);
        let to_cell = Vec3::new(wx - pos.0, 0.0, wz - pos.2);
        if to_cell.length() >= PATH_ARRIVAL_THRESHOLD {
            break (cell, to_cell);
        }
        *idx += 1;
    };
    let (cell, to_cell) = next;
    let here = ((pos.0 / MAZE_CELL_SIZE).floor() as i32, (pos.2 / MAZE_CELL_SIZE).floor() as i32);
    let step = |a: (i32, i32), b: (u8, u8)| (b.0 as i32 - a.0, b.1 as i32 - a.1);
    let (dx, dz) = step(here, cell);
    if dx.abs() + dz.abs() > 1 {
        *idx = path.len();
        return None;
    }

    // run on while the path keeps going the same way
    let mut to_turn = to_cell.length();
    let mut heading = (dx, dz);
    let mut i = *idx;
    while let Some(&after) = path.get(i + 1) {
        let cur = path[i];
        let d = step((cur.0 as i32, cur.1 as i32), after);
        if d != heading {
            break;
        }
        to_turn += MAZE_CELL_SIZE;
        heading = d;
        i += 1;
    }
    Some(PathStep { dir: to_cell.normalize(), to_turn })
}

/// Top speed that still lets a bot slow to a corner-taking pace within `to_turn`.
fn cornering_speed(to_turn: f32, acceleration: f32) -> f32 {
    const TURN_SPEED: f32 = 2.5;
    (2.0 * acceleration * (to_turn - 0.3).max(0.0)).sqrt() + TURN_SPEED
}

pub fn tick_bot(
    bot: &mut Player,
    state: &mut BotState,
    dt: f32,
    physics: &PhysicsConfig,
    maze_cells: Option<(&[u8], u8, u8, &[crate::maze::types::Prop])>,
    engage: Option<Engagement>,
) {
    let mut desired_dir = Vec3::ZERO; // movement we intend to apply every tick
    let want_jump = false; // whether we want to jump this tick
    let mut speed: f32 = physics.walk_speed;
    let mut sprinting = false;
    let mut strafing = false;

    // timers that run whatever the bot does
    if let BotBehavior::Hunt { repath_timer, .. } | BotBehavior::Flank { repath_timer, .. } =
        &mut state.behavior
    {
        *repath_timer -= dt;
    }
    if let BotBehavior::Investigate { give_up_timer, .. } = &mut state.behavior {
        *give_up_timer -= dt;
    }
    if let BotBehavior::HoldAngle { hold_timer, .. } = &mut state.behavior {
        *hold_timer += dt;
    }

    let pos = bot.position;
    let retreating = matches!(state.behavior, BotBehavior::Retreat { .. });
    match (engage, &mut state.behavior) {
        (_, BotBehavior::Static) | (_, BotBehavior::HoldAngle { .. }) => {}

        (_, BotBehavior::Patrol { waypoints, current, arrival_threshold }) => {
            if !waypoints.is_empty() {
                let waypoint = waypoints[*current];
                desired_dir = Vec3::new(waypoint.0 - pos.0, 0.0, waypoint.2 - pos.2);
                if desired_dir.length() < *arrival_threshold {
                    // if we're close enough to the waypoint, switch to the next one
                    *current = (*current + 1) % waypoints.len();
                } else {
                    desired_dir = desired_dir.normalize();
                }
            }
        }

        // In a fight (and not running away): keep to a good range and strafe
        // instead of walking into the enemy.
        (Some(fight), behavior) if !retreating => {
            let to_enemy = Vec3::new(fight.target_pos.0 - pos.0, 0.0, fight.target_pos.2 - pos.2);
            let to_enemy = to_enemy.normalize_or_zero();
            let side = Vec3::new(-to_enemy.z, 0.0, to_enemy.x) * state.tactics.strafe_sign;
            let close_in_from = state.personality.range_max * 0.6;
            if fight.distance > close_in_from {
                // too far to fight well: keep walking the path in
                desired_dir = behavior
                    .path_mut()
                    .and_then(|(path, idx)| follow_path(path, idx, pos))
                    .map_or(to_enemy, |step| step.dir);
            } else if fight.distance < state.personality.range_min {
                desired_dir = (side - to_enemy).normalize_or_zero(); // back off sideways
                strafing = true;
            } else {
                desired_dir = side;
                strafing = true;
            }
            speed = physics.walk_speed * 0.8;
        }

        (_, behavior) => {
            if let Some(step) = behavior.path_mut().and_then(|(path, idx)| follow_path(path, idx, pos)) {
                desired_dir = step.dir;
                // travelling with nobody in sight: run, easing off for corners
                speed = physics
                    .sprint_speed
                    .min(cornering_speed(step.to_turn, physics.acceleration));
                sprinting = speed > physics.walk_speed;
            }
        }
    }

    // pick a strafe side now and then, so strafing doesn't look like a pendulum
    state.tactics.strafe_timer -= dt;
    if state.tactics.strafe_timer <= 0.0 || state.tactics.strafe_sign == 0.0 {
        let mut rng = rand::thread_rng();
        state.tactics.strafe_sign = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
        state.tactics.strafe_timer = rng.gen_range(0.6..1.4);
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
            // Move one axis at a time, keeping whichever is free, so the bot
            // slides along walls (and off corners it clips) either way.
            let (new_x, new_z) = (bot.position.0, bot.position.2);
            bot.position.0 = new_x;
            bot.position.2 = old_pos.2;
            if hits_wall(bot.position.0, bot.position.2) {
                bot.position.0 = old_pos.0;
                state.movement.velocity.x = 0.0;
            }
            bot.position.2 = new_z;
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
    bot.is_sprinting = sprinting;
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
    ) && desired_dir != Vec3::ZERO;
    let dx = bot.position.0 - old_pos.0;
    let dz = bot.position.2 - old_pos.2;
    let moved = (dx * dx + dz * dz).sqrt();
    if strafing {
        // strafed into a wall: go the other way rather than grinding on it
        if moved < 0.01 {
            state.tactics.strafe_sign = -state.tactics.strafe_sign;
            state.tactics.strafe_timer = 0.8;
        }
        state.stuck_timer = 0.0;
    } else if dominated_by_movement && moved < 0.01 {
        state.stuck_timer += dt;
    } else {
        state.stuck_timer = 0.0;
    }

    // Blocked for a moment while walking a path (caught on a corner or a
    // prop): drop the path so the next decision plans a fresh one from here.
    if state.stuck_timer > REPLAN_WHEN_STUCK_SECS {
        if let Some((path, idx)) = state.behavior.path_mut() {
            if *idx < path.len() {
                *idx = path.len();
            }
        }
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
