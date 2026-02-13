use crate::core::GameConfig;
use crate::physics::movement::MovementConfig;
use crate::ui::fonts::{ measure_game_text, text_params };
use crate::weapons::crosshair::CrosshairConfig;
use crate::weapons::state::WeaponInventory;
use macroquad::prelude::*;

const DEV_CODE: &str = "verso";

// DevModeState
pub struct DevModeState {
    pub active_tab: usize,
    pub scroll_offset: f32,

    // debug toggles (persists upon open/closing dev mode)
    pub infinite_ammo: bool,
    pub no_recoil: bool,
    pub show_hitboxes: bool,
    pub dummy_invincible: bool,
    pub god_mode: bool,

    // code prompt
    pub code_input: String,
    pub code_verified: bool,
    pub show_code_prompt: bool,
}

impl DevModeState {
    pub fn new() -> Self {
        Self {
            active_tab: 0,
            scroll_offset: 0.0,
            infinite_ammo: false,
            no_recoil: false,
            show_hitboxes: false,
            dummy_invincible: false,
            god_mode: false,
            code_input: String::new(),
            code_verified: false,
            show_code_prompt: false,
        }
    }

    pub fn request_open(&mut self) {
        if self.code_verified {
            self.show_code_prompt = false;
        } else {
            self.code_input.clear();
            self.show_code_prompt = true;
            // drain stale character queue so keys typed before opening don't pre-fill
            while get_char_pressed().is_some() {}
        }
    }

    pub fn is_unlocked(&self) -> bool {
        self.code_verified && !self.show_code_prompt
    }

    // CODE PROMPT MODAL

    pub fn update_code_prompt(state: &mut DevModeState) -> bool {
        // if prompt not open, ignore input
        if !state.show_code_prompt {
            state.code_input.clear();
            return false;
        }

        // collect typed input
        while let Some(c) = get_char_pressed() {

            if c.is_alphanumeric() {
                state.code_input.push(c);
            }
        }

        // cap input length to prevent overflow
        if state.code_input.len() > 20 {
            state.code_input.truncate(20);
        }

        // backspace
        if is_key_pressed(KeyCode::Backspace) {
            state.code_input.pop();
        }

        // esc to cancel
        if is_key_pressed(KeyCode::Escape) {
            state.show_code_prompt = false;
            return false;
        }

        // enter to submit

        if is_key_pressed(KeyCode::Enter) {
            if state.code_input == DEV_CODE {
                state.code_verified = true;
                state.show_code_prompt = false;
                return true;
            } else {
                // shake input or show error (not implemented)
                state.code_input.clear();
            }
        }
        false
    }

    pub fn draw_code_prompt(state: &DevModeState) {
        if !state.show_code_prompt {
            return;
        }

        let sw = screen_width();
        let sh = screen_height();

        // dark semi-transparent background
        draw_rectangle(0.0, 0.0, sw, sh, Color::from_rgba(0, 0, 0, 180));

        // prompt box

        let bw = 360.0;
        let bh = 180.0;
        let bx = (sw - bw) / 2.0;
        let by = (sh - bh) / 2.0;

        draw_rectangle(bx, by, bw, bh, Color::from_rgba(30, 30, 30, 255));
        draw_rectangle_lines(bx, by, bw, bh, 2.0, Color::from_rgba(80, 150, 220, 255));

        draw_text_ex("Enter Code", bx + 90.0, by + 35.0, text_params(22, WHITE));

        // input box
        let ix = bx + 30.0;
        let iy = by + 60.0;
        let iw = bw - 60.0;
        let ih = 36.0;
        draw_rectangle(ix, iy, iw, ih, Color::from_rgba(15, 15, 15, 255));
        draw_rectangle_lines(ix, iy, iw, ih, 1.0, GRAY);

        // show typed code as dots
        let dots: String = std::iter::repeat('*').take(state.code_input.len()).collect();
        draw_text_ex(&dots, ix + 10.0, iy + 24.0, text_params(22, WHITE));

        draw_text_ex(
            "Press ENTER to SUBMIT, ESC to CANCEL",
            bx + 40.0,
            by + 130.0,
            text_params(14, GRAY)
        );
    }

