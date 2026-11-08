// Pistol stat tables, split by family: semi-automatic sidearms (M1911,
// Desert Eagle, suppressed pistol), revolvers (Revolver, Taurus Raging) and
// the Taurus Raging's ammo-mode switches.

mod revolvers;
mod semi_auto;
mod taurus_modes;

pub use revolvers::{REVOLVER, TAURUS_RAGING, TAURUS_RAGING_SHOTGUN_PELLET_SPREAD};
pub use semi_auto::{DEAGLE, M1911, SUPPRESSED_PISTOL};
pub use taurus_modes::{apply_taurus_raging_pistol_mode, apply_taurus_raging_shotgun_mode};
