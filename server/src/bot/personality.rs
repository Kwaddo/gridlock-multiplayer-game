/*
    This module defines different bot personalites and their difficulties, these will be the bots you can call in the game
    they will have different names so you can associate them with their difficulty, and they will have different strategies for playing the game, which will be defined in the bot module
*/
#[derive(Clone, Copy)]
pub struct BotPersonality {
    pub name: &'static str, // static because we want to have a predefined set of personalities, and we don't want to allocate memory for the name of the personality every time we create a bot
    pub archetype: BotArchetype,
    pub weapon: &'static str, // static because we want to have a predefined set of weapons, and we don't want to allocate memory for the weapon of the personality every time we create a bot
    pub range_min: f32,
    pub range_max: f32,
    pub aggression: f32, // 0.0 to 1.0, where 0.0 is passive and 1.0 is aggressive
    pub flank_weight: f32, // 0.0 to 1.0, where 0.0 is no flanking and 1.0 is always flanking
    pub hold_angle_weight: f32, // 0.0 to 1.0, where 0.0 is no holding angles and 1.0 is always holding angles (for campers / snipers)
    pub push_weight: f32, // 0.0 to 1.0, where 0.0 is no pushing and 1.0 is always pushing (for rushers)
    pub retreat_health_threshold: f32, // 0.0 to 1.0, where 0.0 is never retreating and 1.0 is always retreating (for all archetypes, but especially for campers)
    pub peek_time: f32, // time in seconds that the bot will peek for before going back to cover, this is especially important for snipers and campers
    pub reposition_interval: f32, // time in seconds that the bot will wait before repositioning, this is especially important for campers and snipers
    pub noise_investigation_weight: f32, // 0.0 to 1.0, where 0.0 is no investigating noises and 1.0 is always investigating noises (for all archetypes, but especially for all-rounders and flankers)
}

#[derive(Clone, Copy)]
pub enum BotDifficulty {
    Recruit,
    Easy,
    Normal,
    Hard,
    Nightmare,
}

#[derive(Clone, Copy)]
pub enum BotArchetype {
    Rusher,
    Sniper,
    Flanker,
    KnifeSneaker,
    AllRounder,
    Camper,
}

pub struct BotRuntimeConfig {
    reaction_time_mult: f32, // multiplier for the bot's reaction time, where 1.0 is the base reaction time defined in the BotState, and higher values make the bot react slower
    aim_offset_mult: f32, // multiplier for the bot's aim offset, where 1.0 is the base aim offset defined in the BotShootingConfig, and higher values make the bot less accurate
    fov_degrees: f32, // field of view in degrees that the bot can see, this is especially important for snipers who should have a narrower field of view than rushers
    repath_interval_mult: f32, // multiplier for the bot's pathfinding repath interval, where 1.0 is the base repath interval defined in the BotState, and higher values make the bot recalculate its path less frequently
    target_stickiness: f32, // 0.0 to 1.0, where 0.0 is no stickiness (the bot will switch targets as soon as a better one is available) and 1.0 is perfect stickiness (the bot will never switch targets unless its current target is dead)
    burst_discipline: f32, // 0.0 to 1.0, where 0.0 is no burst discipline (the bot will always shoot as soon as it can) and 1.0 is perfect burst discipline (the bot will always wait for the optimal time to shoot based on its weapon's characteristics)
    wall_read_memory_time: f32, // time in seconds that the bot will remember an enemy's position after losing line of sight, this is especially important for campers and snipers who rely on holding angles and pre-aiming
}
pub fn compute_runtime(_personality: &BotPersonality, difficulty: BotDifficulty) -> BotRuntimeConfig {
    match difficulty {
        BotDifficulty::Recruit => BotRuntimeConfig {
            reaction_time_mult: 1.8,
            aim_offset_mult: 2.2,
            fov_degrees: 90.0,
            repath_interval_mult: 1.8,
            target_stickiness: 0.25,
            burst_discipline: 0.20,
            wall_read_memory_time: 0.8,
        },
        BotDifficulty::Easy => BotRuntimeConfig {
            reaction_time_mult: 1.3,
            aim_offset_mult: 1.6,
            fov_degrees: 120.0,
            repath_interval_mult: 1.4,
            target_stickiness: 0.45,
            burst_discipline: 0.40,
            wall_read_memory_time: 1.4,
        },
        BotDifficulty::Normal => BotRuntimeConfig {
            reaction_time_mult: 1.0,
            aim_offset_mult: 1.0,
            fov_degrees: 150.0,
            repath_interval_mult: 1.0,
            target_stickiness: 0.65,
            burst_discipline: 0.60,
            wall_read_memory_time: 2.2,
        },
        BotDifficulty::Hard => BotRuntimeConfig {
            reaction_time_mult: 0.75,
            aim_offset_mult: 0.7,
            fov_degrees: 180.0,
            repath_interval_mult: 0.75,
            target_stickiness: 0.80,
            burst_discipline: 0.78,
            wall_read_memory_time: 3.0,
        },
        BotDifficulty::Nightmare => BotRuntimeConfig {
            reaction_time_mult: 0.45,
            aim_offset_mult: 0.4,
            fov_degrees: 270.0,
            repath_interval_mult: 0.5,
            target_stickiness: 0.92,
            burst_discipline: 0.92,
            wall_read_memory_time: 4.0,
        },
    }
}
