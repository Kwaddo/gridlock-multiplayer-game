use crate::types::world::WorldId;
use crate::weapons::combat::{default_player_hitboxes, CombatEntity, EntityKind, Hitbox};
use macroquad::prelude::*;

pub struct OtherPlayer {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub target_z: f32,
    pub target_yaw: f32,
    pub target_pitch: f32,
    pub username: String,
    pub skin: Option<Texture2D>,
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
            pitch: 0.0,
            target_x: 0.0,
            target_y: 0.0,
            target_z: 0.0,
            target_yaw: 0.0,
            target_pitch: 0.0,
            username,
            skin: None,
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
