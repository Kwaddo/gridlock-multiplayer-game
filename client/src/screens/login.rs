// login screen rendering and input

use crate::core::game_state::*;
use crate::net;
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;
use std::net::UdpSocket;

pub struct LoginScreen {
    pub ip_input: String,
    pub username_input: String,
    pub active_field: LoginField,
    pub error_msg: String,
}

impl LoginScreen {
    pub fn new() -> LoginScreen {
        LoginScreen {
            ip_input: String::new(),
            username_input: String::new(),
            active_field: LoginField::Ip,
            error_msg: String::new(),
        }
    }

    pub fn update(&mut self) -> Option<(UdpSocket, String, u32)> {
        if is_key_pressed(KeyCode::Tab) {
            // tab to switch fields
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
            if self.ip_input.is_empty() || self.username_input.is_empty() {
                self.error_msg = "Both fields are required.".to_string();
            } else {
                match net::connect(&self.ip_input, &self.username_input) {
                    Some((socket, id, assigned_username)) => {
                        return Some((socket, assigned_username, id));
                    }
                    None => {
                        self.error_msg = "Failed to connect to server.".to_string();
                    }
                }
            }
        }

        while let Some(c) = get_char_pressed() {
            if c != '\t' && c != '\r' && c != '\n' && c as u32 != 8 {
                // ignore tab, enter, backspace
                match self.active_field {
                    LoginField::Ip => {
                        self.ip_input.push(c);
                    }
                    LoginField::Username => {
                        self.username_input.push(c);
                    }
                }
            }
        }
        None
    }

    pub fn draw(&self) {
        clear_background(Color::from_rgba(30, 30, 30, 255));

        let screen_w = screen_width();
        let box_w = 400.0;
        let box_x = (screen_w - box_w) / 2.0;
        let box_y = screen_height() / 2.0 - 120.0;

        let title = "MULTIPLAYER FPS";
        let title_w = measure_game_text(title, 40);
        draw_text_ex(
            title,
            (screen_w - title_w) / 2.0,
            box_y,
            text_params(40, WHITE),
        );

        // IP field
        let ip_y = box_y + 50.0;
        let ip_color = match self.active_field {
            LoginField::Ip => YELLOW,
            _ => GRAY,
        };
        draw_text_ex("Server IP:", box_x, ip_y, text_params(20, ip_color));
        draw_rectangle(
            box_x,
            ip_y + 5.0,
            box_w,
            30.0,
            Color::from_rgba(50, 50, 50, 255),
        );
        draw_rectangle_lines(box_x, ip_y + 5.0, box_w, 30.0, 2.0, ip_color);
        draw_text_ex(
            &self.ip_input,
            box_x + 5.0,
            ip_y + 25.0,
            text_params(20, WHITE),
        );

        // Username field
        let name_y = box_y + 120.0;
        let name_color = match self.active_field {
            LoginField::Username => YELLOW,
            _ => GRAY,
        };
        draw_text_ex("Username:", box_x, name_y, text_params(20, name_color));
        draw_rectangle(
            box_x,
            name_y + 5.0,
            box_w,
            30.0,
            Color::from_rgba(50, 50, 50, 255),
        );
        draw_rectangle_lines(box_x, name_y + 5.0, box_w, 30.0, 2.0, name_color);
        draw_text_ex(
            &self.username_input,
            box_x + 5.0,
            name_y + 25.0,
            text_params(20, WHITE),
        );

        // Hints
        draw_text_ex(
            "TAB to switch fields | ENTER to connect",
            box_x,
            box_y + 185.0,
            text_params(16, GRAY),
        );

        if !self.error_msg.is_empty() {
            draw_text_ex(&self.error_msg, box_x, box_y + 210.0, text_params(18, RED));
        }

        draw_text_ex(
            "F11 to toggle fullscreen",
            box_x,
            box_y + 230.0,
            text_params(16, GRAY),
        );
        
    }
}
