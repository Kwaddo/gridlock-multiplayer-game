use crate::core::GameConfig;
use crate::core::GameState;
use crate::screens::dev_mode::DevModeState;
use crate::settings::SettingsScreen;
use crate::ui::fonts::{measure_game_text, text_params};
use crate::ui::is_button_clicked;
use macroquad::prelude::*;

/// What the player picked in the pause menu.
pub enum PauseAction {
    Go(GameState),
    /// Host only: stop the match for everyone; all return to the lobby.
    EndMatch,
    /// Go back to the lobby, leaving the match running for the others.
    LeaveMatch,
}

#[derive(Clone, Copy, PartialEq)]
enum Button {
    Resume,
    Settings,
    Loadout,
    DevMode,
    /// the dev-mode code prompt was answered
    DevModeUnlocked,
    EndMatch,
    LeaveMatch,
    Exit,
}

const PANEL_W: f32 = 300.0;
const BTN_W: f32 = 200.0;
const BTN_H: f32 = 40.0;
const FIRST_BTN: f32 = 80.0;
const BTN_STEP: f32 = 50.0;
/// Exit sits a little apart from the rest.
const EXIT_GAP: f32 = 70.0;

/// The pause menu. Ending the match takes a second, confirming click: it ends
/// the game for everyone.
#[derive(Default)]
pub struct PauseMenu {
    /// When the "end the match?" question opened (`get_time`), while it is up.
    confirm_end: Option<f64>,
}

/// The question ignores clicks this long after it opens, so a double click on
/// End Match doesn't answer it too.
const CONFIRM_ARM_SECS: f64 = 0.35;

impl PauseMenu {
    /// Back to the main view (e.g. on a new connection).
    pub fn reset(&mut self) {
        self.confirm_end = None;
    }

    /// `in_match`: the player is in the maze; `is_host`: they may end the match.
    pub fn draw(
        &mut self,
        config: &GameConfig,
        settings_screen: &mut SettingsScreen,
        dev_mode: &mut DevModeState,
        in_match: bool,
        is_host: bool,
    ) -> Option<PauseAction> {
        // nothing to confirm once the match is over or we no longer host
        if !(in_match && is_host) {
            self.confirm_end = None;
        }
        let action = if let Some(opened) = self.confirm_end {
            self.draw_confirm_end(config, get_time() - opened >= CONFIRM_ARM_SECS)
        } else {
            match draw_main(config, dev_mode, in_match, is_host)? {
                Button::EndMatch => {
                    self.confirm_end = Some(get_time());
                    None
                }
                picked => button_action(picked, config, settings_screen, dev_mode),
            }
        };
        if action.is_some() {
            self.confirm_end = None;
        }
        action
    }

    /// "End the match for everyone?": Cancel or End Match. `armed`: clicks count.
    fn draw_confirm_end(&mut self, config: &GameConfig, armed: bool) -> Option<PauseAction> {
        // ESC backs out of the question, not out of the menu
        if config.bind_settings.is_pressed() {
            self.confirm_end = None;
            return None;
        }
        let panel_h = 230.0;
        let panel_x = (screen_width() - PANEL_W) / 2.0;
        let panel_y = (screen_height() - panel_h) / 2.0;
        draw_panel_titled(panel_x, panel_y, panel_h, "END MATCH?");

        let note = "Everyone returns to the lobby.";
        let note_w = measure_game_text(note, 16);
        draw_text_ex(
            note,
            (screen_width() - note_w) / 2.0,
            panel_y + 72.0,
            text_params(16, Color::from_rgba(200, 200, 200, 255)),
        );

        let btn_x = (screen_width() - BTN_W) / 2.0;
        if is_button_clicked(
            btn_x,
            panel_y + 100.0,
            BTN_W,
            BTN_H,
            "Cancel",
            Color::from_rgba(50, 50, 50, 255),
            DARKGRAY,
        ) && armed
        {
            self.confirm_end = None;
            return None;
        }
        if is_button_clicked(
            btn_x,
            panel_y + 160.0,
            BTN_W,
            BTN_H,
            "End Match",
            Color::from_rgba(80, 20, 20, 255),
            MAROON,
        ) && armed
        {
            return Some(PauseAction::EndMatch);
        }
        None
    }
}