    // TAB NAMES

    const TAB_NAMES: &[&str] = &["Weapon", "Crosshair", "Movement", "Debug"];

    // Slider helper
    fn draw_slider(
        label: &str,
        value: &mut f32,
        min: f32,
        max: f32,
        step: f32,
        x: f32,
        y: f32,
        width: f32,
        mouse: (f32, f32),
        _clicked: bool,
        held: bool
    ) -> f32 {
        let label_w = 200.0;
        let slider_x = x + label_w;
        let slider_w = width - label_w - 80.0;
        let slider_h = 16.0;
        let row_h = 28.0;

        // label
        draw_text_ex(label, x, y + 20.0, text_params(18, WHITE));

        // slider tracker
        draw_rectangle(slider_x, y + 2.0, slider_w, slider_h, Color::from_rgba(40, 40, 40, 255));
        let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
        draw_rectangle(
            slider_x,
            y + 2.0,
            slider_w * t,
            slider_h,
            Color::from_rgba(80, 150, 220, 255)
        );
        draw_rectangle_lines(slider_x, y + 2.0, slider_w, slider_h, 1.0, GRAY);

        // drag
        if
            held &&
            mouse.1 >= y &&
            mouse.1 <= y + row_h &&
            mouse.0 >= slider_x &&
            mouse.0 <= slider_x + slider_w
        {
            let raw = min + ((mouse.0 - slider_x) / slider_w) * (max - min);
            *value = (raw / step).round() * step;
            *value = value.clamp(min, max);
        }

        // value text
        let precision = if step >= 1.0 {
            0
        } else if step >= 0.1 {
            1
        } else if step >= 0.01 {
            2
        } else {
            3
        };

        let txt = format!("{:.*}", precision, *value);
        draw_text_ex(&txt, slider_x + slider_w + 8.0, y + 14.0, text_params(14, WHITE));

        row_h
    }

    fn draw_slider_u32(
        label: &str,
        value: &mut u32,
        min: u32,
        max: u32,
        step: u32,
        x: f32,
        y: f32,
        w: f32,
        mouse: (f32, f32),
        clicked: bool,
        held: bool
    ) -> f32 {
        let mut fval = *value as f32;
        let h = Self::draw_slider(
            label,
            &mut fval,
            min as f32,
            max as f32,
            step as f32,
            x,
            y,
            w,
            mouse,
            clicked,
            held
        );
        *value = fval as u32;
        h
    }
    fn draw_toggle(
        label: &str,
        value: &mut bool,
        x: f32,
        y: f32,
        mouse: (f32, f32),
        clicked: bool
    ) -> f32 {
        let box_size = 16.0;
        let row_h = 28.0;

        // checkbox
        let bx = x;
        let by = y + 2.0;
        draw_rectangle(bx, by, box_size, box_size, Color::from_rgba(40, 40, 40, 255));
        draw_rectangle_lines(bx, by, box_size, box_size, 1.0, GRAY);
        if *value {
            draw_rectangle(
                bx + 3.0,
                by + 3.0,
                box_size - 6.0,
                box_size - 6.0,
                Color::from_rgba(80, 220, 120, 255)
            );
        }

        // click
        if
            clicked &&
            mouse.0 >= bx &&
            mouse.0 <= bx + box_size + 200.0 &&
            mouse.1 >= y &&
            mouse.1 <= y + row_h
        {
            *value = !*value;
        }

        // label
        draw_text_ex(label, x + box_size + 8.0, y + 14.0, text_params(14, WHITE));

        row_h
    }
    fn draw_section_header(label: &str, x: f32, y: f32) -> f32 {
        draw_text_ex(label, x, y + 16.0, text_params(16, Color::from_rgba(80, 150, 220, 255)));
        draw_line(x, y + 22.0, x + 300.0, y + 22.0, 1.0, Color::from_rgba(80, 150, 220, 100));
        28.0
    }

