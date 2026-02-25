use crate::physics::{ BotMovementState, PhysicsConfig, simulate_bot_physics };
use crate::bot::shooting::{BotShootingConfig, BotShootingState};
use crate::player::{ Player, Team };
use glam::Vec3;

pub struct BotState {
    pub movement: BotMovementState,
    pub behavior: BotBehavior,
    pub can_shoot: bool,
    pub shooting: BotShootingState,
    pub shooting_config: BotShootingConfig,
    pub target_id: Option<u32>,
    pub reaction_timer: f32,
}

#[derive(Clone)]
pub enum BotBehavior {
    Static,
    Patrol {
        waypoints: Vec<(f32, f32, f32)>,
        current: usize,
        arrival_threshold: f32,
    },
    PatrolJump {
        waypoints: Vec<(f32, f32, f32)>,
        current: usize,
        jump_interval: f32,
        jump_timer: f32,
        arrival_threshold: f32,
    },
}

pub fn tick_bot(bot: &mut Player, state: &mut BotState, dt: f32, physics: &PhysicsConfig) {
    let mut desired_dir = Vec3::ZERO; // movement we intend to apply every tick
    let mut want_jump = false; // whether we want to jump this tick
    let speed = physics.walk_speed;

    match &mut state.behavior {
        BotBehavior::Static => {}

        BotBehavior::Patrol { waypoints, current, arrival_threshold } => {
            if !waypoints.is_empty() {
                let waypoint = waypoints[*current];
                desired_dir = Vec3::new(waypoint.0 - bot.position.0, 0.0, waypoint.2 - bot.position.2);

                if desired_dir.length() < *arrival_threshold {
                    // if we're close enough to the waypoint, switch to the next one
                    *current = (*current + 1) % waypoints.len();
                } else {
                    desired_dir = desired_dir.normalize();
                }
            }
        }
        BotBehavior::PatrolJump {
            waypoints,
            current,
            jump_interval,
            jump_timer,
            arrival_threshold,
        } => {
            if !waypoints.is_empty() {
                let waypoint = waypoints[*current];
                desired_dir = Vec3::new(waypoint.0 - bot.position.0, 0.0, waypoint.2 - bot.position.2);

                if desired_dir.length() < *arrival_threshold {
                    *current = (*current + 1) % waypoints.len();
                } else {
                    desired_dir = desired_dir.normalize();
                }
            }

            *jump_timer -= dt;
            if *jump_timer <= 0.0 {
                *jump_timer = *jump_interval;
                if state.movement.is_grounded {
                    want_jump = true;
                }
            }
        }
    }
    let (evt_jump_start, evt_jump_land) = simulate_bot_physics(
        &mut state.movement,
        &mut bot.position,
        desired_dir,
        speed,
        want_jump,
        dt,
        physics
    );

    // update replicated movement flags for client animation state
    bot.is_airborne = !state.movement.is_grounded;
    bot.evt_jump_start = evt_jump_start;
    bot.evt_landed = evt_jump_land;
    bot.is_sprinting = false;
    bot.is_crouched = false;
    bot.is_prone = false;
    bot.is_sliding = false;
    bot.is_ads = state.target_id.is_some();

    // Face movement direction by default.
    if desired_dir.length_squared() > 0.0001 {
        bot.yaw = desired_dir.z.atan2(desired_dir.x);
    }
}
