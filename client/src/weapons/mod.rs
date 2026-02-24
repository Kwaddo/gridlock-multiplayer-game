/* Weapon system module.
 Architecture:
    stats.rs                – static per-weapon configurations (the "data")
    state.rs                – runtime weapon state + inventory
    loadout.rs              – player loadout (which guns in which slots)
    recoil.rs               – bloom/spread/recoil math
    crosshair.rs            – dynamic per-weapon crosshair rendering
    effects.rs              - visual effects like tracers and damage numbers
    crosshair-presets.rs    - predefined crosshair styles for each weapon category
*/

pub mod combat;
pub mod crosshair;
pub mod crosshair_presets;
pub mod effects;
pub mod loadout;
pub mod recoil;
pub mod state;
pub mod stats;

// exports for convenience
pub use effects::{BulletHole, BulletTracer, DamageNumber};
pub use loadout::Loadout;
pub use recoil::SpreadContext;
pub use state::{WeaponInventory, WeaponState};
pub use stats::{WeaponCategory, WeaponId};

/// Gun Game weapon progression — must match the server's GUN_GAME_PROGRESSION.
/// gun_level 1 = first entry, gun_level 19 = last entry, gun_level 20 = knife only.
pub const GUN_GAME_PROGRESSION: [WeaponId; 19] = [
    // Pistols
    WeaponId::M1911,
    WeaponId::Deagle,
    WeaponId::Revolver,
    WeaponId::SuppressedPistol,
    // SMGs
    WeaponId::Mp5,
    WeaponId::P90,
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
    WeaponId::HuntingRifle,
    WeaponId::SemiAutoSniper,
    // Other
    WeaponId::Minigun,
];
