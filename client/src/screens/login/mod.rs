use crate::core::game_state::*;
use crate::net::client::DiscoveredServer;
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::net::UdpSocket;

mod background;
mod play_tab;
mod servers_tab;
mod update;

const MENU_BG: Color = Color::new(0.06, 0.06, 0.12, 1.0);
const PANEL_BG: Color = Color::new(0.09, 0.11, 0.19, 0.95);
const PANEL_BORDER: Color = Color::new(0.20, 0.24, 0.40, 0.7);
const ACCENT: Color = Color::new(1.0, 0.42, 0.21, 1.0);
const ACCENT_HOVER: Color = Color::new(1.0, 0.55, 0.38, 1.0);
const INPUT_BG: Color = Color::new(0.04, 0.05, 0.10, 1.0);
const INPUT_BORDER: Color = Color::new(0.20, 0.24, 0.38, 1.0);
const TEXT_DIM: Color = Color::new(0.50, 0.52, 0.62, 1.0);
const TEXT_LABEL: Color = Color::new(0.72, 0.74, 0.84, 1.0);
const TAB_ACTIVE_BG: Color = Color::new(1.0, 0.42, 0.21, 0.12);
const TAB_INACTIVE_TEXT: Color = Color::new(0.42, 0.44, 0.56, 1.0);
const ROW_BG: Color = Color::new(0.07, 0.08, 0.15, 1.0);
const ROW_HOVER: Color = Color::new(0.12, 0.14, 0.24, 1.0);

struct MenuParticle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    size: f32,
    alpha: f32,
}

impl MenuParticle {
    fn random(w: f32, h: f32) -> Self {
        Self {
            x: gen_range(0.0, w),
            y: gen_range(0.0, h),
            vx: gen_range(-12.0, 12.0),
            vy: gen_range(-8.0, 8.0),
            size: gen_range(0.5, 2.5),
            alpha: gen_range(0.08, 0.30),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ModelQuality {
    Compressed,
    Full,
}

impl ModelQuality {
    pub fn label(self) -> &'static str {
        match self {
            ModelQuality::Compressed => "Compressed",
            ModelQuality::Full => "Full",
        }
    }

    fn toggle(self) -> Self {
        match self {
            ModelQuality::Compressed => ModelQuality::Full,
            ModelQuality::Full => ModelQuality::Compressed,
        }
    }
}

pub struct LoginScreen {
    pub ip_input: String,
    pub username_input: String,
    pub active_field: LoginField,
    pub error_msg: String,
    pub active_tab: MainMenuTab,
    pub model_quality: ModelQuality,
    discovered_servers: Vec<DiscoveredServer>,
    scan_socket: Option<UdpSocket>,
    scan_timer: f32,
    particles: Vec<MenuParticle>,
    anim_time: f32,
    connecting: bool,
}

impl LoginScreen {
    pub fn new() -> Self {
        let sw = screen_width().max(800.0);
        let sh = screen_height().max(600.0);
        let particles = (0..80).map(|_| MenuParticle::random(sw, sh)).collect();

        let scan_socket = crate::net::client::start_discovery();
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
            model_quality: ModelQuality::Compressed,
        }
    }

    pub fn draw(&mut self) {
        self.draw_background();
        self.draw_title();

        let sw = screen_width();
        let sh = screen_height();

        let panel_w = 520.0_f32.min(sw * 0.85);
        let panel_h = 380.0;
        let panel_x = (sw - panel_w) / 2.0;
        let panel_y = sh * 0.28;
        draw_rectangle(panel_x, panel_y, panel_w, panel_h, PANEL_BG);
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, PANEL_BORDER);

        let tab_h = 38.0;
        let tab_w = panel_w / 2.0;
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);

        let is_play = matches!(self.active_tab, MainMenuTab::Play);
        let play_hover = in_rect(mouse, panel_x, panel_y, tab_w, tab_h);
        if play_hover && clicked {
            self.active_tab = MainMenuTab::Play;
        }
        draw_rectangle(
            panel_x,
            panel_y,
            tab_w,
            tab_h,
            if is_play {
                TAB_ACTIVE_BG
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            },
        );
        if is_play {
            draw_rectangle(panel_x, panel_y + tab_h - 2.0, tab_w, 2.0, ACCENT);
        }
        let ptw = measure_game_text("PLAY", 18);
        draw_text_ex(
            "PLAY",
            panel_x + (tab_w - ptw) / 2.0,
            panel_y + 26.0,
            text_params(18, if is_play { WHITE } else { TAB_INACTIVE_TEXT }),
        );

        let sx = panel_x + tab_w;
        let is_srv = matches!(self.active_tab, MainMenuTab::Servers);
        let srv_hover = in_rect(mouse, sx, panel_y, tab_w, tab_h);
        if srv_hover && clicked {
            self.active_tab = MainMenuTab::Servers;
            if self.scan_socket.is_none() {
                self.start_scan();
            }
        }
        draw_rectangle(
            sx,
            panel_y,
            tab_w,
            tab_h,
            if is_srv {
                TAB_ACTIVE_BG
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            },
        );
        if is_srv {
            draw_rectangle(sx, panel_y + tab_h - 2.0, tab_w, 2.0, ACCENT);
        }
        let stw = measure_game_text("SERVERS", 18);
        draw_text_ex(
            "SERVERS",
            sx + (tab_w - stw) / 2.0,
            panel_y + 26.0,
            text_params(18, if is_srv { WHITE } else { TAB_INACTIVE_TEXT }),
        );

        draw_rectangle(
            panel_x,
            panel_y + tab_h,
            panel_w,
            1.0,
            Color::new(0.20, 0.24, 0.40, 0.5),
        );

        let cy = panel_y + tab_h + 1.0;
        let ch = panel_h - tab_h - 1.0;
        let pad = 36.0;

        match self.active_tab {
            MainMenuTab::Play => self.draw_play_tab(panel_x, cy, panel_w, ch, pad),
            MainMenuTab::Servers => self.draw_servers_tab(panel_x, cy, panel_w, ch, pad),
        }

        let fy = panel_y + panel_h + 16.0;
        draw_text_ex(
            "F11 fullscreen   |   TAB switch fields   |   ENTER connect",
            panel_x,
            fy,
            text_params(13, TEXT_DIM),
        );

        if self.connecting {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.55));
            let t = "Connecting...";
            let tw = measure_game_text(t, 28);
            draw_text_ex(t, (sw - tw) / 2.0, sh / 2.0, text_params(28, ACCENT));
        }
    }
}

fn in_rect(mouse: (f32, f32), x: f32, y: f32, w: f32, h: f32) -> bool {
    mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h
}
