use crate::core::game_state::LoginField;
use crate::net::registry::registry_url;
use crate::ui::fonts::{measure_game_text, text_params};

use macroquad::prelude::*;

use super::widgets::{button, checkbox, FIELD_H};
use super::{LoginScreen, TEXT_DIM};

#[cfg(not(target_arch = "wasm32"))]
const HINT_OFFLINE: &str = "Online listing is not configured. LAN players can still join.";
#[cfg(not(target_arch = "wasm32"))]
const HINT_LISTED: &str = "The game will ask your router to open UDP port 7878 automatically.";
#[cfg(not(target_arch = "wasm32"))]
const HINT_UNLISTED: &str = "Visible to players on your local network.";
#[cfg(target_arch = "wasm32")]
const HINT_OFFLINE: &str = "Online play is not configured in this build.";
#[cfg(target_arch = "wasm32")]
const HINT_LISTED: &str = "Anyone can find your game under BROWSE, or join with its room code.";
#[cfg(target_arch = "wasm32")]
const HINT_UNLISTED: &str = "Players join with the room code you get once hosting.";

impl LoginScreen {
    pub(super) fn draw_host_tab(&mut self, px: f32, cy: f32, pw: f32, _ch: f32, pad: f32) {
        let fx = px + pad;
        let fw = pw - pad * 2.0;

        let placeholder = self.host_name();
        self.text_field(LoginField::ServerName, "SERVER NAME", &placeholder, fx, cy + 28.0, fw);

        let online_available = registry_url().is_some();
        let cb_y = cy + 28.0 + 10.0 + FIELD_H + 18.0;
        checkbox(&mut self.list_online, online_available, "List on the online server browser", fx, cb_y);

        let hint = if !online_available {
            HINT_OFFLINE
        } else if self.list_online {
            HINT_LISTED
        } else {
            HINT_UNLISTED
        };
        draw_text_ex(hint, fx, cb_y + 40.0, text_params(12, TEXT_DIM));

        let bw = 240.0;
        let by = cb_y + 64.0;
        if button("HOST GAME", 20, px + (pw - bw) / 2.0, by, bw, 42.0) {
            self.start_hosting();
        }

        let note = "Your game ends when you leave it.";
        let nw = measure_game_text(note, 12);
        draw_text_ex(note, px + (pw - nw) / 2.0, by + 62.0, text_params(12, TEXT_DIM));
    }
}
