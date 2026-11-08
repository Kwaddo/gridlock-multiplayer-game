//! Difficulty multipliers and the runtime stats they produce for a personality.

use super::{BotDifficulty, BotPersonality};

pub struct DifficultyMods {
    pub reaction_time_mult: f32, // multiplier for the bot's reaction time, where 1.0 is the base reaction time defined in the BotState, and higher values make the bot react slower
    pub aim_offset_mult: f32, // multiplier for the bot's aim offset, where 1.0 is the base aim offset defined in the BotShootingConfig, and higher values make the bot less accurate
    pub fov_degrees: f32, // field of view in degrees that the bot can see, this is especially important for snipers who should have a narrower field of view than rushers
    pub repath_interval_mult: f32, // multiplier for the bot's pathfinding repath interval, where 1.0 is the base repath interval defined in the BotState, and higher values make the bot recalculate its path less frequently
    pub target_stickiness: f32, // 0.0 to 1.0, where 0.0 is no stickiness (the bot will switch targets as soon as a better one is available) and 1.0 is perfect stickiness (the bot will never switch targets unless its current target is dead)
    pub burst_discipline: f32, // 0.0 to 1.0, where 0.0 is no burst discipline (the bot will always shoot as soon as it can) and 1.0 is perfect burst discipline (the bot will always wait for the optimal time to shoot based on its weapon's characteristics)
    pub wall_read_memory_time: f32, // time in seconds that the bot will remember an enemy's position after losing line of sight, this is especially important for campers and snipers who rely on holding angles and pre-aiming
    pub turn_rate_mult: f32, // >1.0 = slower, <1.0 = faster, this is a multiplier for the bot's turn rate, where 1.0 is the base turn rate defined in the BotState, and higher values make the bot turn slower
}

pub fn difficulty_mods(d: BotDifficulty) -> DifficultyMods {
    match d {
        BotDifficulty::Recruit =>
            DifficultyMods {
                reaction_time_mult: 1.8,
                aim_offset_mult: 2.2,
                fov_degrees: 90.0,
                repath_interval_mult: 1.8,
                target_stickiness: 0.25,
                burst_discipline: 0.2,
                wall_read_memory_time: 0.8,
                turn_rate_mult: 1.8,
            },
        BotDifficulty::Easy =>
            DifficultyMods {
                reaction_time_mult: 1.3,
                aim_offset_mult: 1.6,
                fov_degrees: 120.0,
                repath_interval_mult: 1.4,
                target_stickiness: 0.45,
                burst_discipline: 0.4,
                wall_read_memory_time: 1.4,
                turn_rate_mult: 1.4,
            },
        BotDifficulty::Normal =>
            DifficultyMods {
                reaction_time_mult: 1.0,
                aim_offset_mult: 1.0,
                fov_degrees: 150.0,
                repath_interval_mult: 1.0,
                target_stickiness: 0.65,
                burst_discipline: 0.6,
                wall_read_memory_time: 2.2,
                turn_rate_mult: 1.0,
            },
        BotDifficulty::Hard =>
            DifficultyMods {
                reaction_time_mult: 0.75,
                aim_offset_mult: 0.7,
                fov_degrees: 180.0,
                repath_interval_mult: 0.75,
                target_stickiness: 0.8,
                burst_discipline: 0.78,
                wall_read_memory_time: 3.0,
                turn_rate_mult: 0.75,
            },
        BotDifficulty::Nightmare =>
            DifficultyMods {
                reaction_time_mult: 0.45,
                aim_offset_mult: 0.4,
                fov_degrees: 270.0,
                repath_interval_mult: 0.5,
                target_stickiness: 0.92,
                burst_discipline: 0.92,
                wall_read_memory_time: 4.0,
                turn_rate_mult: 0.45,
            },
    }
}

pub fn compute_runtime(p: &BotPersonality, d: BotDifficulty) -> DifficultyMods {
    let m = difficulty_mods(d);
    DifficultyMods {
        reaction_time_mult: p.base_reaction_time * m.reaction_time_mult,

        aim_offset_mult: p.base_aim_offset * m.aim_offset_mult,

        fov_degrees: p.base_fov_degrees.max(m.fov_degrees),

        repath_interval_mult: p.base_repath_interval * m.repath_interval_mult,

        target_stickiness: m.target_stickiness,
        burst_discipline: m.burst_discipline,
        wall_read_memory_time: m.wall_read_memory_time,
        turn_rate_mult: p.base_turn_rate * m.turn_rate_mult,
    }
}
