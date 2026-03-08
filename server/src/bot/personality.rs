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

    // Base personality traits
    pub base_reaction_time: f32,
    pub base_aim_offset: f32,
    pub base_repath_interval: f32,
    pub base_fov_degrees: f32, // optional but its how wide the bots eyes are
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

pub struct DifficultyMods {
    pub reaction_time_mult: f32, // multiplier for the bot's reaction time, where 1.0 is the base reaction time defined in the BotState, and higher values make the bot react slower
    pub aim_offset_mult: f32, // multiplier for the bot's aim offset, where 1.0 is the base aim offset defined in the BotShootingConfig, and higher values make the bot less accurate
    pub fov_degrees: f32, // field of view in degrees that the bot can see, this is especially important for snipers who should have a narrower field of view than rushers
    pub repath_interval_mult: f32, // multiplier for the bot's pathfinding repath interval, where 1.0 is the base repath interval defined in the BotState, and higher values make the bot recalculate its path less frequently
    pub target_stickiness: f32, // 0.0 to 1.0, where 0.0 is no stickiness (the bot will switch targets as soon as a better one is available) and 1.0 is perfect stickiness (the bot will never switch targets unless its current target is dead)
    pub burst_discipline: f32, // 0.0 to 1.0, where 0.0 is no burst discipline (the bot will always shoot as soon as it can) and 1.0 is perfect burst discipline (the bot will always wait for the optimal time to shoot based on its weapon's characteristics)
    pub wall_read_memory_time: f32, // time in seconds that the bot will remember an enemy's position after losing line of sight, this is especially important for campers and snipers who rely on holding angles and pre-aiming
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
    }
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

