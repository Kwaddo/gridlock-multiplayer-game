use macroquad::prelude::*;

use super::LoginScreen;
use crate::core::game_state::MainMenuTab;
use crate::net::client::{join_target, ConnectPoll, Connector, Session};
use crate::net::hosting::LocalHost;

#[cfg(not(target_arch = "wasm32"))]
const ADDRESS_PROMPT: &str = "Enter a server address.";
#[cfg(target_arch = "wasm32")]
const ADDRESS_PROMPT: &str = "Enter a room code.";

impl LoginScreen {
    /// Runs one frame of the menu; returns the session once a join succeeds.
    pub fn update(&mut self) -> Option<Session> {
        self.update_background();
        self.browser.poll();

        if let Some(connector) = self.connector.take() {
            match connector.poll() {
                ConnectPoll::Pending(connector) => self.connector = Some(connector),
                ConnectPoll::Connected(session) => return Some(session),
                ConnectPoll::Failed(reason) => self.error_msg = reason.to_string(),
            }
            return None;
        }

        if is_key_pressed(KeyCode::Tab) {
            let fields = self.tab_fields();
            let next = fields.iter().position(|f| *f == self.active_field).map_or(0, |i| i + 1);
            self.active_field = fields[next % fields.len()];
        }
        if is_key_pressed(KeyCode::Backspace) {
            self.field_text_mut(self.active_field).pop();
        }
        if is_key_pressed(KeyCode::Enter) {
            self.confirm();
        }
        while let Some(c) = get_char_pressed() {
            // Backspace, Tab and Enter also arrive here as characters (macOS sends
            // Backspace as DEL, 0x7f); the key handlers above already did their job.
            if !c.is_control() {
                self.field_text_mut(self.active_field).push(c);
            }
        }

        None
    }

    /// ENTER does the main action of whichever tab is open.
    fn confirm(&mut self) {
        match self.active_tab {
            MainMenuTab::Browse => {}
            MainMenuTab::Host => self.start_hosting(),
            MainMenuTab::Direct => {
                let address = self.address_input.trim().to_string();
                self.join(&address);
            }
        }
    }

    pub(super) fn join(&mut self, address: &str) {
        if !self.require_username() {
            return;
        }
        let Some(target) = join_target(address) else {
            self.error_msg = ADDRESS_PROMPT.to_string();
            return;
        };
        self.begin(Connector::join(&target, self.username_input.trim()));
    }

    pub(super) fn start_hosting(&mut self) {
        if !self.require_username() {
            return;
        }
        let name = self.host_name();
        match LocalHost::start(&name, self.list_online) {
            Ok(host) => self.begin(Connector::join_own(host, self.username_input.trim())),
            Err(reason) => self.error_msg = reason,
        }
    }

    /// The server name to host under; defaults to "<username>'s game".
    pub(super) fn host_name(&self) -> String {
        match (self.server_name_input.trim(), self.username_input.trim()) {
            ("", "") => "Gridlock game".to_string(),
            ("", username) => format!("{username}'s game"),
            (name, _) => name.to_string(),
        }
    }

    /// Coming back from a game: say why, and show a fresh server list.
    pub fn returned_with(&mut self, reason: &str) {
        self.error_msg = reason.to_string();
        self.refresh_servers();
    }

    pub(super) fn refresh_servers(&mut self) {
        self.browser.refresh();
    }

    fn begin(&mut self, connector: Result<Connector, &'static str>) {
        match connector {
            Ok(connector) => {
                self.error_msg.clear();
                self.connector = Some(connector);
            }
            Err(reason) => self.error_msg = reason.to_string(),
        }
    }

    fn require_username(&mut self) -> bool {
        if self.username_input.trim().is_empty() {
            self.error_msg = "Enter a username first.".to_string();
            self.active_field = crate::core::game_state::LoginField::Username;
            return false;
        }
        true
    }
}
