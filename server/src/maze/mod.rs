mod data;
mod decoration;
mod generation;
pub mod pathfinding;
pub mod types;
pub mod analysis;

pub use data::MazeData;
#[allow(unused_imports)]
pub use types::{MazeLight, Prop, PropKind, Room};
