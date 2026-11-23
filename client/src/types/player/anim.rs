use crate::types::animation_state::{AnimState, MoveDir};
use crate::types::player_anim::PlayerAnim;

use super::net::NetState;
use super::PlayerSkin;

/// Which clip another player's body shows, and the body's own animation clock
/// and pose (`PlayerAnim`).
pub struct RemoteAnim {
    // the clip picked from their movement and stance
    pub skin_type: PlayerSkin,
    pub sprint_grace: f32,
    // which way they last moved, None = standing (footsteps and the idle switch use it)
    // pending_move_dir/timer, prev_x/z and move_anim_grace used to sit here, nothing ever set them
    pub last_move_dir: MoveDir,
    // timestamp of the last frame where position actually changed (for idle detection)
    pub last_move_time: f64,
    // shot this frame (bot shots now, players with #23), the animation plays the fire kick then clears it
    pub fired: bool,
    // this body's own animation clock and pose, before this skin_type was all a player had
    // and the clock lived on the shared clip
    pub body: PlayerAnim,
}

impl RemoteAnim {
    /// how long we continue treating a player as "sprinting" after the last sprint flag
    /// (helps hide missing/late network packets and stops walk/sprint flicker)
    pub const SPRINT_GRACE: f32 = 0.18; // seconds

    pub fn new() -> Self {
        Self {
            skin_type: PlayerSkin::RifleIdle,
            sprint_grace: 0.0,
            last_move_dir: MoveDir::None,
            last_move_time: 0.0,
            fired: false,
            body: PlayerAnim::new(),
        }
    }

    /// Per-frame clip upkeep for a living body: the sprint grace runs out, and
    /// a body that stopped moving switches to its idle clip.
    pub fn update(&mut self, net: &NetState, delta: f32) {
        if self.sprint_grace > 0.0 {
            self.sprint_grace = (self.sprint_grace - delta).max(0.0);
        }
        // Transition to idle if no position change has been seen for a short
        // grace period. This runs per-frame so it's independent of broadcast
        // frequency, and avoids flicker caused by broadcasts where this
        // player's position didn't change.
        if self.last_move_dir != MoveDir::None {
            let now = macroquad::time::get_time();
            if now - self.last_move_time > 0.12 {
                self.last_move_dir = MoveDir::None;
                let anim = AnimState::from_movement(
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    net.yaw,
                    false,
                    net.is_ads,
                    net.is_crouching,
                    net.is_prone,
                    net.is_sliding,
                    false,
                );
                self.skin_type = AnimState::skin_from_state(anim, None);
                // no snap anymore, the smoothing catches up in ~0.1 s while the blend eases into idle
                // before this it teleported the drawn body onto the real spot, a small visible jump
            }
        }
    }
}
