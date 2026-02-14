use crate::weapons::combat::{default_player_hitboxes, CombatEntity, EntityKind, Hitbox};
use macroquad::prelude::*;

// NPC Target Dummy
pub struct TargetDummy {
    pub position: Vec3, // feet position
    pub hp: f32,
    pub max_hp: f32,
    pub hit_timer: f32,        // >0 = flash red (seconds remaining)
    pub hitboxes: Vec<Hitbox>, // for more complex targets with multiple hit zones, but for simplicity we'll just do one head box and one body box
}

impl CombatEntity for TargetDummy {
    fn entity_kind(&self) -> EntityKind {
        EntityKind::Dummy
    }

    fn entity_id(&self) -> Option<u32> {
        None
    }

    fn entity_position(&self) -> Vec3 {
        self.position
    }

    fn hitboxes(&self) -> &[Hitbox] {
        &self.hitboxes
    }

    fn take_damage(&mut self, dmg: f32) {
        self.hp = (self.hp - dmg).max(1.0); // target dummy never dies, it just has a minimum of 1 HP
        self.hit_timer = 0.05; // flash red for 10ms
    }

    fn is_alive(&self) -> bool {
        true // target dummy never dies, it just has a minimum of 1 HP
    }

    fn update(&mut self, delta: f32) {
        if self.hit_timer > 0.0 {
            self.hit_timer = (self.hit_timer - delta).max(0.0);
        }
        // regen: heal back to full over 3 seconds when not being hit
        if self.hit_timer <= 0.0 && self.hp < self.max_hp {
            self.hp = (self.hp + 20.0 * delta).min(self.max_hp);
        }
    }

    fn hit_timer(&self) -> f32 {
        self.hit_timer
    }

    fn entity_color(&self) -> Color {
        WHITE // zone colors handled by renderer
    }
}

impl TargetDummy {
    pub fn new(pos: Vec3) -> Self {
        Self {
            position: pos,
            hp: 100.0,
            max_hp: 100.0,
            hit_timer: 0.0,
            hitboxes: default_player_hitboxes(),
        }
    }

    pub fn body_center(&self) -> Vec3 {
        self.position + vec3(0.0, 0.6, 0.0) // body center at waist height
    }

    pub fn head_center(&self) -> Vec3 {
        self.position + vec3(0.0, 1.4, 0.0) // head center
    }
}
