use crate::types::world::WorldId;
use crate::weapons::combat::{default_player_hitboxes, CombatEntity, EntityKind, Hitbox};
use macroquad::prelude::*;

mod anim;
mod net;
mod smoothing;

pub use crate::types::player_skin::PlayerSkin;
pub use anim::RemoteAnim;
pub use net::NetState;
pub use smoothing::RenderSmoothing;

/// Another player (or a bot) as we draw and shoot at them. The server's state
/// (`net`), where the body is drawn (`smooth`) and its animation (`anim`) are
/// separate parts; who they are and their health sit on the player itself.
pub struct OtherPlayer {
    pub id: u32,
    pub username: String,
    pub skin: Option<Texture2D>,
    pub color: Color,
    pub world: WorldId,

    // Gun Game progression: 1 = first weapon slot
    pub gun_level: u8,
    pub team: u8,
    pub kills: u32,

    // Current weapon name (e.g. "AK-47", "M4A1") — used to draw world-model gun
    pub current_weapon: String,
    // `current_weapon` parsed, and the name it was parsed from; see weapon_id()
    resolved_weapon: std::cell::RefCell<(String, Option<crate::weapons::stats::WeaponId>)>,

    // bot
    pub is_bot: bool,
    pub is_silent: bool, // no footstep sounds

    // health and hits (the CombatEntity side)
    pub hp: f32,
    pub max_hp: f32,
    pub is_alive: bool,
    pub hit_timer: f32,
    pub hitboxes: Vec<Hitbox>,

    // death animation: when Some, the player is dying and will be removed when timer hits 0
    pub death_timer: Option<f32>,

    /// where they are and what they're doing, as the server last said
    pub net: NetState,
    /// where their body is drawn, eased toward `net` every frame
    pub smooth: RenderSmoothing,
    /// which clip the body shows, and its own clock and pose
    pub anim: RemoteAnim,
}

impl OtherPlayer {
    pub fn color_for_id(id: u32) -> Color {
        let r = (id.wrapping_mul(73).wrapping_add(29) % 256) as u8;
        let g = (id.wrapping_mul(97).wrapping_add(53) % 256) as u8;
        let b = (id.wrapping_mul(61).wrapping_add(19) % 256) as u8;
        Color::from_rgba(r, g, b, 255)
    }

    pub fn new(id: u32, username: String, world: WorldId) -> Self {
        Self {
            id,
            username,
            skin: None,
            color: Self::color_for_id(id),
            world,
            gun_level: 1,
            team: 0,
            kills: 0,
            current_weapon: String::new(),
            resolved_weapon: std::cell::RefCell::new((String::new(), None)),
            is_bot: false,
            is_silent: false,
            hp: 125.0,
            max_hp: 125.0,
            is_alive: true,
            hit_timer: 0.0,
            hitboxes: default_player_hitboxes(),
            death_timer: None,
            net: NetState::default(),
            smooth: RenderSmoothing::new(),
            anim: RemoteAnim::new(),
        }
    }

    /// A new position from the server: `net` takes it, and the drawn body
    /// jumps there on the first one or a long jump (see `RenderSmoothing`).
    pub fn apply_snapshot(&mut self, x: f32, y: f32, z: f32, yaw: f32) {
        self.net.x = x;
        self.net.y = y;
        self.net.z = z;
        self.net.yaw = yaw;
        self.smooth.on_snapshot(x, y, z, yaw);
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
        self.smooth.position()
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
        self.smooth.ease_toward(&self.net, delta);

        // Dying players keep their death skin locked — don't let
        // idle/movement transitions overwrite it.
        if self.death_timer.is_some() {
            return;
        }

        if self.hit_timer > 0.0 {
            self.hit_timer = (self.hit_timer - delta).max(0.0);
        }
        self.anim.update(&self.net, delta);
    }

    fn hit_timer(&self) -> f32 {
        self.hit_timer
    }

    fn entity_color(&self) -> Color {
        self.color
    }
}