pub fn get_personality(p: BotPersonalityId) -> (BotPersonality, BotDifficulty) {
    use BotArchetype::*;
    use BotDifficulty::*;
    match p {
        BotPersonalityId::Heater =>
            (
                BotPersonality {
                    name: "Heater",
                    archetype: Rusher,
                    weapon: "Benelli",
                    range_min: 6.0,
                    range_max: 24.0,
                    aggression: 0.88,
                    flank_weight: 0.3,
                    hold_angle_weight: 0.1,
                    push_weight: 0.92,
                    retreat_health_threshold: 0.15,
                    peek_time: 0.55,
                    reposition_interval: 1.5,
                    noise_investigation_weight: 0.55,
                    base_reaction_time: 0.28,
                    base_aim_offset: 0.035,
                    base_repath_interval: 0.8,
                    base_fov_degrees: 165.0,
                },
                Hard,
            ),
        BotPersonalityId::Stinger =>
            (
                BotPersonality {
                    name: "Stinger",
                    archetype: AllRounder,
                    weapon: "M4A1",
                    range_min: 10.0,
                    range_max: 32.0,
                    aggression: 0.62,
                    flank_weight: 0.4,
                    hold_angle_weight: 0.4,
                    push_weight: 0.55,
                    retreat_health_threshold: 0.3,
                    peek_time: 0.62,
                    reposition_interval: 1.6,
                    noise_investigation_weight: 0.65,
                    base_reaction_time: 0.36,
                    base_aim_offset: 0.05,
                    base_repath_interval: 0.9,
                    base_fov_degrees: 155.0,
                },
                Normal,
            ),
        BotPersonalityId::Rex =>
            (
                BotPersonality {
                    name: "Rex",
                    archetype: Rusher,
                    weapon: "AK-47",
                    range_min: 8.0,
                    range_max: 28.0,
                    aggression: 0.8,
                    flank_weight: 0.28,
                    hold_angle_weight: 0.22,
                    push_weight: 0.82,
                    retreat_health_threshold: 0.22,
                    peek_time: 0.55,
                    reposition_interval: 1.4,
                    noise_investigation_weight: 0.52,
                    base_reaction_time: 0.3,
                    base_aim_offset: 0.038,
                    base_repath_interval: 0.75,
                    base_fov_degrees: 165.0,
                },
                Hard,
            ),
        BotPersonalityId::Merlin =>
            (
                BotPersonality {
                    name: "Merlin",
                    archetype: Camper,
                    weapon: "SCAR-H",
                    range_min: 16.0,
                    range_max: 45.0,
                    aggression: 0.38,
                    flank_weight: 0.24,
                    hold_angle_weight: 0.86,
                    push_weight: 0.2,
                    retreat_health_threshold: 0.42,
                    peek_time: 0.85,
                    reposition_interval: 2.5,
                    noise_investigation_weight: 0.4,
                    base_reaction_time: 0.4,
                    base_aim_offset: 0.055,
                    base_repath_interval: 1.1,
                    base_fov_degrees: 145.0,
                },
                Normal,
            ),
        BotPersonalityId::Viper =>
            (
                BotPersonality {
                    name: "Viper",
                    archetype: Flanker,
                    weapon: "P90",
                    range_min: 7.0,
                    range_max: 24.0,
                    aggression: 0.7,
                    flank_weight: 0.82,
                    hold_angle_weight: 0.22,
                    push_weight: 0.6,
                    retreat_health_threshold: 0.26,
                    peek_time: 0.52,
                    reposition_interval: 1.0,
                    noise_investigation_weight: 0.76,
                    base_reaction_time: 0.3,
                    base_aim_offset: 0.04,
                    base_repath_interval: 0.65,
                    base_fov_degrees: 175.0,
                },
                Hard,
            ),
        BotPersonalityId::Phantom =>
            (
                BotPersonality {
                    name: "Phantom",
                    archetype: Flanker,
                    weapon: "Suppressed Pistol",
                    range_min: 6.0,
                    range_max: 22.0,
                    aggression: 0.56,
                    flank_weight: 0.78,
                    hold_angle_weight: 0.24,
                    push_weight: 0.46,
                    retreat_health_threshold: 0.32,
                    peek_time: 0.6,
                    reposition_interval: 1.1,
                    noise_investigation_weight: 0.82,
                    base_reaction_time: 0.38,
                    base_aim_offset: 0.06,
                    base_repath_interval: 0.85,
                    base_fov_degrees: 170.0,
                },
                Normal,
            ),
        BotPersonalityId::Nexto =>
            (
                BotPersonality {
                    name: "Nexto",
                    archetype: AllRounder,
                    weapon: "M4A1",
                    range_min: 8.0,
                    range_max: 36.0,
                    aggression: 0.78,
                    flank_weight: 0.6,
                    hold_angle_weight: 0.5,
                    push_weight: 0.75,
                    retreat_health_threshold: 0.2,
                    peek_time: 0.6,
                    reposition_interval: 1.2,
                    noise_investigation_weight: 0.82,
                    base_reaction_time: 0.38,
                    base_aim_offset: 0.018,
                    base_repath_interval: 0.85,
                    base_fov_degrees: 190.0,
                },
                Nightmare,
            ),
        BotPersonalityId::Bandit =>
            (
                BotPersonality {
                    name: "Bandit",
                    archetype: Rusher,
                    weapon: "Tommy Gun",
                    range_min: 5.0,
                    range_max: 20.0,
                    aggression: 0.9,
                    flank_weight: 0.32,
                    hold_angle_weight: 0.1,
                    push_weight: 0.95,
                    retreat_health_threshold: 0.18,
                    peek_time: 0.45,
                    reposition_interval: 1.0,
                    noise_investigation_weight: 0.58,
                    base_reaction_time: 0.55,
                    base_aim_offset: 0.09,
                    base_repath_interval: 1.2,
                    base_fov_degrees: 150.0,
                },
                Easy,
            ),
        BotPersonalityId::Slater =>
            (
                BotPersonality {
                    name: "Slater",
                    archetype: AllRounder,
                    weapon: "AK-47",
                    range_min: 10.0,
                    range_max: 34.0,
                    aggression: 0.66,
                    flank_weight: 0.5,
                    hold_angle_weight: 0.5,
                    push_weight: 0.58,
                    retreat_health_threshold: 0.26,
                    peek_time: 0.58,
                    reposition_interval: 1.5,
                    noise_investigation_weight: 0.58,
                    base_reaction_time: 0.3,
                    base_aim_offset: 0.032,
                    base_repath_interval: 0.8,
                    base_fov_degrees: 170.0,
                },
                Hard,
            ),
        BotPersonalityId::Storm =>
            (
                BotPersonality {
                    name: "Storm",
                    archetype: Rusher,
                    weapon: "FAMAS",
                    range_min: 8.0,
                    range_max: 28.0,
                    aggression: 0.72,
                    flank_weight: 0.4,
                    hold_angle_weight: 0.22,
                    push_weight: 0.78,
                    retreat_health_threshold: 0.24,
                    peek_time: 0.55,
                    reposition_interval: 1.2,
                    noise_investigation_weight: 0.62,
                    base_reaction_time: 0.36,
                    base_aim_offset: 0.05,
                    base_repath_interval: 0.95,
                    base_fov_degrees: 160.0,
                },
                Normal,
            ),
        BotPersonalityId::Ghost =>
            (
                BotPersonality {
                    name: "Ghost",
                    archetype: Camper,
                    weapon: "Bolt Action",
                    range_min: 20.0,
                    range_max: 65.0,
                    aggression: 0.26,
                    flank_weight: 0.18,
                    hold_angle_weight: 0.92,
                    push_weight: 0.1,
                    retreat_health_threshold: 0.48,
                    peek_time: 0.95,
                    reposition_interval: 3.0,
                    noise_investigation_weight: 0.34,
                    base_reaction_time: 0.28,
                    base_aim_offset: 0.022,
                    base_repath_interval: 1.2,
                    base_fov_degrees: 145.0,
                },
                Hard,
            ),
        BotPersonalityId::Sniper =>
            (
                BotPersonality {
                    name: "Sniper",
                    archetype: Sniper,
                    weapon: "Bolt Action",
                    range_min: 24.0,
                    range_max: 80.0,
                    aggression: 0.2,
                    flank_weight: 0.15,
                    hold_angle_weight: 0.96,
                    push_weight: 0.06,
                    retreat_health_threshold: 0.52,
                    peek_time: 1.05,
                    reposition_interval: 3.4,
                    noise_investigation_weight: 0.3,
                    base_reaction_time: 0.24,
                    base_aim_offset: 0.016,
                    base_repath_interval: 1.25,
                    base_fov_degrees: 140.0,
                },
                Hard,
            ),
        BotPersonalityId::Bulwark =>
            (
                BotPersonality {
                    name: "Bulwark",
                    archetype: Camper,
                    weapon: "Benelli",
                    range_min: 3.0,
                    range_max: 12.0,
                    aggression: 0.12,
                    flank_weight: 0.05,
                    hold_angle_weight: 0.92,
                    push_weight: 0.05,
                    retreat_health_threshold: 0.55,
                    peek_time: 2.5,
                    reposition_interval: 15.0,
                    noise_investigation_weight: 0.15,
                    base_reaction_time: 0.6,
                    base_aim_offset: 0.12,
                    base_repath_interval: 3.0,
                    base_fov_degrees: 110.0,
                },
                Recruit,
            ),
        BotPersonalityId::Wraith =>
            (
                BotPersonality {
                    name: "Wraith",
                    archetype: Flanker,
                    weapon: "MP5",
                    range_min: 5.0,
                    range_max: 22.0,
                    aggression: 0.82,
                    flank_weight: 0.94,
                    hold_angle_weight: 0.05,
                    push_weight: 0.4,
                    retreat_health_threshold: 0.12,
                    peek_time: 0.35,
                    reposition_interval: 0.9,
                    noise_investigation_weight: 0.88,
                    base_reaction_time: 0.15,
                    base_aim_offset: 0.015,
                    base_repath_interval: 0.4,
                    base_fov_degrees: 210.0,
                },
                Nightmare,
            ),
        BotPersonalityId::Overwatch =>
            (
                BotPersonality {
                    name: "Overwatch",
                    archetype: Sniper,
                    weapon: "Bolt Action",
                    range_min: 18.0,
                    range_max: 70.0,
                    aggression: 0.18,
                    flank_weight: 0.05,
                    hold_angle_weight: 0.98,
                    push_weight: 0.02,
                    retreat_health_threshold: 0.38,
                    peek_time: 2.0,
                    reposition_interval: 8.0,
                    noise_investigation_weight: 0.2,
                    base_reaction_time: 0.12,
                    base_aim_offset: 0.008,
                    base_repath_interval: 0.5,
                    base_fov_degrees: 135.0,
                },
                Nightmare,
            ),
    }
}


