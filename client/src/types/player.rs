use crate::types::world::WorldId;
use crate::weapons::combat::{default_player_hitboxes, CombatEntity, EntityKind, Hitbox};
use macroquad::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerSkin {
    Texture,
    RifleIdle,
    RifleAimIdle,
    RifleWalk,
}

impl PlayerSkin {
    pub const fn is_model_based(self) -> bool {
        !matches!(self, PlayerSkin::Texture)
    }

    pub const fn model_key(self) -> Option<&'static str> {
        match self {
            PlayerSkin::Texture => None,
            PlayerSkin::RifleIdle => Some("rifle_idle"),
            PlayerSkin::RifleAimIdle => Some("rifle_aim_idle"),
            PlayerSkin::RifleWalk => Some("rifle_walk"),
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            PlayerSkin::Texture => "Texture",
            PlayerSkin::RifleIdle => "Rifle Idle",
            PlayerSkin::RifleAimIdle => "Rifle Aim",
            PlayerSkin::RifleWalk => "Rifle Walk",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            PlayerSkin::Texture => PlayerSkin::RifleIdle,
            PlayerSkin::RifleIdle => PlayerSkin::RifleAimIdle,
            PlayerSkin::RifleAimIdle => PlayerSkin::RifleWalk,
            PlayerSkin::RifleWalk => PlayerSkin::Texture,
        }
    }
}

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
    has_snapshot: bool,
    pub username: String,
    pub skin: Option<Texture2D>,
    pub skin_type: PlayerSkin,
    pub color: Color,
    pub world: WorldId,
    pub hp: f32,
    pub max_hp: f32,
    pub hit_timer: f32,
    pub hitboxes: Vec<Hitbox>,
}

impl OtherPlayer {
    const POSITION_SMOOTH_RATE: f32 = 14.0;
    const YAW_SMOOTH_RATE: f32 = 18.0;
    const SNAP_DISTANCE: f32 = 5.0;

    pub fn color_for_id(id: u32) -> Color {
        let r = ((id * 73 + 29) % 256) as u8;
        let g = ((id * 97 + 53) % 256) as u8;
        let b = ((id * 61 + 19) % 256) as u8;
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
            has_snapshot: false,
            username,
            skin: None,
            skin_type: PlayerSkin::RifleIdle,
            color: Self::color_for_id(id),
            world,
            hp: 100.0,
            max_hp: 100.0,
            hit_timer: 0.0,
            hitboxes: default_player_hitboxes(),
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
        vec3(self.x, self.y, self.z)
    }

    fn hitboxes(&self) -> &[Hitbox] {
        &self.hitboxes
    }

    fn take_damage(&mut self, dmg: f32) {
        self.hp = (self.hp - dmg).max(0.0);
        self.hit_timer = 0.3; // flash red for 300ms
    }

    fn is_alive(&self) -> bool {
        self.hp > 0.0
    }

    fn update(&mut self, delta: f32) {
        self.smooth_render_state(delta);
        if self.hit_timer > 0.0 {
            self.hit_timer = (self.hit_timer - delta).max(0.0);
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
