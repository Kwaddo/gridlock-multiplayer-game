use crate::types::animation_state::{AnimState, MoveDir};
use crate::types::world::WorldId;
use crate::weapons::combat::{default_player_hitboxes, CombatEntity, EntityKind, Hitbox};
use macroquad::prelude::*;

pub use crate::types::player_skin::PlayerSkin;

pub struct OtherPlayer {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub render_x: f32,
    pub render_y: f32,
    pub render_z: f32,
    pub render_yaw: f32,
    // look up/down from the server (+ = up) and the smoothed one the body bends to
    // before this pitch never reached other players, so their bodies always looked level
    pub pitch: f32,
    pub render_pitch: f32,
    has_snapshot: bool,
    pub username: String,
    pub skin: Option<Texture2D>,
    pub skin_type: PlayerSkin,
    pub is_sprinting: bool,
    pub is_crouching: bool,
    pub is_prone: bool,
    pub is_sliding: bool,
    pub is_ads: bool,
    pub is_airborne: bool,
    pub evt_jump_start: bool,
    pub evt_landed: bool,
    pub color: Color,
    pub world: WorldId,
    pub hp: f32,
    pub max_hp: f32,
    pub is_alive: bool,
    pub hit_timer: f32,
    pub hitboxes: Vec<Hitbox>,
    pub kills: u32,
    pub sprint_grace: f32,
    // which way they last moved, None = standing (footsteps and the idle switch use it)
    // pending_move_dir/timer, prev_x/z and move_anim_grace used to sit here, nothing ever set them
    pub last_move_dir: MoveDir,
    // timestamp of the last frame where position actually changed (for idle detection)
    pub last_move_time: f64,

    // Gun Game progression: 1 = first weapon slot
    pub gun_level: u8,
    pub team: u8,

    // Current weapon name (e.g. "AK-47", "M4A1") — used to draw world-model gun
    pub current_weapon: String,
    // `current_weapon` parsed, and the name it was parsed from; see weapon_id()
    resolved_weapon: std::cell::RefCell<(String, Option<crate::weapons::stats::WeaponId>)>,

    // bot
    pub is_bot: bool,
    pub is_silent: bool, // no footstep sounds


    // death animation: when Some, the player is dying and will be removed when timer hits 0
    pub death_timer: Option<f32>,

    // this body's own animation clock and pose, before this skin_type was all a player had
    // and the clock lived on the shared clip
    pub anim: crate::types::player_anim::PlayerAnim,
    // shot this frame (bot shots now, players with #23), the animation plays the fire kick then clears it
    pub fired: bool,
}

impl OtherPlayer {
    const POSITION_SMOOTH_RATE: f32 = 22.0;
    const YAW_SMOOTH_RATE: f32 = 18.0;
    const SNAP_DISTANCE: f32 = 5.0;
    /// how long we continue treating a player as "sprinting" after the last sprint flag
    /// (helps hide missing/late network packets and stops walk/sprint flicker)
    pub const SPRINT_GRACE: f32 = 0.18; // seconds

    pub fn color_for_id(id: u32) -> Color {
        let r = (id.wrapping_mul(73).wrapping_add(29) % 256) as u8;
        let g = (id.wrapping_mul(97).wrapping_add(53) % 256) as u8;
        let b = (id.wrapping_mul(61).wrapping_add(19) % 256) as u8;
        Color::from_rgba(r, g, b, 255)
    }

    pub fn new(id: u32, username: String, world: WorldId) -> Self {
        Self {
            id,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            render_x: 0.0,
            render_y: 0.0,
            render_z: 0.0,
            render_yaw: 0.0,
            pitch: 0.0,
            render_pitch: 0.0,
            has_snapshot: false,
            username,
            skin: None,
            skin_type: PlayerSkin::RifleIdle,
            is_sprinting: false,
            is_crouching: false,
            is_prone: false,
            is_sliding: false,
            is_ads: false,
            is_airborne: false,
            evt_jump_start: false,
            evt_landed: false,
            color: Self::color_for_id(id),
            world,
            team: 0,
            hp: 125.0,
            max_hp: 125.0,
            is_alive: true,
            hit_timer: 0.0,
            hitboxes: default_player_hitboxes(),
            kills: 0,
            sprint_grace: 0.0,
            last_move_dir: MoveDir::None,
            last_move_time: 0.0,
            gun_level: 1,
            current_weapon: String::new(),
            resolved_weapon: std::cell::RefCell::new((String::new(), None)),
            is_bot: false,
            is_silent: false,
            death_timer: None,
            anim: crate::types::player_anim::PlayerAnim::new(),
            fired: false,
        }
    }

