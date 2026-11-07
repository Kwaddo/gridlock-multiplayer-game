use crate::core::game_state::*;
use crate::net::browser::ServerBrowser;
use crate::net::client::{ConnectProgress, Connector};
use crate::ui::fonts::{measure_game_text, text_params};
use macroquad::prelude::*;
use macroquad::rand::gen_range;

mod background;
mod browse_tab;
mod direct_tab;
mod host_tab;
mod update;
mod widgets;

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
const ERROR: Color = Color::new(1.0, 0.30, 0.30, 1.0);

const TABS: [(MainMenuTab, &str); 3] = [
    (MainMenuTab::Browse, "BROWSE"),
    (MainMenuTab::Host, "HOST"),
    (MainMenuTab::Direct, "DIRECT"),
];

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

pub struct LoginScreen {
    pub username_input: String,
    pub address_input: String,
    pub server_name_input: String,
    pub list_online: bool,
    pub active_field: LoginField,
    pub error_msg: String,
    pub active_tab: MainMenuTab,
    browser: ServerBrowser,
    particles: Vec<MenuParticle>,
    anim_time: f32,
    connector: Option<Connector>,
}

impl LoginScreen {
    pub fn new() -> Self {
        let sw = screen_width().max(800.0);
        let sh = screen_height().max(600.0);
        let particles = (0..80).map(|_| MenuParticle::random(sw, sh)).collect();

        Self {
            username_input: String::new(),
            address_input: String::new(),
            server_name_input: String::new(),
            list_online: false,
            active_field: LoginField::Username,
            error_msg: String::new(),
            active_tab: MainMenuTab::Browse,
            browser: ServerBrowser::new(),
            particles,
            anim_time: 0.0,
            connector: None,
        }
    }

    pub fn draw(&mut self) {
        self.draw_background();
        self.draw_title();

        let sw = screen_width();
        let sh = screen_height();

        let panel_w = 560.0_f32.min(sw * 0.85);
        let panel_h = 470.0;
        let panel_x = (sw - panel_w) / 2.0;
        let panel_y = sh * 0.24;
        let pad = 36.0;
        draw_rectangle(panel_x, panel_y, panel_w, panel_h, PANEL_BG);
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, PANEL_BORDER);

        // Everyone needs a name, whichever way they get into a game.
        self.text_field(LoginField::Username, "USERNAME", "Player", panel_x + pad, panel_y + 26.0, panel_w - pad * 2.0);

        let tab_y = panel_y + 92.0;
        let tab_h = 38.0;
        self.draw_tabs(panel_x, tab_y, panel_w, tab_h);

        let cy = tab_y + tab_h + 1.0;
        let ch = panel_h - (cy - panel_y) - 34.0;
        match self.active_tab {
            MainMenuTab::Browse => self.draw_browse_tab(panel_x, cy, panel_w, ch, pad),
            MainMenuTab::Host => self.draw_host_tab(panel_x, cy, panel_w, ch, pad),
            MainMenuTab::Direct => self.draw_direct_tab(panel_x, cy, panel_w, ch, pad),
        }

        if !self.error_msg.is_empty() {
            let ew = measure_game_text(&self.error_msg, 15);
            let ey = panel_y + panel_h - 14.0;
            draw_text_ex(&self.error_msg, panel_x + (panel_w - ew) / 2.0, ey, text_params(15, ERROR));
        }

        draw_text_ex(
            "F11 fullscreen   |   TAB switch fields   |   ENTER confirm",
            panel_x,
            panel_y + panel_h + 16.0,
            text_params(13, TEXT_DIM),
        );