/*
    =====================================================================
    COMPUTED TIERLIST  (base_stat * difficulty_multiplier = runtime value)
    =====================================================================
    Lower reaction/aim/repath = better.  Higher FOV/stickiness/burst/memory = better.

    RANK  BOT          DIFF   REACT   AIM      FOV    REPATH  STICK  BURST  WALL MEM
    ----- ------------ -----  ------  -------  -----  ------  -----  -----  --------
    S+    Overwatch    NM     0.054s  0.0032   270    0.25s   0.92   0.92   4.0s
           Near-aimbot with a semi-auto sniper. Locks a sightline and doesn't miss.
    S     Wraith       NM     0.068s  0.0060   270    0.20s   0.92   0.92   4.0s
           Elite flanker with an MP5. Fastest repath in the game, always behind you.
    S-    Nexto        NM     0.171s  0.0072   270    0.43s   0.92   0.92   4.0s
           Does everything well. The "pro player" bot — no weaknesses, adapts to any fight.
    ----- ------------ -----  ------  -------  -----  ------  -----  -----  --------
    A+    Sniper       H      0.180s  0.0112   180    0.94s   0.80   0.78   3.0s
           Bolt action headhunter. Holds long corridors, deadly accurate at range.
    A     Slater       H      0.225s  0.0224   180    0.60s   0.80   0.78   3.0s
           Methodical fragger. Doesn't panic, doesn't whiff — just consistent kills.
    A     Ghost        H      0.210s  0.0154   180    0.90s   0.80   0.78   3.0s
           Patient camper. Sits in one spot forever, punishes anyone who peeks.
    A     Heater       H      0.210s  0.0245   180    0.60s   0.80   0.78   3.0s
           Fast-entry duelist with shotgun. Kicks down the door and starts blasting.
    A-    Viper        H      0.225s  0.0280   180    0.49s   0.80   0.78   3.0s
           Flanking ambusher. Takes the long way around and catches you off guard.
    A-    Rex          H      0.225s  0.0266   180    0.56s   0.80   0.78   3.0s
           Frontline enforcer with AK. Walks forward and shoots everything in his path.
    ----- ------------ -----  ------  -------  -----  ------  -----  -----  --------
    B+    Stinger      N      0.360s  0.0500   155    0.90s   0.65   0.60   2.2s
           Poke and harass. Takes a shot, ducks, repositions, does it again.
    B     Storm        N      0.360s  0.0500   160    0.95s   0.65   0.60   2.2s
           Constant pressure rusher. Never stops moving, never stops shooting.
    B     Phantom      N      0.380s  0.0600   170    0.85s   0.65   0.60   2.2s
           Sneaky flanker with suppressed pistol. Tries to lurk but aim lets him down.
    B-    Merlin       N      0.400s  0.0550   150    1.10s   0.65   0.60   2.2s
           Smart angle holder. Good positioning, average mechanics — brains over aim.
    ----- ------------ -----  ------  -------  -----  ------  -----  -----  --------
    C     Bandit       E      0.715s  0.1440   150    1.68s   0.45   0.40   1.4s
           Chaotic spray-and-pray rusher. All aggression, no accuracy. Free kill if you aim.
    D     Bulwark      R      1.080s  0.2640   110    5.40s   0.25   0.20   0.8s
           Walking target practice. Sits in a corner with a shotgun and misses everything.

    DIFF KEY:  R = Recruit, E = Easy, N = Normal, H = Hard, NM = Nightmare
*/
