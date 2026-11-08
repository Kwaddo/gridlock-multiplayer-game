/*
    This module defines different bot personalites and their difficulties, these will be the bots you can call in the game
    they will have different names so you can associate them with their difficulty, and they will have different strategies for playing the game, which will be defined in the bot module
*/
mod difficulty;
mod roster;

pub use difficulty::{compute_runtime, difficulty_mods};
pub use roster::get_personality;

#[derive(Clone, Copy)]
#[allow(dead_code)]
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

    // Base personality traits
    pub base_reaction_time: f32,
    pub base_aim_offset: f32,
    pub base_repath_interval: f32,
    pub base_fov_degrees: f32, // optional but its how wide the bots eyes are
    pub base_turn_rate:   f32, // how fast the bot can turn to look at targets, in degrees per second, this is especially important for snipers who should have a slower turn rate than rushers, or for difficulties
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BotDifficulty {
    Recruit,
    Easy,
    Normal,
    Hard,
    Nightmare,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BotArchetype {
    Rusher,
    Sniper,
    Flanker,
    AllRounder,
    Camper,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BotPersonalityId {
    Heater, // fast-entry duelist, high aggression, close-mid range
    Stinger, // poke/harass bot, quick peeks, medium aggression
    Rex, // frontline enforcer, pushes with pressure
    Merlin, // holds angles and repositions smartly
    Viper, // flanker/ambusher,
    Phantom, // stealthy lurker, low noise + surprise engages
    Nexto, // high-skill all-rounder,
    Bandit, // chaotic rusher, high push, lower discipline
    Slater, // methodical fragger, strong fundamentals
    Storm, // tempo disruptor, constant pressure and movement
    Ghost, // patient camper/anchor, punishes over-peeks
    Sniper, // long-range specialist, angle control first
    Bulwark, // Anchor / tank
    Wraith, // flanker
    Overwatch, // angle-holding specialist
}

#[allow(dead_code)]
impl BotPersonalityId {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(BotPersonalityId::Heater),
            1 => Some(BotPersonalityId::Stinger),
            2 => Some(BotPersonalityId::Rex),
            3 => Some(BotPersonalityId::Merlin),
            4 => Some(BotPersonalityId::Viper),
            5 => Some(BotPersonalityId::Phantom),
            6 => Some(BotPersonalityId::Nexto),
            7 => Some(BotPersonalityId::Bandit),
            8 => Some(BotPersonalityId::Slater),
            9 => Some(BotPersonalityId::Storm),
            10 => Some(BotPersonalityId::Ghost),
            11 => Some(BotPersonalityId::Sniper),
            12 => Some(BotPersonalityId::Bulwark),
            13 => Some(BotPersonalityId::Wraith),
            14 => Some(BotPersonalityId::Overwatch),
            _ => None,
        }
    }
    pub fn as_u8(&self) -> u8 {
        match self {
            BotPersonalityId::Heater => 0,
            BotPersonalityId::Stinger => 1,
            BotPersonalityId::Rex => 2,
            BotPersonalityId::Merlin => 3,
            BotPersonalityId::Viper => 4,
            BotPersonalityId::Phantom => 5,
            BotPersonalityId::Nexto => 6,
            BotPersonalityId::Bandit => 7,
            BotPersonalityId::Slater => 8,
            BotPersonalityId::Storm => 9,
            BotPersonalityId::Ghost => 10,
            BotPersonalityId::Sniper => 11,
            BotPersonalityId::Bulwark => 12,
            BotPersonalityId::Wraith => 13,
            BotPersonalityId::Overwatch => 14,
        }
    }
}
