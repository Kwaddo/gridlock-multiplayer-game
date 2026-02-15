use super::{LoginField, LoginScreen, MainMenuTab};
use crate::net;
use crate::net::client::{poll_discovery, start_discovery};
use macroquad::prelude::*;
use std::net::UdpSocket;

impl LoginScreen {
    pub fn update(&mut self) -> Option<(UdpSocket, String, u32)> {
        let dt = get_frame_time();
        self.anim_time += dt;

        self.update_particles(dt);
        self.poll_discovery(dt);

        // deferred connect from previous frame
        if self.connecting {
            self.connecting = false;
            match net::connect(&self.ip_input, &self.username_input) {
                Some((socket, id, assigned_username)) => {
                    return Some((socket, assigned_username, id));
                }
                None => {
                    self.error_msg = "Failed to connect to server.".to_string();
                }
            }
            return None;
        }

        self.consume_input();
        None
    }

    fn update_particles(&mut self, dt: f32) {
        let sw = screen_width();
        let sh = screen_height();
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            if p.x < 0.0 {
                p.x += sw;
            }
            if p.x > sw {
                p.x -= sw;
            }
            if p.y < 0.0 {
                p.y += sh;
            }
            if p.y > sh {
                p.y -= sh;
            }
        }
    }

    fn poll_discovery(&mut self, dt: f32) {
        if self.scan_timer > 0.0 {
            self.scan_timer -= dt;
            if let Some(ref sock) = self.scan_socket {
                for ns in poll_discovery(sock) {
                    if !self.discovered_servers.iter().any(|s| s.addr == ns.addr) {
                        self.discovered_servers.push(ns);
                    }
                }
            }
            if self.discovered_servers.len() == 1 && self.ip_input.is_empty() {
                self.ip_input = self.discovered_servers[0].addr.clone();
            }
            if self.scan_timer <= 0.0 {
                self.scan_socket = None;
            }
        }
    }

    fn consume_input(&mut self) {
        if matches!(self.active_tab, MainMenuTab::Play) {
            if is_key_pressed(KeyCode::Tab) {
                self.active_field = match self.active_field {
                    LoginField::Ip => LoginField::Username,
                    LoginField::Username => LoginField::Ip,
                };
            }
            if is_key_pressed(KeyCode::Backspace) {
                match self.active_field {
                    LoginField::Ip => {
                        self.ip_input.pop();
                    }
                    LoginField::Username => {
                        self.username_input.pop();
                    }
                }
            }
            if is_key_pressed(KeyCode::Enter) {
                self.try_connect();
            }
        }

        while let Some(c) = get_char_pressed() {
            if matches!(self.active_tab, MainMenuTab::Play) {
                if c != '\t' && c != '\r' && c != '\n' && c as u32 != 8 {
                    match self.active_field {
                        LoginField::Ip => self.ip_input.push(c),
                        LoginField::Username => self.username_input.push(c),
                    }
                }
            }
        }
    }

    pub(super) fn try_connect(&mut self) {
        if self.ip_input.is_empty() || self.username_input.is_empty() {
            self.error_msg = "Both fields are required.".to_string();
        } else {
            self.error_msg.clear();
            self.connecting = true;
        }
    }

    pub(super) fn start_scan(&mut self) {
        self.discovered_servers.clear();
        self.scan_socket = start_discovery();
        self.scan_timer = 2.0;
    }
}
