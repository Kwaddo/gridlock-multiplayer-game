use crate::core::game_state::LoginField;

use macroquad::prelude::*;

use super::widgets::{button, FIELD_H};
use super::LoginScreen;

// Native builds join UDP hosts by address; the browser build joins rooms by code.
#[cfg(not(target_arch = "wasm32"))]
const ADDRESS_LABEL: &str = "SERVER ADDRESS";
#[cfg(not(target_arch = "wasm32"))]
const ADDRESS_PLACEHOLDER: &str = "127.0.0.1:7878";
#[cfg(target_arch = "wasm32")]
const ADDRESS_LABEL: &str = "ROOM CODE";
#[cfg(target_arch = "wasm32")]
const ADDRESS_PLACEHOLDER: &str = "ABC123";

impl LoginScreen {
    pub(super) fn draw_direct_tab(&mut self, px: f32, cy: f32, pw: f32, _ch: f32, pad: f32) {
        let fx = px + pad;
        let fw = pw - pad * 2.0;
        self.text_field(LoginField::Address, ADDRESS_LABEL, ADDRESS_PLACEHOLDER, fx, cy + 28.0, fw);

        let bw = 220.0;
        let by = cy + 28.0 + 10.0 + FIELD_H + 24.0;
        if button("JOIN", 20, px + (pw - bw) / 2.0, by, bw, 42.0) {
            let address = self.address_input.trim().to_string();
            self.join(&address);
        }
    }
}
