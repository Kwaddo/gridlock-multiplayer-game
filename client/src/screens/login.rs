// main menu screen with visual overhaul, server browser, and loading support

use crate::core::game_state::*;
use crate::net;
use crate::net::client::{poll_discovery, start_discovery, DiscoveredServer};
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::net::UdpSocket;

// ── color palette ──────────────────────────────────────────────────────────
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

// ── background particle ────────────────────────────────────────────────────
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

// ── main menu struct ───────────────────────────────────────────────────────
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

    // server browser
    discovered_servers: Vec<DiscoveredServer>,
    scan_socket: Option<UdpSocket>,
    scan_timer: f32,

    // visual state
    particles: Vec<MenuParticle>,
    anim_time: f32,

    // deferred connect (so we can show "Connecting..." for one frame)
    connecting: bool,
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
            model_quality: ModelQuality::Compressed,
        }
    }

    // ── per-frame logic (call first) ───────────────────────────────────────
    pub fn update(&mut self) -> Option<(UdpSocket, String, u32)> {
        let dt = get_frame_time();
        self.anim_time += dt;

        // animate particles
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

        // poll LAN discovery
        if self.scan_timer > 0.0 {
            self.scan_timer -= dt;
            if let Some(ref sock) = self.scan_socket {
                for ns in poll_discovery(sock) {
                    if !self.discovered_servers.iter().any(|s| s.addr == ns.addr) {
                        self.discovered_servers.push(ns);
                    }
                }
            }
            // auto-fill IP as soon as exactly one server is found
            if self.discovered_servers.len() == 1 && self.ip_input.is_empty() {
                self.ip_input = self.discovered_servers[0].addr.clone();
            }
            if self.scan_timer <= 0.0 {
                self.scan_socket = None;
            }
        }

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

        // keyboard input (play tab only, but ALWAYS drain the char buffer
        // so stale keypresses don't leak when switching tabs)
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
        // always drain the char buffer regardless of active tab
        while let Some(c) = get_char_pressed() {
            if matches!(self.active_tab, MainMenuTab::Play) {
                if c != '\t' && c != '\r' && c != '\n' && c as u32 != 8 {
                    match self.active_field {
                        LoginField::Ip => self.ip_input.push(c),
                        LoginField::Username => self.username_input.push(c),
                    }
                }
            }
            // chars on Servers tab are consumed and discarded
        }

        None
    }

    fn try_connect(&mut self) {
        if self.ip_input.is_empty() || self.username_input.is_empty() {
            self.error_msg = "Both fields are required.".to_string();
        } else {
            self.error_msg.clear();
            self.connecting = true;
        }
    }

    fn start_scan(&mut self) {
        self.discovered_servers.clear();
        self.scan_socket = start_discovery();
        self.scan_timer = 2.0;
    }

    // ── drawing ────────────────────────────────────────────────────────────
    pub fn draw(&mut self) {
        self.draw_background();
        self.draw_title();

        let sw = screen_width();
        let sh = screen_height();

        // ── panel ──
        let panel_w = 520.0_f32.min(sw * 0.85);
        let panel_h = 380.0;
        let panel_x = (sw - panel_w) / 2.0;
        let panel_y = sh * 0.28;
        draw_rectangle(panel_x, panel_y, panel_w, panel_h, PANEL_BG);
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, PANEL_BORDER);

        // ── tabs ──
        let tab_h = 38.0;
        let tab_w = panel_w / 2.0;
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);

        // play tab
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

        // servers tab
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

        // separator
        draw_rectangle(
            panel_x,
            panel_y + tab_h,
            panel_w,
            1.0,
            Color::new(0.20, 0.24, 0.40, 0.5),
        );

        // ── content ──
        let cy = panel_y + tab_h + 1.0;
        let ch = panel_h - tab_h - 1.0;
        let pad = 36.0;

        match self.active_tab {
            MainMenuTab::Play => self.draw_play_tab(panel_x, cy, panel_w, ch, pad),
            MainMenuTab::Servers => self.draw_servers_tab(panel_x, cy, panel_w, ch, pad),
        }

        // ── footer hints ──
        let fy = panel_y + panel_h + 16.0;
        draw_text_ex(
            "F11 fullscreen   |   TAB switch fields   |   ENTER connect",
            panel_x,
            fy,
            text_params(13, TEXT_DIM),
        );

        // ── connecting overlay ──
        if self.connecting {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.55));
            let t = "Connecting...";
            let tw = measure_game_text(t, 28);
            draw_text_ex(t, (sw - tw) / 2.0, sh / 2.0, text_params(28, ACCENT));
        }
    }

    // ── background (reusable for loading screen) ───────────────────────────
    pub fn update_background(&mut self) {
        let dt = get_frame_time();
        self.anim_time += dt;
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

    pub fn draw_background(&self) {
        clear_background(MENU_BG);

        // particles
        for p in &self.particles {
            draw_circle(p.x, p.y, p.size, Color::new(0.50, 0.55, 0.80, p.alpha));
        }

        // subtle scanlines
        let sw = screen_width();
        let sh = screen_height();
        let mut y = 0.0;
        while y < sh {
            draw_line(0.0, y, sw, y, 1.0, Color::new(0.0, 0.0, 0.0, 0.06));
            y += 4.0;
        }
    }

    fn draw_title(&self) {
        let sw = screen_width();
        let sh = screen_height();
        let title = "MULTIPLAYER FPS";
        let ts: u16 = 52;
        let tw = measure_game_text(title, ts);
        let tx = (sw - tw) / 2.0;
        let ty = sh * 0.15;

        // shadow
        draw_text_ex(
            title,
            tx + 2.0,
            ty + 2.0,
            text_params(ts, Color::new(0.0, 0.0, 0.0, 0.5)),
        );
        // glow pulse
        let g = ((self.anim_time * 0.8).sin() * 0.12 + 0.88).clamp(0.76, 1.0);
        draw_text_ex(title, tx, ty, text_params(ts, Color::new(g, g, g, 1.0)));

        // accent underline
        let lw = tw * 0.55;
        let lx = (sw - lw) / 2.0;
        let ly = ty + 14.0;
        draw_rectangle(lx, ly, lw, 2.0, ACCENT);
    }

    // ── loading screen (called from main.rs during Loading state) ──────────
    pub fn draw_loading(&self, total: usize, loaded: usize, current: &str, started: bool) {
        self.draw_background();
        self.draw_title();

        let sw = screen_width();
        let sh = screen_height();

        let pw = 480.0_f32.min(sw * 0.85);
        let ph = 150.0;
        let px = (sw - pw) / 2.0;
        let py = sh * 0.38;
        draw_rectangle(px, py, pw, ph, PANEL_BG);
        draw_rectangle_lines(px, py, pw, ph, 1.0, PANEL_BORDER);

        // label
        let label = if !started {
            "Receiving asset manifest..."
        } else if total == 0 {
            "Joining server..."
        } else {
            "Loading assets..."
        };
        let lw = measure_game_text(label, 20);
        draw_text_ex(label, (sw - lw) / 2.0, py + 34.0, text_params(20, WHITE));

        // progress bar
        let pad = 30.0;
        let bx = px + pad;
        let bw = pw - pad * 2.0;
        let bh = 22.0;
        let by = py + 54.0;
        draw_rectangle(bx, by, bw, bh, INPUT_BG);
        draw_rectangle_lines(bx, by, bw, bh, 1.0, PANEL_BORDER);
        if total > 0 {
            let frac = loaded as f32 / total as f32;
            let fw = (bw - 4.0) * frac;
            draw_rectangle(bx + 2.0, by + 2.0, fw, bh - 4.0, ACCENT);
        } else if !started {
            // indeterminate pulse
            let pulse_w = bw * 0.25;
            let offset = ((self.anim_time * 1.2).sin() * 0.5 + 0.5) * (bw - pulse_w);
            draw_rectangle(
                bx + 2.0 + offset,
                by + 2.0,
                pulse_w,
                bh - 4.0,
                Color::new(ACCENT.r, ACCENT.g, ACCENT.b, 0.45),
            );
        }

        // count
        if total > 0 {
            let count = format!("{} / {}", loaded, total);
            let cw = measure_game_text(&count, 15);
            draw_text_ex(
                &count,
                px + pw - pad - cw,
                by + bh + 18.0,
                text_params(15, TEXT_LABEL),
            );
        }

        // current asset name
        if !current.is_empty() {
            draw_text_ex(current, bx, by + bh + 18.0, text_params(13, TEXT_DIM));
        }
    }

    // ── play tab ───────────────────────────────────────────────────────────
    fn draw_play_tab(&mut self, px: f32, cy: f32, pw: f32, _ch: f32, pad: f32) {
        let field_w = pw - pad * 2.0;
        let field_h = 34.0;
        let fx = px + pad;
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);

        // ── ip field ──
        let ip_y = cy + 24.0;
        let ip_active = matches!(self.active_field, LoginField::Ip);
        draw_text_ex(
            "SERVER ADDRESS",
            fx,
            ip_y,
            text_params(13, if ip_active { ACCENT } else { TEXT_LABEL }),
        );
        let ip_fy = ip_y + 10.0;
        self.draw_input(
            fx,
            ip_fy,
            field_w,
            field_h,
            &self.ip_input.clone(),
            ip_active,
            "127.0.0.1:7878",
        );
        if in_rect(mouse, fx, ip_fy, field_w, field_h) && clicked {
            self.active_field = LoginField::Ip;
        }

        // ── username field ──
        let nm_y = ip_fy + field_h + 26.0;
        let nm_active = matches!(self.active_field, LoginField::Username);
        draw_text_ex(
            "USERNAME",
            fx,
            nm_y,
            text_params(13, if nm_active { ACCENT } else { TEXT_LABEL }),
        );
        let nm_fy = nm_y + 10.0;
        self.draw_input(
            fx,
            nm_fy,
            field_w,
            field_h,
            &self.username_input.clone(),
            nm_active,
            "Player",
        );
        if in_rect(mouse, fx, nm_fy, field_w, field_h) && clicked {
            self.active_field = LoginField::Username;
        }

        // ── model quality toggle ──
        let mq_y = nm_fy + field_h + 18.0;
        let mq_label = format!("Model Quality: {}", self.model_quality.label());
        let mq_w = measure_game_text(&mq_label, 14) + 24.0;
        let mq_h = 28.0;
        let mq_x = px + (pw - mq_w) / 2.0;
        let mq_hover = in_rect(mouse, mq_x, mq_y, mq_w, mq_h);
        draw_rectangle(mq_x, mq_y, mq_w, mq_h, if mq_hover { Color::new(0.14, 0.16, 0.26, 1.0) } else { Color::new(0.08, 0.10, 0.18, 1.0) });
        draw_rectangle_lines(mq_x, mq_y, mq_w, mq_h, 1.0, INPUT_BORDER);
        let mq_tw = measure_game_text(&mq_label, 14);
        draw_text_ex(&mq_label, mq_x + (mq_w - mq_tw) / 2.0, mq_y + 19.0, text_params(14, TEXT_LABEL));
        if mq_hover && clicked {
            self.model_quality = self.model_quality.toggle();
        }

        // ── connect button ──
        let bw = 220.0;
        let bh = 42.0;
        let bx = px + (pw - bw) / 2.0;
        let by = mq_y + mq_h + 14.0;
        let hover = in_rect(mouse, bx, by, bw, bh);
        draw_rectangle(bx, by, bw, bh, if hover { ACCENT_HOVER } else { ACCENT });
        let ctxt = "CONNECT";
        let ctw = measure_game_text(ctxt, 20);
        draw_text_ex(
            ctxt,
            bx + (bw - ctw) / 2.0,
            by + 28.0,
            text_params(20, Color::new(0.05, 0.05, 0.05, 1.0)),
        );
        if hover && clicked {
            self.try_connect();
        }

        // ── error message ──
        if !self.error_msg.is_empty() {
            let ey = by + bh + 16.0;
            let ew = measure_game_text(&self.error_msg, 15);
            draw_text_ex(
                &self.error_msg,
                px + (pw - ew) / 2.0,
                ey,
                text_params(15, Color::new(1.0, 0.30, 0.30, 1.0)),
            );
        }
    }

    // ── servers tab ────────────────────────────────────────────────────────
    fn draw_servers_tab(&mut self, px: f32, cy: f32, pw: f32, ch: f32, pad: f32) {
        let mouse = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let scanning = self.scan_timer > 0.0;

        // ── column header bar ──
        let hdr_y = cy + 10.0;
        let hdr_h = 30.0;
        let lx = px + pad;
        let lw = pw - pad * 2.0;
        draw_rectangle(lx, hdr_y, lw, hdr_h, Color::new(0.08, 0.09, 0.16, 1.0));
        draw_rectangle(
            lx,
            hdr_y + hdr_h - 1.0,
            lw,
            1.0,
            Color::new(ACCENT.r, ACCENT.g, ACCENT.b, 0.25),
        );

        // column labels
        let col_pad = 14.0;
        draw_text_ex(
            "SERVER",
            lx + col_pad,
            hdr_y + 20.0,
            text_params(11, TEXT_DIM),
        );
        let players_label = "PLAYERS";
        let plw = measure_game_text(players_label, 11);
        draw_text_ex(
            players_label,
            lx + lw - plw - col_pad - 90.0,
            hdr_y + 20.0,
            text_params(11, TEXT_DIM),
        );
        let addr_label = "ADDRESS";
        let alw = measure_game_text(addr_label, 11);
        draw_text_ex(
            addr_label,
            lx + lw - alw - col_pad,
            hdr_y + 20.0,
            text_params(11, TEXT_DIM),
        );

        // ── server list ──
        let ly = hdr_y + hdr_h;
        let lh = ch - (ly - cy) - 44.0;
        draw_rectangle(lx, ly, lw, lh, INPUT_BG);
        draw_rectangle_lines(lx, hdr_y, lw, hdr_h + lh, 1.0, INPUT_BORDER);

        if self.discovered_servers.is_empty() {
            let empty = if scanning {
                "Searching local network..."
            } else {
                "No servers on local network"
            };
            let ew = measure_game_text(empty, 14);
            draw_text_ex(
                empty,
                px + (pw - ew) / 2.0,
                ly + lh / 2.0 + 4.0,
                text_params(14, TEXT_DIM),
            );
        } else {
            let row_h = 52.0;
            let row_gap = 1.0;
            let mut ry = ly;
            let mut click_idx: Option<usize> = None;

            for (i, server) in self.discovered_servers.iter().enumerate() {
                if ry + row_h > ly + lh {
                    break;
                }

                let hover = in_rect(mouse, lx, ry, lw, row_h);

                // alternating rows with hover highlight
                let bg = if hover {
                    ROW_HOVER
                } else if i % 2 == 0 {
                    ROW_BG
                } else {
                    Color::new(0.05, 0.06, 0.12, 1.0)
                };
                draw_rectangle(lx, ry, lw, row_h, bg);

                // left accent bar on hover
                if hover {
                    draw_rectangle(lx, ry, 3.0, row_h, ACCENT);
                }

                // server name (bold, larger)
                let text_x = lx + col_pad + (if hover { 4.0 } else { 0.0 });
                draw_text_ex(
                    &server.name,
                    text_x,
                    ry + 22.0,
                    text_params(
                        16,
                        if hover {
                            WHITE
                        } else {
                            Color::new(0.88, 0.88, 0.95, 1.0)
                        },
                    ),
                );

                // ping dot (visual flair - green = reachable)
                draw_circle(
                    text_x + measure_game_text(&server.name, 16) + 8.0,
                    ry + 18.0,
                    3.0,
                    Color::new(0.3, 0.9, 0.4, 0.8),
                );

                // address (bottom-left, smaller)
                draw_text_ex(&server.addr, text_x, ry + 40.0, text_params(12, TEXT_DIM));

                // player count (right-aligned, with color coding)
                let pc = format!("{}/{}", server.players, server.max_players);
                let pcw = measure_game_text(&pc, 15);
                let pc_color = if server.players == 0 {
                    TEXT_DIM
                } else if server.players >= server.max_players {
                    Color::new(1.0, 0.35, 0.35, 1.0)
                } else {
                    Color::new(0.3, 0.9, 0.5, 1.0)
                };
                draw_text_ex(
                    &pc,
                    lx + lw - pcw - col_pad - 90.0,
                    ry + 30.0,
                    text_params(15, pc_color),
                );

                // address column (right)
                let addr_short = &server.addr;
                let aw = measure_game_text(addr_short, 12);
                draw_text_ex(
                    addr_short,
                    lx + lw - aw - col_pad,
                    ry + 30.0,
                    text_params(12, TEXT_LABEL),
                );

                // row separator
                draw_rectangle(
                    lx + 6.0,
                    ry + row_h - row_gap,
                    lw - 12.0,
                    row_gap,
                    Color::new(0.16, 0.18, 0.28, 0.5),
                );

                if hover && clicked {
                    click_idx = Some(i);
                }
                ry += row_h;
            }

            // handle server selection click
            if let Some(idx) = click_idx {
                self.ip_input = self.discovered_servers[idx].addr.clone();
                self.active_tab = MainMenuTab::Play;
            }
        }

        // ── bottom bar: status + refresh ──
        let bar_y = ly + lh + 8.0;

        // status text
        let dots = ".".repeat((self.anim_time * 3.0) as usize % 4);
        let status = if scanning {
            format!("Scanning{}", dots)
        } else if self.discovered_servers.is_empty() {
            "No servers found".to_string()
        } else {
            format!("{} server(s)", self.discovered_servers.len())
        };
        draw_text_ex(&status, lx, bar_y + 14.0, text_params(13, TEXT_DIM));

        // refresh button
        let rbw = 92.0;
        let rbh = 26.0;
        let rbx = lx + lw - rbw;
        let rby = bar_y;
        let rh = in_rect(mouse, rbx, rby, rbw, rbh);
        draw_rectangle(rbx, rby, rbw, rbh, if rh { ACCENT_HOVER } else { ACCENT });
        let rt = "REFRESH";
        let rtw = measure_game_text(rt, 13);
        draw_text_ex(
            rt,
            rbx + (rbw - rtw) / 2.0,
            rby + 18.0,
            text_params(13, Color::new(0.05, 0.05, 0.05, 1.0)),
        );
        if rh && clicked {
            self.start_scan();
        }
    }

    // ── input field helper ─────────────────────────────────────────────────
    fn draw_input(
        &self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: &str,
        active: bool,
        placeholder: &str,
    ) {
        draw_rectangle(x, y, w, h, INPUT_BG);
        let bc = if active { ACCENT } else { INPUT_BORDER };
        draw_rectangle_lines(x, y, w, h, if active { 2.0 } else { 1.0 }, bc);

        if text.is_empty() && !active {
            draw_text_ex(
                placeholder,
                x + 10.0,
                y + 23.0,
                text_params(17, Color::new(0.30, 0.32, 0.42, 0.5)),
            );
        } else {
            draw_text_ex(text, x + 10.0, y + 23.0, text_params(17, WHITE));
            if active {
                let visible = (self.anim_time * 2.5) as i32 % 2 == 0;
                if visible {
                    let tw = measure_game_text(text, 17);
                    draw_rectangle(x + 10.0 + tw + 2.0, y + 6.0, 2.0, h - 12.0, ACCENT);
                }
            }
        }
    }
}

// ── hit-test helper ────────────────────────────────────────────────────────
fn in_rect(mouse: (f32, f32), x: f32, y: f32, w: f32, h: f32) -> bool {
    mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h
}
