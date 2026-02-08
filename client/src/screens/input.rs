use crate::core::GameConfig;
use crate::physics::movement::MovementState;
use macroquad::prelude::*;

/// Process mouse input and update camera orientation
pub fn process_mouse(
    last_mouse_pos: &mut (f32, f32),
    yaw: &mut f32,
    pitch: &mut f32,
    sensitivity: f32,
) {
    let mouse_pos = mouse_position();
    let delta = (
        mouse_pos.0 - last_mouse_pos.0,
        mouse_pos.1 - last_mouse_pos.1,
    );
    *last_mouse_pos = mouse_pos;

    *yaw += delta.0 * sensitivity;
    *pitch -= delta.1 * sensitivity;
    *pitch = pitch.clamp(-1.55, 1.55);
}

/// Process keyboard input and return movement direction
pub fn process_movement_keys(yaw: f32, config: &GameConfig) -> Vec3 {
    let right = vec3(-yaw.sin(), 0.0, yaw.cos());
    let forward = vec3(yaw.cos(), 0.0, yaw.sin());

    let mut move_dir = Vec3::ZERO;
    if is_key_down(config.key_forward) {
        move_dir += forward;
    }
    if is_key_down(config.key_backward) {
        move_dir -= forward;
    }
    if is_key_down(config.key_left) {
        move_dir -= right;
    }
    if is_key_down(config.key_right) {
        move_dir += right;
    }

    if move_dir != Vec3::ZERO {
        move_dir.normalize()
    } else {
        Vec3::ZERO
    }
}

/// Process sprint input
pub fn process_sprint(config: &GameConfig, movement_state: &MovementState, move_dir: Vec3) -> bool {
    is_key_down(config.key_sprint)
        && movement_state.is_grounded
        && move_dir != Vec3::ZERO
        && movement_state.stamina > 0.0
}

/// Process crouch input (handles both toggle and hold)
pub fn process_crouch(config: &GameConfig, is_crouching: bool) -> bool {
    if config.crouch_toggle {
        if is_key_pressed(config.key_crouch) {
            !is_crouching
        } else {
            is_crouching
        }
    } else {
        is_key_down(config.key_crouch)
    }
}

/// Process jump input
pub fn process_jump(config: &GameConfig, is_grounded: bool) -> bool {
    is_key_down(config.key_jump) && is_grounded
}
