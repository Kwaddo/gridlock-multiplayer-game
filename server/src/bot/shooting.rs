use crate::server::handlers::other::handle_hit;

pub struct BotShootingConfig {
    pub fire_rate: f32, // shots per second
    pub angular_offset: f32, // max angle offset in degrees for inaccuracy
    pub damage: f32,
    pub range: f32,
    pub reaction_time: f32, // time in seconds before bot starts shooting after seeing target
}

pub struct BotShootingState {
    pub fire_cooldown: f32, // time until next shot can be fired
    pub has_target: bool, // whether bot currently has a valid target in sight
    pub reaction_remaining: f32, // time remaining before bot will start shooting at target
}
