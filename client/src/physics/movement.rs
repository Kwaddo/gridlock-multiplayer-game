use macroquad::prelude::*;

/*
   MOVEMENT STATE
    This struct holds the current movement state of the player, such as velocity,
    whether they are on the ground, crouching, sprinting, etc.
    It is updated based on player input and physics calculations.
*/
#[derive(Debug, PartialEq)]
pub struct MovementState {
    pub velocity: Vec3,
    pub is_grounded: bool,
    pub is_crouching: bool,
    pub is_sprinting: bool,
    pub is_slow_walking: bool,
    pub is_prone: bool,
    pub prone_transition_timer: f32,
    pub is_sliding: bool,
    pub slide_timer: f32,
    pub slide_cooldown_timer: f32,
    pub slide_direction: Vec3,
    pub slide_speed_current: f32,
    pub stamina: f32,
    pub height: f32, // eye height
    pub is_stamina_depleted: bool,
}

impl MovementState {
    pub fn new() -> Self {
        Self {
            velocity: Vec3::ZERO,
            is_grounded: true,
            is_crouching: false,
            is_sprinting: false,
            is_slow_walking: false,
            is_prone: false,
            prone_transition_timer: 0.0,
            is_sliding: false,
            slide_timer: 0.0,
            slide_cooldown_timer: 0.0,
            slide_direction: Vec3::ZERO,
            slide_speed_current: 0.0,
            stamina: 100.0,
            height: 1.6, // default eye height
            is_stamina_depleted: false,
        }
    }
    pub fn is_moving(&self) -> bool {
        // true if player has significant horizontal velocity
        let h_speed_sq = self.velocity.x * self.velocity.x + self.velocity.z * self.velocity.z;
        h_speed_sq > 0.5
    }
}

/*
    MOVEMENT CONFIG
     This struct holds the configuration values for movement, such as speeds,
     acceleration, gravity, jump strength, and stamina drain/recover rates.
     These values can be tweaked to change the feel of the movement.
*/
pub struct MovementConfig {
    pub walk_speed: f32,
    pub sprint_speed: f32,
    pub crouch_speed: f32,
    pub slow_walk_speed: f32,
    pub prone_speed: f32,
    pub prone_transition_time: f32,
    pub prone_height: f32,
    pub slide_initial_speed: f32,
    pub slide_duration: f32,
    pub slide_stamina_cost: f32,
    pub slide_cooldown: f32,
    pub slide_friction: f32,
    pub slide_height: f32,
    pub slide_ads_time_mult: f32,
    pub acceleration: f32,
    pub friction: f32,
    pub gravity: f32,
    pub jump_strength: f32,
    pub stamina_drain: f32,   // per second
    pub stamina_recover: f32, // per second
    pub stamina_depleted_threshold: f32,
    pub stamina_depleted_recover: f32,
    pub stamina_depleted_speed_mult: f32,
}

impl MovementConfig {
    pub fn new() -> Self {
        Self {
            walk_speed: 5.0,
            sprint_speed: 8.0,
            crouch_speed: 2.5,
            slow_walk_speed: 3.5,
            prone_speed: 1.5,
            prone_transition_time: 0.8,
            prone_height: 0.3,
            slide_initial_speed: 10.0,
            slide_duration: 0.8,
            slide_stamina_cost: 30.0,
            slide_cooldown: 2.0,
            slide_friction: 12.0,
            slide_height: 0.4,
            slide_ads_time_mult: 3.0,
            acceleration: 10.0,
            friction: 8.0,
            gravity: 20.0,
            jump_strength: 6.0,
            stamina_drain: 35.0,
            stamina_recover: 15.0,
            stamina_depleted_threshold: 10.0,
            stamina_depleted_recover: 7.5,
            stamina_depleted_speed_mult: 0.5,
        }
    }
}
