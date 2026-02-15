use super::particles::MenuParticle;
use crate::net::client::{start_discovery, DiscoveredServer};
use macroquad::prelude::*;
use std::net::UdpSocket;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginField {
    Ip,
    Username,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainMenuTab {
    Play,
    Servers,
}

pub struct LoginScreen {
    pub ip_input: String,
    pub username_input: String,
    pub active_field: LoginField,
    pub error_msg: String,
    pub active_tab: MainMenuTab,

    // server browser
    pub(super) discovered_servers: Vec<DiscoveredServer>,
    pub(super) scan_socket: Option<UdpSocket>,
    pub(super) scan_timer: f32,

    // visual state
    pub(super) particles: Vec<MenuParticle>,
    pub(super) anim_time: f32,

    // deferred connect (so we can show "Connecting..." for one frame)
    pub(super) connecting: bool,
}

impl LoginScreen {
    pub fn new() -> Self {
        let sw = screen_width().max(800.0);
        let sh = screen_height().max(600.0);
        let particles = (0..80).map(|_| MenuParticle::random(sw, sh)).collect();

        let scan_socket = start_discovery();
        Self {
            ip_input: String::new(),
            username_input: String::new(),
            active_field: LoginField::Ip,
            error_msg: String::new(),
            active_tab: MainMenuTab::Play,
            discovered_servers: Vec::new(),
            scan_socket,
            scan_timer: 2.0,
            particles,
            anim_time: 0.0,
            connecting: false,
        }
    }
}
