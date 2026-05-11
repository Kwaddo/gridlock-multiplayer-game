#[derive(Clone, Copy)]
pub struct ServerWeaponStats {
    pub damage: u32,
    pub range: f32,
    pub shots_per_sec: f32,
    pub pellet_count: u32,
}

pub fn get_weapon_stats(name: &str) -> ServerWeaponStats {
    match name {
        // Pistols
        "M1911" => ServerWeaponStats { damage: 22, range: 40.0, shots_per_sec: 5.0, pellet_count: 1 },
        "Desert Eagle" => ServerWeaponStats { damage: 75, range: 50.0, shots_per_sec: 1.2, pellet_count: 1 },
        "Revolver" => ServerWeaponStats { damage: 45, range: 45.0, shots_per_sec: 1.6, pellet_count: 1 },
        "Suppressed Pistol" => ServerWeaponStats { damage: 24, range: 35.0, shots_per_sec: 4.5, pellet_count: 1 },
        "Taurus Raging" => ServerWeaponStats { damage: 14, range: 15.0, shots_per_sec: 1.4, pellet_count: 1 },

        // SMGs
        "MP5" => ServerWeaponStats { damage: 20, range: 45.0, shots_per_sec: 11.0, pellet_count: 1 },
        "P90" => ServerWeaponStats { damage: 17, range: 40.0, shots_per_sec: 13.0, pellet_count: 1 },
        "Tommy Gun" => ServerWeaponStats { damage: 23, range: 35.0, shots_per_sec: 9.0, pellet_count: 1 },
        "Honey Badger" => ServerWeaponStats { damage: 26, range: 50.0, shots_per_sec: 8.0, pellet_count: 1 },

        // Shotguns — damage is per pellet
        "SPAS-12" => ServerWeaponStats { damage: 12, range: 20.0, shots_per_sec: 1.0, pellet_count: 8 },
        "Benelli" => ServerWeaponStats { damage: 10, range: 18.0, shots_per_sec: 2.5, pellet_count: 8 },

        // ARs
        "M16" => ServerWeaponStats { damage: 27, range: 70.0, shots_per_sec: 7.0, pellet_count: 1 },
        "M4A1" => ServerWeaponStats { damage: 25, range: 80.0, shots_per_sec: 8.0, pellet_count: 1 },
        "AK-47" => ServerWeaponStats { damage: 34, range: 75.0, shots_per_sec: 6.0, pellet_count: 1 },
        "FAMAS" => ServerWeaponStats { damage: 24, range: 70.0, shots_per_sec: 8.0, pellet_count: 1 },
        "SCAR-H" => ServerWeaponStats { damage: 30, range: 80.0, shots_per_sec: 6.5, pellet_count: 1 },
        "AS VAL" => ServerWeaponStats { damage: 22, range: 40.0, shots_per_sec: 10.0, pellet_count: 1 },

        // Snipers
        "Bolt Action" => ServerWeaponStats { damage: 105, range: 120.0, shots_per_sec: 0.8, pellet_count: 1 },
        "Hunting Rifle" => ServerWeaponStats { damage: 75, range: 100.0, shots_per_sec: 1.2, pellet_count: 1 },
        "Semi-Auto Sniper" => ServerWeaponStats { damage: 58, range: 110.0, shots_per_sec: 2.0, pellet_count: 1 },

        // Other / melee
        "Minigun" => ServerWeaponStats { damage: 14, range: 50.0, shots_per_sec: 20.0, pellet_count: 1 },
        "Punch" | "Knife" => ServerWeaponStats { damage: 44, range: 2.5, shots_per_sec: 3.0, pellet_count: 1 },

        // Safe fallback
        _ => ServerWeaponStats { damage: 20, range: 40.0, shots_per_sec: 5.0, pellet_count: 1 },
    }
}
