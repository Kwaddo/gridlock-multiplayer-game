/*
 * Weapon stats and constants.
 * These are all the tunable numbers that define how each weapon behaves.
 * They are used by the client for visuals and feel, but the authoritative
 * logic lives on the server (see `server/src/weapons.rs`).
 *
 * ARCHITECTURE:
 *   WeaponCategory — the broad class (Pistol, Smg, Shotgun, AssaultRifle, Sniper, Other)
 *   WeaponId       — a specific gun (M1911, Deagle, Ak, Mp5, …)
 *
 * To add a new gun:
 *   1. Add a variant to `WeaponId`
 *   2. Create a `const YOUR_GUN: WeaponStats` in the appropriate submodule
 *   3. Wire it in `WeaponId::stats()` and `WeaponId::category()` below
 *   4. Add the model path to manifest.json
 */

pub mod pistols;
pub mod smgs;
pub mod shotguns;
pub mod assault_rifles;
pub mod snipers;
pub mod other;

// --- weapon category (broad class) ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WeaponCategory {
    Pistol,
    Smg,
    Shotgun,
    AssaultRifle,
    Sniper,
    Other,
}

impl WeaponCategory {
    pub const ALL: [WeaponCategory; 6] = [
        WeaponCategory::Pistol,
        WeaponCategory::Smg,
        WeaponCategory::Shotgun,
        WeaponCategory::AssaultRifle,
        WeaponCategory::Sniper,
        WeaponCategory::Other,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            WeaponCategory::Pistol => "Pistols",
            WeaponCategory::Smg => "SMGs",
            WeaponCategory::Shotgun => "Shotguns",
            WeaponCategory::AssaultRifle => "Assault Rifles",
            WeaponCategory::Sniper => "Snipers",
            WeaponCategory::Other => "Other",
        }
    }
}

// --- weapon id (specific gun) ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WeaponId {
    // Pistols
    M1911,
    Deagle,
    Revolver,
    SuppressedPistol,
    // TaurusRaging,

    // SMGs
    Mp5,
    P90,
    // HoneyBadger,
    TommyGun,

    // Shotguns
    Benelli,
    Spas,

    // Assault Rifles
    M4a1,
    Ak,
    Famas,
    M16,
    Scar,
    AsVal,

    // Snipers
    BoltAction,
    // HuntingRifle,
    // SemiAutoSniper,

    // Other
    // GrenadeLauncher,
    Minigun,
    // Rpg,
}

impl WeaponId {
    pub const ALL: &[WeaponId] = &[
        // Pistols
        WeaponId::M1911,
        WeaponId::Deagle,
        WeaponId::Revolver,
        WeaponId::SuppressedPistol,
        // WeaponId::TaurusRaging,

        // SMGs
        WeaponId::Mp5,
        WeaponId::P90,
        // WeaponId::HoneyBadger,
        WeaponId::TommyGun,

        // Shotguns
        WeaponId::Benelli,
        WeaponId::Spas,

        // Assault Rifles
        WeaponId::M4a1,
        WeaponId::Ak,
        WeaponId::Famas,
        WeaponId::M16,
        WeaponId::Scar,
        WeaponId::AsVal,

        // Snipers
        WeaponId::BoltAction,
        // WeaponId::HuntingRifle,
        // WeaponId::SemiAutoSniper,

        // Other
        // WeaponId::GrenadeLauncher,
        WeaponId::Minigun,
        // WeaponId::Rpg,
    ];

    pub fn category(self) -> WeaponCategory {
        match self {
            WeaponId::M1911 | WeaponId::Deagle | WeaponId::Revolver
            | WeaponId::SuppressedPistol
            // | WeaponId::TaurusRaging
            => WeaponCategory::Pistol,

            WeaponId::Mp5 | WeaponId::P90
            // | WeaponId::HoneyBadger
            | WeaponId::TommyGun
            => WeaponCategory::Smg,

            WeaponId::Benelli |
            WeaponId::Spas
            => WeaponCategory::Shotgun,

            WeaponId::M4a1 | WeaponId::Ak | WeaponId::Famas
            | WeaponId::M16 | WeaponId::Scar
            | WeaponId::AsVal
            => WeaponCategory::AssaultRifle,

            WeaponId::BoltAction
            // | WeaponId::HuntingRifle | WeaponId::SemiAutoSniper
            => WeaponCategory::Sniper,

            // WeaponId::GrenadeLauncher | WeaponId::Rpg |
            WeaponId::Minigun
            => WeaponCategory::Other,
        }
    }

