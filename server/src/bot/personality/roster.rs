//! The named bot roster: base stats and difficulty of every personality.

use super::{BotArchetype, BotDifficulty, BotPersonality, BotPersonalityId};

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
                    base_turn_rate: 6.0,
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
                    base_turn_rate: 4.5,
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
                    base_turn_rate: 5.5,
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
                    base_turn_rate: 3.5,
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
                    base_turn_rate: 5.5,
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
                    base_turn_rate: 4.0,
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
                    base_turn_rate: 7.0,
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
                    base_turn_rate: 3.0,
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
                    base_turn_rate: 5.0,
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
                    base_turn_rate: 4.5,
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
                    base_turn_rate: 4.0,
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
                    base_turn_rate: 3.0,
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
                    base_turn_rate: 2.0,
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
                    base_turn_rate: 9.0,
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
                    base_turn_rate: 5.0,
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
