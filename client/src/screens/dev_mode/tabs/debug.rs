use crate::ui::fonts::text_params;
use crate::weapons::crosshair::CrosshairConfig;
use crate::weapons::state::WeaponInventory;
use macroquad::prelude::*;

use super::super::state::DevModeState;
use super::super::widgets::{draw_action_button, draw_section_header, draw_toggle};

pub(in crate::screens::dev_mode) fn draw_debug_tab(
    dev: &mut DevModeState,
    inv: &mut WeaponInventory,
    x: f32,
    mut y: f32,
    mouse: (f32, f32),
    clicked: bool,
) {
    y += draw_section_header("Toggles", x, y);
    y += draw_toggle(
        "Infinite Ammo",
        &mut dev.infinite_ammo,
        x,
        y,
        mouse,
        clicked,
    );
    y += draw_toggle("No Recoil", &mut dev.no_recoil, x, y, mouse, clicked);
    y += draw_toggle("God Mode", &mut dev.god_mode, x, y, mouse, clicked);
    y += draw_toggle(
        "Show Hitboxes",
        &mut dev.show_hitboxes,
        x,
        y,
        mouse,
        clicked,
    );
    y += draw_toggle(
        "Show Health Bars",
        &mut dev.show_health_bars,
        x,
        y,
        mouse,
        clicked,
    );

    y += draw_section_header("Actions", x, y);

    let (h, pressed) = draw_action_button("Refill Ammo", x, y, 160.0, mouse, clicked);
    y += h;
    if pressed {
        let w = inv.active_mut();
        w.ammo = w.stats_data.magazine_size;
        w.reserve = w.stats_data.max_reserve;
    }

    let (_, pressed) = draw_action_button("Reset All Weapons", x, y, 180.0, mouse, clicked);
    if pressed {
        for w in &mut inv.weapons {
            w.reset_stats();
            w.crosshair = CrosshairConfig::for_category(w.id.category());
        }
    }

    // skin test: put every animation on the next loaded character, check it in the mirror
    y += 20.0;
    y += draw_section_header("Character (skin test)", x, y);
    draw_text_ex(&dev.character_label, x, y + 14.0, text_params(14, Color::from_rgba(180, 230, 180, 255)));
    y += 22.0;
    let (h, next) = draw_action_button("Next Character", x, y, 170.0, mouse, clicked);
    y += h;
    if next {
        dev.next_character_requested = true;
    }
    let (h, back) = draw_action_button("Back to default", x, y, 170.0, mouse, clicked);
    y += h;
    if back {
        dev.default_character_requested = true;
    }

    y += 20.0;
    y += draw_section_header("Distance Ruler", x, y);
    y += draw_toggle("Ruler Active", &mut dev.ruler_active, x, y, mouse, clicked);

    let (h, set_start) = draw_action_button("Set Start", x, y, 120.0, mouse, clicked);
    y += h;
    if set_start {
        dev.ruler_set_start_requested = true;
    }

    let (h, set_end) = draw_action_button("Set End", x, y, 120.0, mouse, clicked);
    y += h;
    if set_end {
        dev.ruler_set_end_requested = true;
    }

    let (h, clear) = draw_action_button("Clear Ruler", x, y, 120.0, mouse, clicked);
    y += h;
    if clear {
        dev.ruler_start = None;
        dev.ruler_end = None;
    }

    if let (Some(start), Some(end)) = (dev.ruler_start, dev.ruler_end) {
        let dist = (end - start).length();
        draw_text_ex(
            &format!("Distance: {:.2}", dist),
            x,
            y + 14.0,
            text_params(14, Color::from_rgba(180, 230, 180, 255)),
        );
    } else {
        draw_text_ex(
            "Set both points to measure distance",
            x,
            y + 14.0,
            text_params(14, GRAY),
        );
    }
}