    pub fn stats(self) -> &'static WeaponStats {
        match self {
            // Pistols
            WeaponId::M1911 => &pistols::M1911,
            WeaponId::Deagle => &pistols::DEAGLE,
            WeaponId::Revolver => &pistols::REVOLVER,
            WeaponId::SuppressedPistol => &pistols::SUPPRESSED_PISTOL,
            // WeaponId::TaurusRaging => &pistols::TAURUS_RAGING,

            // SMGs
            WeaponId::Mp5 => &smgs::MP5,
            WeaponId::P90 => &smgs::P90,
            // WeaponId::HoneyBadger => &smgs::HONEY_BADGER,
            WeaponId::TommyGun => &smgs::TOMMY_GUN,

            // Shotguns
            WeaponId::Benelli => &shotguns::BENELLI,
            WeaponId::Spas => &shotguns::SPAS,

            // Assault Rifles
            WeaponId::M4a1 => &assault_rifles::M4A1,
            WeaponId::Ak => &assault_rifles::AK,
            WeaponId::Famas => &assault_rifles::FAMAS,
            WeaponId::M16 => &assault_rifles::M16_STATS,
            WeaponId::Scar => &assault_rifles::SCAR,
            WeaponId::AsVal => &assault_rifles::AS_VAL,

            // Snipers
            WeaponId::BoltAction => &snipers::BOLT_ACTION,
            // WeaponId::HuntingRifle => &snipers::HUNTING_RIFLE,
            // WeaponId::SemiAutoSniper => &snipers::SEMI_AUTO_SNIPER,

            // Other
            // WeaponId::GrenadeLauncher => &other::GRENADE_LAUNCHER,
            WeaponId::Minigun => &other::MINIGUN,
            // WeaponId::Rpg => &other::RPG,
        }
    }

    pub fn display_name(self) -> &'static str {
        self.stats().name
    }

    /// All weapon IDs belonging to a given category.
    pub fn all_in(cat: WeaponCategory) -> Vec<WeaponId> {
        Self::ALL.iter().copied().filter(|w| w.category() == cat).collect()
    }
}

// crosshair visual style (drives how `crosshair.rs` renders).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrosshairStyle {
    // Standard weapon — hip-fire and ADS are texture crosshairs
    Standard,
    // Sniper — hip-fire is no crosshair, ADS is a full scope overlay
    Scope,
}

// fire mode determines input behaviour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireMode {
    SemiAuto,      // one shot per click
    FullAuto,      // hold to keep firing
    BoltAction,    // delay after each shot
    Burst(u32),    // one click fires N shots automatically, then must release
}

// all the numbers that define how a weapon behaves and feels.
#[derive(Clone)]
pub struct WeaponStats {
    pub name: &'static str,
    pub fire_mode: FireMode,

    // --- Damage
    pub damage: f32,
    pub headshot_mult: f32,
    pub pellet_count: u32, // 1 for most guns, >1 for shotgun
    pub pellet_spread: Option<&'static [(f32, f32)]>, // (radian around circle, how far it deviates from center)

    // --- Fire rate & ammo
    pub fire_rate: f32, // rounds per second
    pub magazine_size: u32,
    pub max_reserve: u32, // total spare ammo
    pub reload_time: f32, // seconds

    // --- Recoil
    pub recoil_vertical: f32,   // pitch kick per shot (radians)
    pub recoil_horizontal: f32, // max random yaw kick per shot (radians)
    pub recoil_recovery: f32,   // how fast recoil settles back (rad/s)

    // --- Spread / bloom
    pub hip_spread: f32,         // base spread at hip (radians)
    pub ads_spread: f32,         // base spread when aimed (radians)
    pub spread_per_shot: f32,    // bloom added per shot (radians)
    pub spread_recovery: f32,    // bloom decay rate when not firing (rad/s)
    pub move_spread_mult: f32,   // multiplier applied to spread when moving
    pub sprint_spread_mult: f32, // multiplier when sprinting (stacks with move)
    pub crouch_spread_mult: f32, // multiplier when crouching (<1 = tighter)

    // --- ADS (aim down sights)
    pub ads_zoom_fov: f32,            // target FOV when ADS (lower = more zoom)
    pub ads_time: f32,                // seconds to reach full ADS
    pub ads_sensitivity: Option<f32>, // custom sensitivity multiplier when ADS (None = use default), will only use this for sniper scope

