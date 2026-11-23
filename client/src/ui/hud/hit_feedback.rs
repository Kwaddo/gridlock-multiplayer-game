use macroquad::prelude::*;
use std::cell::RefCell;
use std::fmt::Write;

use crate::ui::fonts::{measure_game_text, text_params};
use crate::ui::prims::draw_circle;
use crate::weapons::effects::DamageNumber;
use crate::weapons::state::WeaponState;

// the hit marker and the floating damage numbers, drawn over the crosshair

thread_local! {
    // reused for every damage number so drawing them does not allocate
    static DAMAGE_TEXT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// The X (or, for pellet weapons, one dot per pellet that hit) shown for a
/// moment after a hit, sized from the active weapon's crosshair.
pub(super) fn draw_hit_marker(
    cx: f32,
    cy: f32,
    weapon: &WeaponState,
    hit_marker_timer: f32,
    hit_marker_headshot: bool,
    hit_marker_pellet_mask: u16,
) {
    let sh = screen_height();
    let xhair = &weapon.crosshair;
    let ads_t = weapon.ads_progress;

    if hit_marker_timer > 0.0 {
        let t = (hit_marker_timer / 0.35).min(1.0);
        let alpha = (t * 255.0) as u8;
        let hm_color = if hit_marker_headshot {
            Color::from_rgba(255, 215, 0, alpha)
        } else {
            Color::from_rgba(255, 255, 255, alpha)
        };

        let hip_gap = ((xhair.center_gap * sh) / 100.0) * (1.0 - ads_t);
        let ads_rest_gap = ((xhair.center_gap * sh) / 100.0) * 0.4 * ads_t;
        let gap = hip_gap + ads_rest_gap;
        let len = (xhair.line_length * sh) / 100.0;
        let current_size = (gap + len) * 2.0;

        let current_size = if current_size < 10.0 {
            (sh * 3.0) / 100.0
        } else {
            current_size
        };

        let pop = 1.0 + 0.4 * t;
        let hm_thick = 2.5;
        let stats = weapon.stats();
        let pellet_spread = stats.pellet_spread;
        if let Some(ptrn) = pellet_spread {
            let dot_radius = 5.0;
            let ring_radius = current_size * 0.3 * pop;
            for (i, &(angle, radius)) in ptrn.iter().enumerate() {
                let angle: f32 = angle;
                let radius: f32 = radius;
                if hit_marker_pellet_mask & (1 << i) != 0 {
                    let r: f32 = ring_radius * radius;
                    let dx: f32 = cx + angle.cos() * r;
                    let dy: f32 = cy - angle.sin() * r;
                    draw_circle(dx, dy, dot_radius, RED);
                }
            }
        } else {
            let hm_size = current_size * 0.4 * pop;
            let hm_gap = current_size * 0.1 * pop;
            draw_line(
                cx - hm_size,
                cy - hm_size,
                cx - hm_gap,
                cy - hm_gap,
                hm_thick,
                hm_color,
            );
            draw_line(
                cx + hm_gap,
                cy - hm_gap,
                cx + hm_size,
                cy - hm_size,
                hm_thick,
                hm_color,
            );
            draw_line(
                cx - hm_size,
                cy + hm_size,
                cx - hm_gap,
                cy + hm_gap,
                hm_thick,
                hm_color,
            );
            draw_line(
                cx + hm_gap,
                cy + hm_gap,
                cx + hm_size,
                cy + hm_size,
                hm_thick,
                hm_color,
            );
        }
    }
}

pub(super) fn draw_damage_numbers(damage_numbers: &[DamageNumber]) {
    for dmg in damage_numbers {
        let alpha = (dmg.lifetime.min(1.0) * 255.0) as u8;
        let (color, size) = if dmg.is_headshot {
            (Color::from_rgba(255, 215, 0, alpha), 28)
        } else {
            (Color::from_rgba(255, 255, 255, alpha), 22)
        };
        DAMAGE_TEXT.with_borrow_mut(|text| {
            text.clear();
            let _ = write!(text, "{}", dmg.amount as u32);
            let tw = measure_game_text(text, size);
            draw_text_ex(
                text,
                dmg.screen_x - tw / 2.0,
                dmg.screen_y,
                text_params(size, color),
            );
        });
    }
}
