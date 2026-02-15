/*
    Crosshair settings tab — customize crosshairs per weapon with live preview.
    Features: HSV color picker, preset load/save/overwrite/delete, per-weapon config.
*/

mod color;
mod draw;
mod prompt;
mod state;
mod types;
mod update;
mod widgets;

pub use draw::draw_crosshair_settings;
pub use prompt::{draw_crosshair_save_prompt, update_crosshair_save_prompt};
pub use state::CrosshairSettingsPreview;
pub use update::update_crosshair_settings;
