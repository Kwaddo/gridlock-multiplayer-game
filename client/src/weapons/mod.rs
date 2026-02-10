/* Weapon system module.
 Architecture:
    stats.rs     – static per-weapon configurations (the "data")
    state.rs     – runtime weapon state + inventory
    recoil.rs    – bloom/spread/recoil math
    firing.rs    – fire logic, hitscan raycast, bullet spawn
    bullet.rs    – bullet tracers + wall mark decals'
    crosshair.rs – dynamic per-weapon crosshair rendering
*/


pub mod combat;
pub mod crosshair;
pub mod effects;
pub mod recoil;
pub mod state;
pub mod stats;

// exports for convenience
pub use effects::{BulletTracer, DamageNumber};
pub use recoil::SpreadContext;
pub use state::{WeaponInventory, WeaponState};
pub use stats::WeaponKind;