    pub fn apply_snapshot(&mut self, x: f32, y: f32, z: f32, yaw: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
        self.yaw = yaw;

        if !self.has_snapshot {
            self.render_x = x;
            self.render_y = y;
            self.render_z = z;
            self.render_yaw = yaw;
            self.has_snapshot = true;
            return;
        }

        let dx = self.render_x - x;
        let dy = self.render_y - y;
        let dz = self.render_z - z;
        let dist_sq = dx * dx + dy * dy + dz * dz;
        if dist_sq > Self::SNAP_DISTANCE * Self::SNAP_DISTANCE {
            self.render_x = x;
            self.render_y = y;
            self.render_z = z;
            self.render_yaw = yaw;
        }
    }

    /// `current_weapon` as a WeaponId. The name lookup (a case-insensitive scan
    /// of every weapon) runs only when the name changes, not on every draw.
    pub fn weapon_id(&self) -> Option<crate::weapons::stats::WeaponId> {
        let mut cache = self.resolved_weapon.borrow_mut();
        if cache.0 != self.current_weapon {
            cache.0.clone_from(&self.current_weapon);
            cache.1 = crate::weapons::stats::WeaponId::from_name(&self.current_weapon);
        }
        cache.1
    }

    pub fn render_position(&self) -> Vec3 {
        vec3(self.render_x, self.render_y, self.render_z)
    }

    pub fn smooth_render_state(&mut self, delta: f32) {
        if !self.has_snapshot {
            return;
        }

        let pos_alpha = 1.0 - (-Self::POSITION_SMOOTH_RATE * delta).exp();
        self.render_x += (self.x - self.render_x) * pos_alpha;
        self.render_y += (self.y - self.render_y) * pos_alpha;
        self.render_z += (self.z - self.render_z) * pos_alpha;

        let yaw_alpha = 1.0 - (-Self::YAW_SMOOTH_RATE * delta).exp();
        self.render_yaw = lerp_angle(self.render_yaw, self.yaw, yaw_alpha);
        // pitch never wraps (its +-89 degrees), a plain ease is enough
        self.render_pitch += (self.pitch - self.render_pitch) * yaw_alpha;
    }
}

impl CombatEntity for OtherPlayer {
    fn entity_kind(&self) -> EntityKind {
        EntityKind::Player
    }

    fn entity_id(&self) -> Option<u32> {
        Some(self.id)
    }

    fn entity_position(&self) -> Vec3 {
        self.render_position()
    }

    fn hitboxes(&self) -> &[Hitbox] {
        &self.hitboxes
    }

    fn take_damage(&mut self, dmg: f32) {
        self.hp = (self.hp - dmg).max(0.0);
        self.hit_timer = 0.3; // flash red for 300ms
    }

    fn is_alive(&self) -> bool {
        self.is_alive
    }

    fn update(&mut self, delta: f32) {
        self.smooth_render_state(delta);

        // Dying players keep their death skin locked — don't let
        // idle/movement transitions overwrite it.
        if self.death_timer.is_some() {
            return;
        }

        if self.hit_timer > 0.0 {
            self.hit_timer = (self.hit_timer - delta).max(0.0);
        }
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
                    self.yaw,
                    false,
                    self.is_ads,
                    self.is_crouching,
                    self.is_prone,
                    self.is_sliding,
                    false,
                );
                self.skin_type = AnimState::skin_from_state(anim, None);
                // no snap anymore, the smoothing catches up in ~0.1 s while the blend eases into idle
                // before this it teleported the drawn body onto the real spot, a small visible jump
            }
        }
    }

    fn hit_timer(&self) -> f32 {
        self.hit_timer
    }

    fn entity_color(&self) -> Color {
        self.color
    }
}

fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let mut delta = to - from;
    while delta > std::f32::consts::PI {
        delta -= std::f32::consts::TAU;
    }
    while delta < -std::f32::consts::PI {
        delta += std::f32::consts::TAU;
    }
    from + delta * t
}
