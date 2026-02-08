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
    pub stamina: f32,
    pub height: f32, // eye height
}


impl MovementState {
    pub fn new() -> Self {
        Self {
            velocity: Vec3::ZERO,
            is_grounded: true,
            is_crouching: false,
            is_sprinting: false,
            stamina: 100.0,
            height: 1.6, // default eye height
        }
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
    pub acceleration: f32,
    pub friction: f32,
    pub gravity: f32,
    pub jump_strength: f32,
    pub stamina_drain: f32, // per second
    pub stamina_recover: f32, // per second
}

impl MovementConfig {
    pub fn new() -> Self {
        Self {
            walk_speed: 5.0,
            sprint_speed: 8.0,
            crouch_speed: 2.5,
            acceleration: 10.0,
            friction: 8.0,
            gravity: 20.0,
            jump_strength: 4.0,
            stamina_drain: 35.0,
            stamina_recover: 15.0,
        }
    }
}