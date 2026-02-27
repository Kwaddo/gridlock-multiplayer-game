use glam::Vec3;

use crate::math::{from_vec3, to_vec3};
use crate::maze::pathfinding::{cell_to_world, world_to_cell};

#[derive(Debug, Clone, Copy)]
pub struct BotMovementState {
    pub velocity: Vec3,
    pub is_grounded: bool,
    pub height: f32,
}

pub struct PhysicsConfig {
    pub walk_speed: f32,
    pub sprint_speed: f32,
    pub crouch_speed: f32,
    pub gravity: f32,
    pub acceleration: f32,
    pub friction: f32,
    pub jump_strength: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            walk_speed: 5.0,
            sprint_speed: 8.0,
            crouch_speed: 2.5,
            gravity: 20.0,
            acceleration: 10.0,
            friction: 8.0,
            jump_strength: 6.0,
        }
    }
}

pub fn simulate_bot_physics(
    state: &mut BotMovementState,
    position: &mut (f32, f32, f32),
    desired_direction: Vec3,
    speed: f32,
    want_jump: bool,
    dt: f32,
    config: &PhysicsConfig,
) -> (bool, bool) {
    if dt <= 0.0 {
        return (false, false);
    }

    let mut evt_jump_start = false;
    let mut evt_landed = false;

    let mut input = Vec3::new(desired_direction.x, 0.0, desired_direction.z);
    if input.length_squared() > 1e-6 {
        input = input.normalize();
    } else {
        input = Vec3::ZERO;
    }

    let was_grounded = state.is_grounded;

    if want_jump && state.is_grounded {
        state.velocity.y = config.jump_strength;
        state.is_grounded = false;
        evt_jump_start = true;
    }

    state.velocity.y -= config.gravity * dt;

    let mut horiz_vel = Vec3::new(state.velocity.x, 0.0, state.velocity.z);
    if input != Vec3::ZERO {
        let target = input * speed;
        let delta = target - horiz_vel;
        let max_step = config.acceleration * dt;

        if delta.length_squared() > max_step * max_step {
            horiz_vel += delta.normalize() * max_step;
        } else {
            horiz_vel = target;
        }
    } else if state.is_grounded {
        let horiz_speed = horiz_vel.length();
        if horiz_speed > 1e-6 {
            let drop = config.friction * dt;
            let new_speed = (horiz_speed - drop).max(0.0);
            if new_speed == 0.0 {
                horiz_vel = Vec3::ZERO;
            } else {
                horiz_vel *= new_speed / horiz_speed;
            }
        }
    }

    state.velocity.x = horiz_vel.x;
    state.velocity.z = horiz_vel.z;

    let mut pos = to_vec3(*position);
    pos += state.velocity * dt;

    if pos.y <= 0.0 {
        pos.y = 0.0;
        if state.velocity.y < 0.0 {
            state.velocity.y = 0.0;
        }
        state.is_grounded = true;
    } else {
        state.is_grounded = false;
    }

    if !was_grounded && state.is_grounded {
        evt_landed = true;
    }

    *position = from_vec3(pos);
    (evt_jump_start, evt_landed)
}
