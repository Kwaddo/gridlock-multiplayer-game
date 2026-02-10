use crate::core::GameConfig;
use crate::physics::movement::MovementState;
use macroquad::prelude::*;
use crate::weapons::*;

// process mouse input and update camera orientation


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

// process keyboard input and return movement direction
pub fn process_movement_keys(yaw: f32, config: &GameConfig) -> Vec3 {
    let right = vec3(-yaw.sin(), 0.0, yaw.cos());
    let forward = vec3(yaw.cos(), 0.0, yaw.sin());

    let mut move_dir = Vec3::ZERO;
    if is_key_down(config.key_forward) || is_key_down(KeyCode::Up) {
        move_dir += forward;
    }
    if is_key_down(config.key_backward) || is_key_down(KeyCode::Down) {
        move_dir -= forward;
    }
    if is_key_down(config.key_left) || is_key_down(KeyCode::Left) {
        move_dir -= right;
    }
    if is_key_down(config.key_right) || is_key_down(KeyCode::Right) {
        move_dir += right;
    }

    if move_dir != Vec3::ZERO {
        move_dir.normalize()
    } else {
        Vec3::ZERO
    }
}

// process sprint input
pub fn process_sprint(config: &GameConfig, movement_state: &MovementState, move_dir: Vec3) -> bool {
    is_key_down(config.key_sprint)
        && movement_state.is_grounded
        && move_dir != Vec3::ZERO
        && movement_state.stamina > 0.0
}

// process crouch input (handles both toggle and hold)
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

// process jump input
pub fn process_jump(config: &GameConfig, is_grounded: bool, weapons: &mut WeaponInventory) -> bool {
    let weapon = weapons.active_mut();
    if weapon.is_aiming && is_key_down(config.key_jump) && is_grounded {
        weapon.is_aiming = false;
        weapon.ads_progress = 0.0;
        weapon.jump_cancelled_ads = true; // prevent process_weapon_input from re-enabling ADS this frame
    }
    is_key_down(config.key_jump) && is_grounded 
}

// process weapon input and return the desired weapon action
pub fn process_weapon_input(
    inventory: &mut WeaponInventory,
    config: &GameConfig,
) {
    // weapon switching (number keys)
    if is_key_pressed(KeyCode::Key1) { inventory.switch_to(0); }
    if is_key_pressed(KeyCode::Key2) { inventory.switch_to(1); }
    if is_key_pressed(KeyCode::Key3) { inventory.switch_to(2); }
    if is_key_pressed(KeyCode::Key4) { inventory.switch_to(3); }
    if is_key_pressed(KeyCode::Key5) { inventory.switch_to(4); }

    // mouse wheel switching
    let (_, wheel_y) = mouse_wheel();
       if wheel_y > 0.0 {
        inventory.scroll(-1);
    } else if wheel_y < 0.0 {
        inventory.scroll(1);
    }

    // reload
      if is_key_pressed(config.key_reload) {
        inventory.active_mut().start_reload();
    }

   // ADS (only set if grounded — jump cancels ADS elsewhere)
    let weapon = inventory.active_mut();
    if !weapon.jump_cancelled_ads {
        weapon.is_aiming = config.bind_aim.is_down();
    }
    // clear the flag once aim button is released so player can re-aim
    if !config.bind_aim.is_down() {
        weapon.jump_cancelled_ads = false;
    }
}