/// The main pause view; returns the button clicked. A dev-mode code prompt,
/// when open, takes the input instead.
fn draw_main(
    config: &GameConfig,
    dev_mode: &mut DevModeState,
    in_match: bool,
    is_host: bool,
) -> Option<Button> {
    let mut buttons = vec![Button::Resume, Button::Settings];
    if !in_match {
        buttons.push(Button::Loadout);
    }
    buttons.push(Button::DevMode);
    if in_match {
        if is_host {
            buttons.push(Button::EndMatch);
        }
        buttons.push(Button::LeaveMatch);
    }
    // Exit is drawn after these

    let panel_h = FIRST_BTN + (buttons.len() - 1) as f32 * BTN_STEP + EXIT_GAP + BTN_H + 20.0;
    let panel_x = (screen_width() - PANEL_W) / 2.0;
    let panel_y = (screen_height() - panel_h) / 2.0;

    // if code prompt open, ONLY handle input for that (ignore all other pause menu input including mouse clicks)
    if dev_mode.show_code_prompt {
        // still draw the pause menu visuals behind it
        draw_panel(panel_x, panel_y, panel_h);
        if DevModeState::update_code_prompt(dev_mode) {
            return Some(Button::DevModeUnlocked);
        }
        DevModeState::draw_code_prompt(dev_mode);
        return None; // Block all button interaction
    }

    // ESC to resume (is_key_pressed is consumed by the Playing state's check,
    // so this only fires on a *second* ESC press while paused as to not immediately unpause when you open the menu)
    if config.bind_settings.is_pressed() {
        return Some(Button::Resume);
    }

    draw_panel(panel_x, panel_y, panel_h);

    let btn_x = (screen_width() - BTN_W) / 2.0;
    let btn_color = Color::from_rgba(50, 50, 50, 255);
    let btn_hover = DARKGRAY;
    let danger = (Color::from_rgba(80, 20, 20, 255), MAROON);

    let mut y = panel_y + FIRST_BTN;
    let mut picked = None;
    for button in buttons.iter().copied().chain(std::iter::once(Button::Exit)) {
        if button == Button::Exit {
            y += EXIT_GAP - BTN_STEP;
        }
        let (label, (color, hover)) = match button {
            Button::Resume => ("Resume", (btn_color, btn_hover)),
            Button::Settings => ("Settings", (btn_color, btn_hover)),
            Button::Loadout => ("Loadout", (btn_color, btn_hover)),
            Button::DevMode => (
                "Dev Mode",
                (Color::from_rgba(40, 60, 80, 255), Color::from_rgba(60, 90, 120, 255)),
            ),
            Button::EndMatch => ("End Match", danger),
            Button::DevModeUnlocked => continue, // never listed
            Button::LeaveMatch => ("Leave Match", (btn_color, btn_hover)),
            Button::Exit => ("Exit", danger),
        };
        if is_button_clicked(btn_x, y, BTN_W, BTN_H, label, color, hover) && picked.is_none() {
            picked = Some(button);
        }
        y += BTN_STEP;
    }

    picked
}

fn button_action(
    button: Button,
    config: &GameConfig,
    settings_screen: &mut SettingsScreen,
    dev_mode: &mut DevModeState,
) -> Option<PauseAction> {
    match button {
        Button::Resume => Some(PauseAction::Go(GameState::Playing)),
        Button::Settings => {
            settings_screen.enter(config);
            Some(PauseAction::Go(GameState::Settings))
        }
        Button::Loadout => Some(PauseAction::Go(GameState::LobbyInventory)),
        Button::DevMode => {
            dev_mode.request_open();
            dev_mode.is_unlocked().then_some(PauseAction::Go(GameState::DevMode))
        }
        Button::DevModeUnlocked => Some(PauseAction::Go(GameState::DevMode)),
        Button::EndMatch => Some(PauseAction::EndMatch),
        Button::LeaveMatch => Some(PauseAction::LeaveMatch),
        Button::Exit => Some(PauseAction::Go(GameState::Exit)),
    }
}

fn draw_panel(panel_x: f32, panel_y: f32, panel_h: f32) {
    draw_panel_titled(panel_x, panel_y, panel_h, "PAUSED");
}

/// Dimmed game behind, and the menu panel with its title.
fn draw_panel_titled(panel_x: f32, panel_y: f32, panel_h: f32, title: &str) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::from_rgba(0, 0, 0, 150),
    );
    draw_rectangle(
        panel_x,
        panel_y,
        PANEL_W,
        panel_h,
        Color::from_rgba(30, 30, 30, 240),
    );
    draw_rectangle_lines(panel_x, panel_y, PANEL_W, panel_h, 2.0, WHITE);

    let title_w = measure_game_text(title, 30);
    draw_text_ex(
        title,
        (screen_width() - title_w) / 2.0,
        panel_y + 40.0,
        text_params(30, WHITE),
    );
}