    fn draw_action_button(
        label: &str,
        x: f32,
        y: f32,
        w: f32,
        mouse: (f32, f32),
        clicked: bool
    ) -> (f32, bool) {
        let h = 28.0;
        let hover = mouse.0 >= x && mouse.0 <= x + w && mouse.1 >= y && mouse.1 <= y + h;
        let color = if hover {
            Color::from_rgba(60, 60, 60, 255)
        } else {
            Color::from_rgba(40, 40, 40, 255)
        };
        draw_rectangle(x, y, w, h, color);
        draw_rectangle_lines(x, y, w, h, 1.0, GRAY);
        let tw = measure_game_text(label, 14);
        draw_text_ex(label, x + (w - tw) / 2.0, y + 18.0, text_params(14, WHITE));
        (32.0, hover && clicked)
    }
}

// --- Main overlay 
pub fn draw_dev_overlay(
    dev: &mut DevModeState,
    inventory: &mut WeaponInventory,
    move_config: &mut MovementConfig,
    _game_config: &mut GameConfig,
) {
    let sw = screen_width();
    let sh = screen_height();
    let mouse = mouse_position();
    let clicked = is_mouse_button_pressed(MouseButton::Left);
    let held = is_mouse_button_down(MouseButton::Left);

    // panel on the right side
    let panel_w = 460.0;
    let panel_x = sw - panel_w - 10.0;
    let panel_y = 10.0;
    let panel_h = sh - 20.0;

    // background
    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::from_rgba(20, 20, 20, 230));
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, Color::from_rgba(80, 150, 220, 255));

    // title
    draw_text_ex("DEV MODE", panel_x + 10.0, panel_y + 24.0, text_params(20, Color::from_rgba(80, 150, 220, 255)));
    draw_text_ex("F3 to close", panel_x + panel_w - 100.0, panel_y + 24.0, text_params(12, GRAY));

    // tab buttons
    let tab_y = panel_y + 36.0;
    let tab_w = (panel_w - 20.0) / DevModeState::TAB_NAMES.len() as f32;
    for (i, name) in DevModeState::TAB_NAMES.iter().enumerate() {
        let tx = panel_x + 10.0 + i as f32 * tab_w;
        let active = i == dev.active_tab;
        let color = if active {
            Color::from_rgba(80, 150, 220, 255)
        } else if mouse.0 >= tx && mouse.0 <= tx + tab_w && mouse.1 >= tab_y && mouse.1 <= tab_y + 26.0 {
            Color::from_rgba(60, 60, 60, 255)
        } else {
            Color::from_rgba(40, 40, 40, 255)
        };
        draw_rectangle(tx, tab_y, tab_w - 2.0, 26.0, color);
        let tw = measure_game_text(name, 14);
        draw_text_ex(name, tx + (tab_w - 2.0 - tw) / 2.0, tab_y + 18.0, text_params(14, WHITE));

        if clicked && mouse.0 >= tx && mouse.0 <= tx + tab_w && mouse.1 >= tab_y && mouse.1 <= tab_y + 26.0 {
            dev.active_tab = i;
        }
    }

    // content area
    let content_x = panel_x + 16.0;
    let content_y = tab_y + 36.0;
    let content_w = panel_w - 32.0;

    match dev.active_tab {
        0 => draw_weapon_tab(inventory, content_x, content_y, content_w, mouse, clicked, held),
        1 => draw_crosshair_tab(inventory, content_x, content_y, content_w, mouse, clicked, held),
        2 => draw_movement_tab(move_config, content_x, content_y, content_w, mouse, clicked, held),
        3 => draw_debug_tab(dev, inventory, content_x, content_y, content_w, mouse, clicked, held),
        _ => {}
    }
}

