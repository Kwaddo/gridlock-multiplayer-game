#[derive(Clone, Copy)]
pub struct ServerWeaponStats {
    pub damage: u32,
    pub range: f32,
    pub shots_per_sec: f32,
}

pub fn get_weapon_stats(name: &str) -> ServerWeaponStats {
    match name {
        // Pistols
        "M1911" => ServerWeaponStats { damage: 22, range: 40.0, shots_per_sec: 5.0 },
        "Desert Eagle" => ServerWeaponStats { damage: 75, range: 50.0, shots_per_sec: 1.2 },
        "Revolver" => ServerWeaponStats { damage: 45, range: 45.0, shots_per_sec: 1.6 },
        "Suppressed Pistol" => ServerWeaponStats { damage: 24, range: 35.0, shots_per_sec: 4.5 },

        // SMGs
        "MP5" => ServerWeaponStats { damage: 20, range: 45.0, shots_per_sec: 11.0 },
        "P90" => ServerWeaponStats { damage: 17, range: 40.0, shots_per_sec: 13.0 },
        "Tommy Gun" => ServerWeaponStats { damage: 23, range: 35.0, shots_per_sec: 9.0 },

        // Shotguns
        "SPAS-12" => ServerWeaponStats { damage: 12, range: 20.0, shots_per_sec: 1.0 },
        "Benelli" => ServerWeaponStats { damage: 10, range: 18.0, shots_per_sec: 2.5 },

        // ARs
        "M16" => ServerWeaponStats { damage: 27, range: 70.0, shots_per_sec: 7.0 },
        "M4A1" => ServerWeaponStats { damage: 25, range: 80.0, shots_per_sec: 8.0 },
        "AK-47" => ServerWeaponStats { damage: 34, range: 75.0, shots_per_sec: 6.0 },
        "FAMAS" => ServerWeaponStats { damage: 24, range: 70.0, shots_per_sec: 8.0 },
        "SCAR-H" => ServerWeaponStats { damage: 30, range: 80.0, shots_per_sec: 6.5 },
        "AS VAL" => ServerWeaponStats { damage: 22, range: 40.0, shots_per_sec: 10.0 },

        // Snipers
        "Bolt Action" => ServerWeaponStats { damage: 105, range: 120.0, shots_per_sec: 0.8 },
        "Hunting Rifle" => ServerWeaponStats { damage: 75, range: 100.0, shots_per_sec: 1.2 },
        "Semi-Auto Sniper" => ServerWeaponStats { damage: 58, range: 110.0, shots_per_sec: 2.0 },

        // Other / melee
        "Minigun" => ServerWeaponStats { damage: 14, range: 50.0, shots_per_sec: 20.0 },
        "Knife" => ServerWeaponStats { damage: 40, range: 2.5, shots_per_sec: 3.0 },

        // Safe fallback
        _ => ServerWeaponStats { damage: 20, range: 40.0, shots_per_sec: 5.0 },
    }
}
