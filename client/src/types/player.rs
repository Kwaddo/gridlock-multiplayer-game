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

