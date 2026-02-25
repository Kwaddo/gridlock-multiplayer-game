use crate::bot::ai::BotBehavior;
use crate::bot::shooting::BotShootingConfig;
use crate::player::Team;

pub const LEVEL_3_TIME_LIMIT_SECS: u64 = 45;

#[derive(Clone)]
pub struct ShootingRangeBotConfig {
    pub position: (f32, f32, f32),
    pub behavior: BotBehavior,
    pub team: Team,
    pub weapon: Option<&'static str>,
    pub health: Option<u32>,
    pub can_shoot: Option<bool>,
    pub shooting: Option<BotShootingConfig>,
}

pub struct ShootingRangeLevelConfig {
    pub default_weapon: &'static str,
    pub default_health: u32,
    pub default_can_shoot: bool,
    pub default_shooting: BotShootingConfig,
    pub time_limit_secs: Option<u64>,
    pub bots: Vec<ShootingRangeBotConfig>,
}

fn behavior_from_code(code: u8) -> BotBehavior {
    match code {
        0 => BotBehavior::Static,
        1 => BotBehavior::Patrol {
            waypoints: vec![(38.0, 0.0, -1.2), (38.0, 0.0, 1.2)],
            current: 0,
            arrival_threshold: 0.6,
        },
        2 => BotBehavior::Patrol {
            waypoints: vec![(37.0, 0.0, 0.0), (39.5, 0.0, 0.0)],
            current: 0,
            arrival_threshold: 0.6,
        },
        _ => BotBehavior::Static,
    }
}

pub fn lane_positions() -> [(f32, f32, f32); 6] {
    [
        (39.0, 0.0, -7.0),
        (39.0, 0.0, -4.2),
        (39.0, 0.0, -1.4),
        (39.0, 0.0, 1.4),
        (39.0, 0.0, 4.2),
        (39.0, 0.0, 7.0),
    ]
}

pub fn level_config(level: u8, behavior_override: Option<u8>) -> ShootingRangeLevelConfig {
    match level {
        1 => level_1_config(behavior_override),
        2 => level_2_config(behavior_override),
        3 => level_3_config(),
        _ => level_1_config(behavior_override),
    }
}

fn level_1_config(_behavior_override: Option<u8>) -> ShootingRangeLevelConfig {
    let p = lane_positions();
    let level_1_weapons: [&str; 6] = [
        "M1911",
        "Desert Eagle",
        "Revolver",
        "Suppressed Pistol",
        "MP5",
        "P90",
    ];

    let bots = p
        .iter()
        .enumerate()
        .map(|(i, pos)| ShootingRangeBotConfig {
            position: (38.0, 0.0, pos.2),
            behavior: BotBehavior::Static,
            team: if i % 2 == 0 { Team::Red } else { Team::Blue },
            weapon: Some(level_1_weapons[i % level_1_weapons.len()]),
            health: Some(100),
            can_shoot: Some(true),
            shooting: Some(BotShootingConfig {
                fire_rate: 1.6,
                angular_offset: 0.16,
                reaction_time: 1.2,
            }),
        })
        .collect();

    ShootingRangeLevelConfig {
        default_weapon: "M1911",
        default_health: 100,
        default_can_shoot: true,
        default_shooting: BotShootingConfig {
            fire_rate: 1.8,
            angular_offset: 0.18,
            reaction_time: 1.5,
        },
        time_limit_secs: None,
        bots,
    }
}

fn level_2_config(behavior_override: Option<u8>) -> ShootingRangeLevelConfig {
    let p = lane_positions();
    let level_2_weapons: [&str; 6] = [
        "M4A1",
        "AK-47",
        "FAMAS",
        "SCAR-H",
        "Benelli",
        "SPAS-12",
    ];

    let side_to_side = |base_z: f32| BotBehavior::Patrol {
        waypoints: vec![(38.0, 0.0, base_z - 1.2), (38.0, 0.0, base_z + 1.2)],
        current: 0,
        arrival_threshold: 0.5,
    };
    let front_back = |base_z: f32| BotBehavior::Patrol {
        waypoints: vec![(37.0, 0.0, base_z), (39.5, 0.0, base_z)],
        current: 0,
        arrival_threshold: 0.5,
    };

    let forced_behavior = behavior_override.map(behavior_from_code);
    let bots = p
        .iter()
        .enumerate()
        .map(|(i, pos)| {
            let behavior = forced_behavior.clone().unwrap_or_else(|| {
                if i % 2 == 0 {
                    side_to_side(pos.2)
                } else {
                    front_back(pos.2)
                }
            });
            ShootingRangeBotConfig {
                position: (38.0, 0.0, pos.2),
                behavior,
                team: if i % 2 == 0 { Team::Red } else { Team::Blue },
                weapon: Some(level_2_weapons[i % level_2_weapons.len()]),
                health: Some(100),
                can_shoot: Some(true),
                shooting: Some(BotShootingConfig {
                    fire_rate: 1.0,
                    angular_offset: 0.09,
                    reaction_time: 0.9,
                }),
            }
        })
        .collect();

    ShootingRangeLevelConfig {
        default_weapon: "M4A1",
        default_health: 100,
        default_can_shoot: true,
        default_shooting: BotShootingConfig {
            fire_rate: 1.0,
            angular_offset: 0.08,
            reaction_time: 1.0,
        },
        time_limit_secs: None,
        bots,
    }
}

fn level_3_config() -> ShootingRangeLevelConfig {
    // 15 stationary one-shot targets spread around the range.
    let positions: [(f32, f32, f32); 15] = [
        (33.0, 0.0, -7.0),
        (34.5, 0.0, -5.6),
        (36.0, 0.0, -6.8),
        (37.5, 0.0, -4.8),
        (39.0, 0.0, -6.0),
        (33.8, 0.0, -2.5),
        (35.2, 0.0, -1.0),
        (36.8, 0.0, -3.0),
        (38.5, 0.0, -1.6),
        (34.0, 0.0, 1.8),
        (35.6, 0.0, 3.0),
        (37.2, 0.0, 2.2),
        (38.8, 0.0, 4.2),
        (36.0, 0.0, 6.0),
        (39.2, 0.0, 6.8),
    ];

    let bots = positions
        .iter()
        .enumerate()
        .map(|(i, pos)| ShootingRangeBotConfig {
            position: *pos,
            behavior: BotBehavior::Static,
            team: if i % 2 == 0 { Team::Red } else { Team::Blue },
            weapon: Some("Knife"),
            health: Some(1),
            can_shoot: Some(false),
            shooting: None,
        })
        .collect();

    ShootingRangeLevelConfig {
        default_weapon: "Knife",
        default_health: 1,
        default_can_shoot: false,
        default_shooting: BotShootingConfig {
            fire_rate: 999.0,
            angular_offset: 0.0,
            reaction_time: 999.0,
        },
        time_limit_secs: Some(LEVEL_3_TIME_LIMIT_SECS),
        bots,
    }
}