// --- Tab 0: Weapon Stats 
fn draw_weapon_tab(
    inv: &mut WeaponInventory,
    x: f32, mut y: f32, w: f32,
    mouse: (f32, f32), clicked: bool, held: bool,
) {
    let weapon = inv.active_mut();
    let name = weapon.stats_data.name;
    draw_text_ex(&format!("Active: {}", name), x, y + 14.0, text_params(16, Color::from_rgba(255, 200, 80, 255)));
    y += 24.0;

    y += DevModeState::draw_section_header("Damage", x, y);
    y += DevModeState::draw_slider("damage", &mut weapon.stats_data.damage, 1.0, 200.0, 1.0, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("headshot_mult", &mut weapon.stats_data.headshot_mult, 1.0, 5.0, 0.1, x, y, w, mouse, clicked, held);

    y += DevModeState::draw_section_header("Fire", x, y);
    y += DevModeState::draw_slider("fire_rate", &mut weapon.stats_data.fire_rate, 0.5, 20.0, 0.5, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider_u32("pellet_count", &mut weapon.stats_data.pellet_count, 1, 20, 1, x, y, w, mouse, clicked, held);

    y += DevModeState::draw_section_header("Ammo", x, y);
    y += DevModeState::draw_slider_u32("magazine_size", &mut weapon.stats_data.magazine_size, 1, 100, 1, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider_u32("max_reserve", &mut weapon.stats_data.max_reserve, 0, 500, 10, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("reload_time", &mut weapon.stats_data.reload_time, 0.1, 5.0, 0.1, x, y, w, mouse, clicked, held);

    y += DevModeState::draw_section_header("Recoil", x, y);
    y += DevModeState::draw_slider("recoil_vertical", &mut weapon.stats_data.recoil_vertical, 0.0, 0.2, 0.005, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("recoil_horizontal", &mut weapon.stats_data.recoil_horizontal, 0.0, 0.1, 0.002, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("recoil_recovery", &mut weapon.stats_data.recoil_recovery, 0.5, 10.0, 0.5, x, y, w, mouse, clicked, held);

    y += DevModeState::draw_section_header("Spread", x, y);
    y += DevModeState::draw_slider("hip_spread", &mut weapon.stats_data.hip_spread, 0.0, 0.2, 0.005, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("ads_spread", &mut weapon.stats_data.ads_spread, 0.0, 0.1, 0.001, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("spread_per_shot", &mut weapon.stats_data.spread_per_shot, 0.0, 0.1, 0.005, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("spread_recovery", &mut weapon.stats_data.spread_recovery, 0.1, 5.0, 0.1, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("move_spread_mult", &mut weapon.stats_data.move_spread_mult, 1.0, 3.0, 0.1, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("sprint_spread_mult", &mut weapon.stats_data.sprint_spread_mult, 1.0, 5.0, 0.1, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("crouch_spread_mult", &mut weapon.stats_data.crouch_spread_mult, 0.1, 1.0, 0.05, x, y, w, mouse, clicked, held);

    y += DevModeState::draw_section_header("ADS", x, y);
    y += DevModeState::draw_slider("ads_zoom_fov", &mut weapon.stats_data.ads_zoom_fov, 10.0, 90.0, 1.0, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("ads_time", &mut weapon.stats_data.ads_time, 0.01, 1.0, 0.01, x, y, w, mouse, clicked, held);

    y += DevModeState::draw_section_header("Bullet", x, y);
    y += DevModeState::draw_slider("bullet_range", &mut weapon.stats_data.bullet_range, 5.0, 200.0, 5.0, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("tracer_length", &mut weapon.stats_data.tracer_length, 0.5, 15.0, 0.5, x, y, w, mouse, clicked, held);

    y += DevModeState::draw_section_header("Falloff", x, y);
    y += DevModeState::draw_slider("falloff_start", &mut weapon.stats_data.falloff_start, 1.0, 100.0, 1.0, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("falloff_end", &mut weapon.stats_data.falloff_end, 5.0, 200.0, 5.0, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("falloff_min_mult", &mut weapon.stats_data.falloff_min_mult, 0.0, 1.0, 0.05, x, y, w, mouse, clicked, held);

    // reset button
    y += 8.0;
    let (_h, pressed) = DevModeState::draw_action_button("Reset Weapon to Defaults", x, y, 240.0, mouse, clicked);
    if pressed {
        inv.active_mut().reset_stats();
    }
}

// ── Tab 1: Crosshair ────────────────────────────────────────────────────
fn draw_crosshair_tab(
    inv: &mut WeaponInventory,
    x: f32, mut y: f32, w: f32,
    mouse: (f32, f32), clicked: bool, held: bool,
) {
    let weapon = inv.active_mut();
    let name = weapon.stats_data.name;
    draw_text_ex(&format!("Crosshair: {}", name), x, y + 14.0, text_params(16, Color::from_rgba(255, 200, 80, 255)));
    y += 24.0;

    let ch = &mut weapon.crosshair;

    y += DevModeState::draw_section_header("Shape", x, y);
    y += DevModeState::draw_slider("center_gap", &mut ch.center_gap, 0.0, 20.0, 0.5, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("line_length", &mut ch.line_length, 0.0, 30.0, 0.5, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("line_thickness", &mut ch.line_thickness, 0.0, 5.0, 0.1, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_toggle("dot_enabled", &mut ch.dot_enabled, x, y, mouse, clicked);
    y += DevModeState::draw_slider("dot_size", &mut ch.dot_size, 0.0, 8.0, 0.05, x, y, w, mouse, clicked, held);

    y += DevModeState::draw_section_header("Kick", x, y);
    y += DevModeState::draw_slider("kick_per_shot", &mut weapon.stats_data.crosshair_kick_per_shot, 0.0, 10.0, 0.5, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("kick_decay_rate", &mut weapon.stats_data.crosshair_kick_decay, 1.0, 20.0, 1.0, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("kick_max", &mut weapon.stats_data.crosshair_kick_max, 0.0, 30.0, 1.0, x, y, w, mouse, clicked, held);

    y += DevModeState::draw_section_header("Color (RGB)", x, y);
    let mut r = ch.color.r * 255.0;
    let mut g = ch.color.g * 255.0;
    let mut b = ch.color.b * 255.0;
    y += DevModeState::draw_slider("Red", &mut r, 0.0, 255.0, 5.0, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("Green", &mut g, 0.0, 255.0, 5.0, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("Blue", &mut b, 0.0, 255.0, 5.0, x, y, w, mouse, clicked, held);
    ch.color = Color::from_rgba(r as u8, g as u8, b as u8, 255);

    y += DevModeState::draw_section_header("Outline", x, y);
    y += DevModeState::draw_toggle("outline_enabled", &mut ch.outline_enabled, x, y, mouse, clicked);
    y += DevModeState::draw_slider("outline_thickness", &mut ch.outline_thickness, 0.0, 3.0, 0.1, x, y, w, mouse, clicked, held);

    use crate::weapons::crosshair::CrosshairType;
    let type_label = match ch.crosshair_type {
        CrosshairType::Cross => "Cross",
        CrosshairType::FourCurves => "4-Curves",
        CrosshairType::Chevron => "Chevron",
        CrosshairType::Circle => "Circle",
        CrosshairType::Square => "Square",
        CrosshairType::Triangle => "Triangle",
        CrosshairType::AngleBrackets => "Angle Brackets",
    };
    let (btn_h, pressed) = DevModeState::draw_action_button(&format!("Type: {}", type_label), x, y, 200.0, mouse, clicked);
    if pressed {
        ch.crosshair_type = match ch.crosshair_type {
            CrosshairType::Cross => CrosshairType::FourCurves,
            CrosshairType::FourCurves => CrosshairType::Chevron,
            CrosshairType::Chevron => CrosshairType::Circle,
            CrosshairType::Circle => CrosshairType::Square,
            CrosshairType::Square => CrosshairType::Triangle,
            CrosshairType::Triangle => CrosshairType::AngleBrackets,
            CrosshairType::AngleBrackets => CrosshairType::Cross,
        };
    }
    y += btn_h;

    // reset
    y += 8.0;
    let kind = weapon.kind;
    let (_h, pressed) = DevModeState::draw_action_button("Reset Crosshair to Defaults", x, y, 260.0, mouse, clicked);
    if pressed {
        inv.active_mut().crosshair = CrosshairConfig::for_weapon(kind);
    }
}

// ── Tab 2: Movement ─────────────────────────────────────────────────────
fn draw_movement_tab(
    mov: &mut MovementConfig,
    x: f32, mut y: f32, w: f32,
    mouse: (f32, f32), clicked: bool, held: bool,
) {
    y += DevModeState::draw_section_header("Speed", x, y);
    y += DevModeState::draw_slider("walk_speed", &mut mov.walk_speed, 1.0, 15.0, 0.5, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("sprint_speed", &mut mov.sprint_speed, 2.0, 25.0, 0.5, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("crouch_speed", &mut mov.crouch_speed, 0.5, 8.0, 0.5, x, y, w, mouse, clicked, held);

    y += DevModeState::draw_section_header("Physics", x, y);
    y += DevModeState::draw_slider("acceleration", &mut mov.acceleration, 1.0, 30.0, 1.0, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("friction", &mut mov.friction, 1.0, 20.0, 1.0, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("gravity", &mut mov.gravity, 5.0, 40.0, 1.0, x, y, w, mouse, clicked, held);
    y += DevModeState::draw_slider("jump_strength", &mut mov.jump_strength, 1.0, 15.0, 0.5, x, y, w, mouse, clicked, held);

    y += DevModeState::draw_section_header("Stamina", x, y);
    y += DevModeState::draw_slider("stamina_drain", &mut mov.stamina_drain, 5.0, 100.0, 5.0, x, y, w, mouse, clicked, held);
    DevModeState::draw_slider("stamina_recover", &mut mov.stamina_recover, 5.0, 50.0, 5.0, x, y, w, mouse, clicked, held);
}

// ── Tab 3: Debug ─────────────────────────────────────────────────────────
fn draw_debug_tab(
    dev: &mut DevModeState,
    inv: &mut WeaponInventory,
    x: f32, mut y: f32, _w: f32,
    mouse: (f32, f32), clicked: bool, _held: bool,
) {
    y += DevModeState::draw_section_header("Toggles", x, y);
    y += DevModeState::draw_toggle("Infinite Ammo", &mut dev.infinite_ammo, x, y, mouse, clicked);
    y += DevModeState::draw_toggle("No Recoil", &mut dev.no_recoil, x, y, mouse, clicked);
    y += DevModeState::draw_toggle("God Mode", &mut dev.god_mode, x, y, mouse, clicked);
    y += DevModeState::draw_toggle("Show Hitboxes", &mut dev.show_hitboxes, x, y, mouse, clicked);
    y += DevModeState::draw_toggle("Dummy Invincible", &mut dev.dummy_invincible, x, y, mouse, clicked);

    y += DevModeState::draw_section_header("Actions", x, y);

    let (_h, pressed) = DevModeState::draw_action_button("Refill Ammo", x, y, 160.0, mouse, clicked);
    y += _h;
    if pressed {
        let w = inv.active_mut();
        w.ammo = w.stats_data.magazine_size;
        w.reserve = w.stats_data.max_reserve;
    }

    let (_h, pressed) = DevModeState::draw_action_button("Reset All Weapons", x, y, 180.0, mouse, clicked);
    if pressed {
        for w in &mut inv.weapons {
            w.reset_stats();
            w.crosshair = CrosshairConfig::for_weapon(w.kind);
        }
    }
}