        if let Some(connector) = &self.connector {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.55));
            draw_connecting(&connector.progress(), sw, sh);
        }
    }

    fn draw_tabs(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let tab_w = w / TABS.len() as f32;
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        for (i, (tab, label)) in TABS.into_iter().enumerate() {
            let tx = x + tab_w * i as f32;
            if clicked && in_rect(mouse_position(), tx, y, tab_w, h) {
                self.select_tab(tab);
            }
            let active = self.active_tab == tab;
            if active {
                draw_rectangle(tx, y, tab_w, h, TAB_ACTIVE_BG);
                draw_rectangle(tx, y + h - 2.0, tab_w, 2.0, ACCENT);
            }
            let lw = measure_game_text(label, 18);
            let color = if active { WHITE } else { TAB_INACTIVE_TEXT };
            draw_text_ex(label, tx + (tab_w - lw) / 2.0, y + 26.0, text_params(18, color));
        }
        draw_rectangle(x, y, w, 1.0, Color::new(0.20, 0.24, 0.40, 0.5));
        draw_rectangle(x, y + h, w, 1.0, Color::new(0.20, 0.24, 0.40, 0.5));
    }

    fn select_tab(&mut self, tab: MainMenuTab) {
        self.active_tab = tab;
        self.error_msg.clear();
        // keep focus on a field that is visible on the new tab
        if !self.tab_fields().contains(&self.active_field) {
            self.active_field = LoginField::Username;
        }
    }

    /// Fields on screen for the current tab, in TAB order.
    fn tab_fields(&self) -> &'static [LoginField] {
        match self.active_tab {
            MainMenuTab::Browse => &[LoginField::Username],
            MainMenuTab::Host => &[LoginField::Username, LoginField::ServerName],
            MainMenuTab::Direct => &[LoginField::Username, LoginField::Address],
        }
    }
}

/// The "Connecting" panel: current step, an estimated progress bar, and how
/// long the join has taken and may still take before it gives up.
fn draw_connecting(p: &ConnectProgress, sw: f32, sh: f32) {
    let pw = 480.0_f32.min(sw * 0.85);
    let ph = 176.0;
    let px = (sw - pw) / 2.0;
    let py = sh * 0.38;
    let pad = 30.0;
    draw_rectangle(px, py, pw, ph, PANEL_BG);
    draw_rectangle_lines(px, py, pw, ph, 1.0, PANEL_BORDER);

    let centered = |text: &str, y: f32, size: u16, color: Color| {
        let w = measure_game_text(text, size);
        draw_text_ex(text, (sw - w) / 2.0, y, text_params(size, color));
    };

    // dots cycle so the panel visibly lives even when the bar barely moves
    let dots = ".".repeat(1 + (p.elapsed.as_millis() / 400 % 3) as usize);
    centered(&format!("CONNECTING{dots:<3}"), py + 34.0, 22, ACCENT);
    centered(&format!("Step {} of {}  -  {}", p.step, p.steps, p.stage), py + 60.0, 16, WHITE);

    let bx = px + pad;
    let bw = pw - pad * 2.0;
    let bh = 22.0;
    let by = py + 76.0;
    draw_rectangle(bx, by, bw, bh, INPUT_BG);
    draw_rectangle_lines(bx, by, bw, bh, 1.0, PANEL_BORDER);
    let frac = p.fraction.clamp(0.0, 1.0);
    draw_rectangle(bx + 2.0, by + 2.0, (bw - 4.0) * frac, bh - 4.0, ACCENT);
    let pct = format!("{}%", (frac * 100.0).round() as u32);
    let pct_w = measure_game_text(&pct, 14);
    draw_text_ex(&pct, bx + (bw - pct_w) / 2.0, by + 16.0, text_params(14, WHITE));

    // long room URLs keep their distinctive end
    let chars: Vec<char> = p.address.chars().collect();
    let address = if chars.len() > 40 {
        format!("...{}", chars[chars.len() - 37..].iter().collect::<String>())
    } else {
        p.address.to_string()
    };
    draw_text_ex(&address, bx, by + bh + 20.0, text_params(13, TEXT_DIM));
    let elapsed = format!("{:.1}s", p.elapsed.as_secs_f32());
    let ew = measure_game_text(&elapsed, 15);
    draw_text_ex(&elapsed, bx + bw - ew, by + bh + 20.0, text_params(15, TEXT_LABEL));

    let gives_up = p.gives_up_in.as_secs_f32().ceil() as u32;
    let footer = if p.join_attempts > 1 {
        format!("Join request sent {}x  |  gives up in {}s", p.join_attempts, gives_up)
    } else {
        format!("Gives up in {}s if there is no answer", gives_up)
    };
    centered(&footer, py + ph - 16.0, 13, TEXT_DIM);
}

fn in_rect(mouse: (f32, f32), x: f32, y: f32, w: f32, h: f32) -> bool {
    mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h
}
