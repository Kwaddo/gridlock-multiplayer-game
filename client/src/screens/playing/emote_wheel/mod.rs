//! Hold-to-open emote wheel: mouse movement steers a virtual cursor over five
//! slices, releasing the key plays the hovered one.

mod constants;
mod draw;

use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::types::player::PlayerSkin;
use constants::{CURSOR_MAX, DEADZONE};

const SLOTS: usize = PlayerSkin::EMOTES.len();

#[derive(Default)]
pub struct EmoteWheel {
    open: bool,
    cursor: Vec2,
    /// Emote id being played (0 = none).
    pub playing: u8,
    /// Where the body faces while emoting; the mouse orbits the camera instead.
    pub body_yaw: f32,
    /// Slice chosen on the last release; stays lit while the wheel fades out.
    picked: Option<usize>,
    /// 0 = hidden, 1 = fully open; eases both ways.
    open_t: f32,
    /// Per-slice highlight, eased towards 1 for the hovered slice.
    hover: [f32; SLOTS],
}

impl EmoteWheel {
    pub fn is_playing(&self) -> bool {
        self.playing != crate::net::protocols::EMOTE_NONE
    }

    /// Feed the key state, mouse movement and current yaw; true while the wheel is open.
    pub fn update(&mut self, held: bool, mouse_delta: Vec2, yaw: f32) -> bool {
        if held {
            if !self.open {
                self.open = true;
                self.cursor = Vec2::ZERO;
                self.picked = None;
            }
            self.cursor = (self.cursor + mouse_delta).clamp_length_max(CURSOR_MAX);
            return true;
        }
        if self.open {
            self.open = false;
            self.picked = slot_at(self.cursor);
            if let Some(i) = self.picked {
                if !self.is_playing() {
                    self.body_yaw = yaw; // switching emotes keeps the facing
                }
                self.playing = i as u8 + 1;
            }
        }
        false
    }

    /// Close without picking.
    pub fn close(&mut self) {
        self.open = false;
        self.picked = None;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Slice to light up: the hovered one while open, the picked one while fading out.
    fn lit_slot(&self) -> Option<usize> {
        if self.open {
            slot_at(self.cursor)
        } else {
            self.picked
        }
    }
}

/// Slice under the cursor: slot 0 is up, then clockwise (screen y points down).
fn slot_at(c: Vec2) -> Option<usize> {
    if c.length() < DEADZONE {
        return None;
    }
    let a = c.x.atan2(-c.y).rem_euclid(TAU);
    Some(((a / TAU * SLOTS as f32 + 0.5) as usize) % SLOTS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_follow_direction() {
        assert_eq!(slot_at(vec2(0.0, -50.0)), Some(0));
        assert_eq!(slot_at(vec2(50.0, 0.0)), Some(1));
        assert_eq!(slot_at(vec2(-50.0, 0.0)), Some(4));
        assert_eq!(slot_at(vec2(5.0, 5.0)), None);
    }

    #[test]
    fn release_on_slice_plays_it() {
        let mut w = EmoteWheel::default();
        assert!(w.update(true, vec2(0.0, -80.0), 1.5));
        assert!(!w.update(false, Vec2::ZERO, 1.5));
        assert_eq!(w.playing, 1);
        assert_eq!(w.body_yaw, 1.5);
        assert_eq!(w.lit_slot(), Some(0)); // stays lit while fading out
    }

    #[test]
    fn switching_emote_keeps_body_yaw() {
        let mut w = EmoteWheel::default();
        w.update(true, vec2(0.0, -80.0), 1.5);
        w.update(false, Vec2::ZERO, 1.5);
        w.update(true, vec2(80.0, 0.0), 3.0); // camera orbited meanwhile
        w.update(false, Vec2::ZERO, 3.0);
        assert_eq!(w.playing, 2);
        assert_eq!(w.body_yaw, 1.5);
    }

    #[test]
    fn release_in_deadzone_cancels() {
        let mut w = EmoteWheel::default();
        w.update(true, vec2(3.0, 3.0), 0.0);
        w.update(false, Vec2::ZERO, 0.0);
        assert_eq!(w.playing, 0);
        assert_eq!(w.lit_slot(), None);
    }

    #[test]
    fn open_animation_eases_in_and_out() {
        let mut w = EmoteWheel::default();
        w.update(true, Vec2::ZERO, 0.0);
        w.animate(0.07);
        assert!(w.open_t > 0.0 && w.open_t < 1.0); // mid-way, not a snap
        w.animate(1.0);
        assert_eq!(w.open_t, 1.0);
        w.close();
        w.animate(1.0);
        assert_eq!(w.open_t, 0.0);
    }
}
