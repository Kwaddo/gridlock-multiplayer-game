use crate::net::protocols::{
    MODE_GUN_GAME, MODE_SEARCH_AND_DESTROY, MODE_TEAM_DEATHMATCH, balanced_bot_order,
};
use macroquad::prelude::*;

/// Bots per side once the match starts: with two or more humans, the
/// `red + blue` bots fill the smaller team first, as the server places them
/// (`balanced_bot_order`); a player alone gets what was asked for.
pub(super) fn bot_split(red: u8, blue: u8, red_humans: usize, blue_humans: usize) -> (usize, usize) {
    if red_humans + blue_humans < 2 {
        return (red as usize, blue as usize);
    }
    let order = balanced_bot_order(red_humans, blue_humans, (red + blue) as usize);
    let to_red = order.iter().filter(|r| **r).count();
    (to_red, order.len() - to_red)
}

pub(super) fn mode_name(mode: u8) -> &'static str {
    match mode {
        MODE_GUN_GAME => "GUN GAME",
        MODE_TEAM_DEATHMATCH => "TEAM DEATHMATCH",
        MODE_SEARCH_AND_DESTROY => "SEARCH & DESTROY",
        _ => "UNKNOWN MODE",
    }
}

pub(super) fn mode_description(mode: u8) -> &'static str {
    match mode {
        MODE_GUN_GAME => "Progress through every weapon — first to finish the arsenal wins.",
        MODE_TEAM_DEATHMATCH => "Two teams compete. First to 100 kills or end of 15 minutes wins.",
        MODE_SEARCH_AND_DESTROY => {
            "Attackers plant the bomb, defenders defuse it. No respawns — rounds decide the match."
        }
        _ => "",
    }
}

pub(super) fn mode_tips(mode: u8) -> &'static [&'static str] {
    match mode {
        MODE_GUN_GAME => &[
            "Every kill moves you on to the next weapon.",
            "After the last gun, win it with a punch kill.",
            "Get punched and you drop back to the first weapon.",
        ],
        MODE_TEAM_DEATHMATCH => &[
            "Every kill scores for your team; first to 100 wins.",
            "You respawn a few seconds after dying.",
            "Teams are balanced for you before the match starts.",
        ],
        MODE_SEARCH_AND_DESTROY => &[
            "Attackers plant the bomb at the site; defenders stop or defuse it.",
            "One life per round: the dead wait for the next round.",
            "Win the most rounds to win the match.",
        ],
        _ => &[],
    }
}

/// A `StartGame` duration code as players read it.
pub(super) fn duration_text(mode: u8, code: u16) -> String {
    match code {
        u16::MAX => "Unlimited".to_string(),
        0 if mode == MODE_SEARCH_AND_DESTROY => "Rounds".to_string(),
        0 => "15 MIN".to_string(),
        secs if secs % 60 == 0 => format!("{} MIN", secs / 60),
        secs => format!("{}:{:02}", secs / 60, secs % 60),
    }
}

/// The enabled bot difficulties (bit0 = Recruit .. bit4 = Nightmare).
pub(super) fn difficulty_summary(mask: u8) -> String {
    let names = ["Rec", "Easy", "Norm", "Hard", "NM"];
    let on: Vec<&str> = names.iter().enumerate().filter(|(i, _)| mask >> i & 1 == 1).map(|(_, n)| *n).collect();
    match on.len() {
        5 => "All".to_string(),
        0 => "None".to_string(),
        _ => on.join(", "),
    }
}

/// Word-wrapped text `w` wide from `x`, first baseline at `y`; centred in
/// that width or left-aligned.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_wrapped(text: &str, x: f32, w: f32, y: f32, size: u16, color: Color, centred: bool) {
    use crate::ui::fonts::{measure_game_text, text_params};

    WRAPPED.with_borrow_mut(|cache| {
        let lines = wrapped_lines(cache, text, w, size);
        let line_h = size as f32 + 6.0;
        for (i, line) in lines.iter().enumerate() {
            let lx = if centred { x + (w - measure_game_text(line, size)) / 2.0 } else { x };
            draw_text_ex(line, lx, y + i as f32 * line_h, text_params(size, color));
        }
    });
}

/// Texts draw_wrapped has wrapped lately; the console shows only a few.
const WRAPPED_CACHE: usize = 8;

/// One text wrapped for a width and size.
struct Wrapped {
    text: String,
    /// (width bits, size, DPI scale bits, game font loaded): what the lines depend on
    key: (u32, u16, u32, bool),
    lines: Vec<String>,
}

thread_local! {
    static WRAPPED: std::cell::RefCell<Vec<Wrapped>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The lines of `text` wrapped to `w`, wrapped again only when the text,
/// width, size or font scale changes.
fn wrapped_lines<'a>(cache: &'a mut Vec<Wrapped>, text: &str, w: f32, size: u16) -> &'a [String] {
    use crate::ui::fonts::{game_font, measure_game_text_uncached};

    let key = (w.to_bits(), size, screen_dpi_scale().to_bits(), game_font().is_some());
    if let Some(i) = cache.iter().position(|c| c.key == key && c.text == text) {
        return &cache[i].lines;
    }

    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let test = if cur.is_empty() { word.to_string() } else { format!("{} {}", cur, word) };
        if !cur.is_empty() && measure_game_text_uncached(&test, size) > w {
            lines.push(std::mem::replace(&mut cur, word.to_string()));
        } else {
            cur = test;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }

    if cache.len() >= WRAPPED_CACHE {
        cache.remove(0);
    }
    cache.push(Wrapped { text: text.to_string(), key, lines });
    &cache[cache.len() - 1].lines
}