    // --- Bullet visual
    pub bullet_speed: f32,     // tracer speed (units/s), purely visual
    pub bullet_range: f32,     // max hitscan distance (units)
    pub tracer_length: f32,    // visible tracer trail length (units)
    pub tracer_color: [u8; 4], // color of the bullet tracer (for visual variety, not necessarily tied to damage or team color)
    pub tracer_thickness: f32, // thickness of the bullet tracer line (units)
    pub tracer_lifetime: f32,  // how long the tracer trail lingers (seconds)

    // --- Crosshair kick (visual gap expansion on fire, not user-editable)
    pub crosshair_kick_per_shot: f32, // how much gap to add per shot (% of screen height)
    pub crosshair_kick_decay: f32,    // how quickly the gap recovers (units per second)
    pub crosshair_kick_max: f32,      // maximum kick gap

    // --- Crosshair
    pub crosshair_style: CrosshairStyle, // which render path (Standard = cross/texture, Scope = sniper overlay)
    pub fsa_time: Option<f32>,           // seconds to reach first shot accuracy, None = never (SMG)
    pub fsa_suppresses_kick: bool,       // when true, FSA shot skips recoil + crosshair kick (precision weapons)
    // visual crosshair config (color, gap, thickness, textures) lives on CrosshairConfig in state.rs

    // --- HUD icon
    pub fire_sound_asset_key: &'static str, // key used to look up shot sound in manifest-loaded audio map
    pub icon_asset_key: &'static str,       // key used to look up texture in assets HashMap
    pub icon_width: f32,                    // display width in pixels (tweak per weapon)
    pub icon_height: f32,                   // display height in pixels
    pub icon_flip_x: bool,                  // true if the source image faces left

    // --- First-person viewmodel (3D model or 2D sprite fallback)
    pub viewmodel_model: Option<&'static str>, // .obj model path
    pub viewmodel_asset_key: Option<&'static str>, // 2D sprite fallback
    pub viewmodel_scale: f32,                  // multiplier for model size
    pub viewmodel_position: (f32, f32, f32),   // position offset (right, up, forward)
    pub viewmodel_rotation: (f32, f32, f32),   // rotation (pitch, yaw, roll) in radians
    pub viewmodel_muzzle_offset: (f32, f32, f32), // muzzle in normalized model space
    pub viewmodel_hands_scale: f32,            // per-weapon hands scale multiplier
    pub viewmodel_hands_offset: (f32, f32, f32), // per-weapon hand position offset
    pub viewmodel_hands_rotation: (f32, f32, f32), // per-weapon hand rotation (pitch, yaw, roll)
    pub viewmodel_pair_offset_pos: (f32, f32, f32), // hand+gun locked-pair position offset
    pub viewmodel_pair_offset_rot: (f32, f32, f32), // hand+gun locked-pair rotation offset (pitch, yaw, roll)
    pub viewmodel_ads_pos_offset: (f32, f32, f32),        // gun position delta at full ADS
    pub viewmodel_ads_rot_offset: (f32, f32, f32),        // gun rotation delta at full ADS
    pub viewmodel_ads_hands_pos_offset: (f32, f32, f32),  // hands position delta at full ADS
    pub viewmodel_ads_hands_rot_offset: (f32, f32, f32),  // hands rotation delta at full ADS

    // --- damage falloff
    pub falloff_start: f32, // distance at which damage falloff begins (units)
    pub falloff_end: f32,   // distance at which damage reaches minimum (units)
    pub falloff_min_mult: f32, // minimum damage multiplier at max falloff (0..1)

    // --- spray pattern
    pub recoil_pattern: &'static [(f32, f32)], // list of (vertical, horizontal) recoil offsets for each shot in the magazine, applied in order and looping if you exceed the magazine size (like real guns with repeating patterns) rather than randomness shit
    pub pattern_randomness: f32, // how much random variance to add to the recoil pattern (radians),
    pub randomness_growth: f32, // additional randomness per shot to make the pattern less predictable the longer you fire (radians)
    pub pattern_reset_time: f32, // time in seconds to fully reset after releasing trigger
}

pub(super) const VM_BASE_SCALE: f32 = 0.003;
pub(super) const VM_BASE_POS: (f32, f32, f32) = (0.30, 0.08, -0.37);
pub(super) const VM_BASE_ROT: (f32, f32, f32) = (0.05, -1.45, 0.0);
