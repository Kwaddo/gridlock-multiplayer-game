// Assault rifle stat tables, split by family: the classic platforms
// (M16, M4A1, AK) and the specialist rifles (FAMAS, SCAR-H, AS VAL).

mod classic;
mod specialist;

pub use classic::{ AK, M16_STATS, M4A1 };
pub use specialist::{ AS_VAL, FAMAS, SCAR };
