/*
    This module is used to simulate an actual real network player for testing
    this "player" will follow instructions from user to mvoe around and shoot
    will be used to test combat mechanics and hit detection without needing to connect to a 
    real server or have another player, will also serve well for testing player models later on
*/

use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;
use crate::weapons::combat::{ CombatEntity, EntityKind, Hitbox };
use macroquad::prelude::*;
use macroquad::rand::gen_range;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MovementPattern {
    None,
    ForwardBack,
    SideToSide,
    Forward,
    Back,
    Left,
    Right,
    Circle,
    Random,
}

#[derive(Clone, Debug)]
pub struct ActionState {
    pub jump_spam: bool,
    pub crouch: bool,
    pub slide: bool,
    pub shoot: bool,
}

pub struct SimulatedPlayer {
    pub player: OtherPlayer,
    pub pattern: MovementPattern,
    pub action_state: ActionState,
    movement_timer: f32,
    pattern_change_timer: f32,
    current_random_pattern: MovementPattern,
}

impl SimulatedPlayer {
    pub fn new(id: u32, username: String, world: WorldId, spawn_pos: Vec3) -> Self {
        let mut player = OtherPlayer::new(id, username, world);
        player.x = spawn_pos.x;
        player.y = spawn_pos.y;
        player.z = spawn_pos.z;

        Self {
            player,
            pattern: MovementPattern::None,
            action_state: ActionState {
                jump_spam: false,
                crouch: false,
                slide: false,
                shoot: false,
            },
            movement_timer: 0.0,
            pattern_change_timer: gen_range(3.0, 8.0),
            current_random_pattern: MovementPattern::ForwardBack,
        }
    }

    fn random_pattern() -> MovementPattern {
        match gen_range(0, 7) {
            0 => MovementPattern::ForwardBack,
            1 => MovementPattern::SideToSide,
            2 => MovementPattern::Forward,
            3 => MovementPattern::Back,
            4 => MovementPattern::Left,
            5 => MovementPattern::Right,
            _ => MovementPattern::Circle,
        }
    }

    pub fn simulate(&mut self, delta: f32) {
        self.movement_timer += delta;

        let active_pattern = if self.pattern == MovementPattern::Random {
            self.pattern_change_timer -= delta;
            if self.pattern_change_timer <= 0.0 {
                self.current_random_pattern = Self::random_pattern();
                self.pattern_change_timer = gen_range(3.0, 8.0);
                self.movement_timer = 0.0;
            }
            self.current_random_pattern
        } else {
            self.pattern
        };

        let move_speed = 3.0;
        let oscillation_period = 2.0;

        match active_pattern {
            MovementPattern::None => {},
            MovementPattern::ForwardBack => {
                let t = (self.movement_timer / oscillation_period * std::f32::consts::PI * 2.0).sin();
                self.player.z += t * move_speed * delta;
            }
            MovementPattern::SideToSide => {
                let t = (self.movement_timer / oscillation_period * std::f32::consts::PI * 2.0).sin();
                self.player.x += t * move_speed * delta;
            }
            MovementPattern::Forward => {
                self.player.z += move_speed * delta;
            }
            MovementPattern::Back => {
                self.player.z -= move_speed * delta;
            }
            MovementPattern::Left => {
                self.player.x -= move_speed * delta;
            }
            MovementPattern::Right => {
                self.player.x += move_speed * delta;
            }
            MovementPattern::Circle => {
                let radius = 5.0;
                let angle = self.movement_timer / oscillation_period * std::f32::consts::PI * 2.0;
                self.player.x = angle.cos() * radius;
                self.player.z = angle.sin() * radius;
            }
            MovementPattern::Random => unreachable!(),
        }

        self.player.x = self.player.x.clamp(-50.0, 50.0);
        self.player.z = self.player.z.clamp(-50.0, 50.0);

        self.player.update(delta);
    }

    pub fn is_dead(&self) -> bool {
        self.player.hp <= 0.0
    }
}

impl CombatEntity for SimulatedPlayer {
    fn entity_kind(&self) -> EntityKind {
        EntityKind::Bot
    }

    fn entity_id(&self) -> Option<u32> {
        self.player.entity_id()
    }

    fn entity_position(&self) -> Vec3 {
        self.player.entity_position()
    }

    fn hitboxes(&self) -> &[Hitbox] {
        self.player.hitboxes()
    }

    fn take_damage(&mut self, dmg: f32) {
        self.player.take_damage(dmg);
    }

    fn is_alive(&self) -> bool {
        self.player.is_alive()
    }

    fn update(&mut self, delta: f32) {
        self.simulate(delta);
    }

    fn hit_timer(&self) -> f32 {
        self.player.hit_timer()
    }

    fn entity_color(&self) -> Color {
        self.player.entity_color()
    }
}
