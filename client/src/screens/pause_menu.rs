use crate::core::GameConfig;
use crate::core::GameState;
use crate::screens::dev_mode::DevModeState;
use crate::settings::SettingsScreen;
use crate::ui::fonts::{ measure_game_text, text_params };
use crate::ui::is_button_clicked;
use macroquad::prelude::*;

pub fn draw_pause_menu(
    config: &GameConfig,
    settings_screen: &mut SettingsScreen,
    dev_mode: &mut DevModeState
) -> Option<GameState> {
    
    // if code prompt open, ONLY handle input for that (ignore all other pause menu input including mouse clicks)
    if dev_mode.show_code_prompt {
        // still draw the pause menu visuals behind it
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(0, 0, 0, 150));
        
        let panel_w = 300.0;
        let panel_h = 310.0;
        let panel_x = (screen_width() - panel_w) / 2.0;
        let panel_y = (screen_height() - panel_h) / 2.0;
        draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::from_rgba(30, 30, 30, 240));
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, WHITE);
        
        let title = "PAUSED";
        let title_w = measure_game_text(title, 30);
        draw_text_ex(
            title,
            (screen_width() - title_w) / 2.0,
            panel_y + 40.0,
            text_params(30, WHITE)
        );
        
        if DevModeState::update_code_prompt(dev_mode) {
            return Some(GameState::DevMode);
        }
        DevModeState::draw_code_prompt(dev_mode);
        return None; // Block all button interaction
    }
    
    // ESC to resume (is_key_pressed is consumed by the Playing state's check,
    // so this only fires on a *second* ESC press while paused as to not immediately unpause when you open the menu)
    if is_key_pressed(config.key_settings) {
        return Some(GameState::Playing);
    }

    // Semi-transparent overlay on top of the game world
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(0, 0, 0, 150));

    let panel_w = 300.0;
    let panel_h = 310.0;
    let panel_x = (screen_width() - panel_w) / 2.0;
    let panel_y = (screen_height() - panel_h) / 2.0;
    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::from_rgba(30, 30, 30, 240));
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, WHITE);

    let title = "PAUSED";
    let title_w = measure_game_text(title, 30);
    draw_text_ex(title, (screen_width() - title_w) / 2.0, panel_y + 40.0, text_params(30, WHITE));

    let btn_w = 200.0;
    let btn_h = 40.0;
    let btn_x = (screen_width() - btn_w) / 2.0;
    let btn_color = Color::from_rgba(50, 50, 50, 255);
    let btn_hover = DARKGRAY;

    if is_button_clicked(btn_x, panel_y + 80.0, btn_w, btn_h, "Resume", btn_color, btn_hover) {
        return Some(GameState::Playing);
    }

    if is_button_clicked(btn_x, panel_y + 130.0, btn_w, btn_h, "Settings", btn_color, btn_hover) {
        settings_screen.enter(config);
        return Some(GameState::Settings);
    }

    if
        is_button_clicked(
            btn_x,
            panel_y + 180.0,
            btn_w,
            btn_h,
            "Dev Mode",
            Color::from_rgba(40, 60, 80, 255),
            Color::from_rgba(60, 90, 120, 255)
        )
    {
        dev_mode.request_open();
        if dev_mode.is_unlocked() {
            return Some(GameState::DevMode);
        }
    }

    if
        is_button_clicked(
            btn_x,
            panel_y + 250.0,
            btn_w,
            btn_h,
            "Exit",
            Color::from_rgba(80, 20, 20, 255),
            MAROON
        )
    {
        return Some(GameState::Exit);
    }

    None
}
