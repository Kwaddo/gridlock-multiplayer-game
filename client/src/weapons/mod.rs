/* Weapon system module.
 Architecture:
    stats.rs                – static per-weapon configurations (the "data")
    state.rs                – runtime weapon state + inventory
    recoil.rs               – bloom/spread/recoil math
    crosshair.rs            – dynamic per-weapon crosshair rendering
    effects.rs              - visual effects like tracers and damage numbers
    crosshair-presets.rs    - predefined crosshair styles for each weapon kind
*/


pub mod combat;
pub mod crosshair;
pub mod effects;
pub mod recoil;
pub mod state;
pub mod stats;
pub mod crosshair_presets;


// exports for convenience
pub use effects::{BulletTracer, DamageNumber};
pub use recoil::SpreadContext;
pub use state::{WeaponInventory, WeaponState};
pub use stats::WeaponKind;
