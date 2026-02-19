/*
    This module is used to simulate an actual real network player for testing
    this "player" will follow instructions from user to mvoe around and shoot
    will be used to test combat mechanics and hit detection without needing to connect to a
    real server or have another player, will also serve well for testing player models later on
*/

use crate::types::animation_state::AnimState;
use crate::types::player::{OtherPlayer, PlayerSkin};
use crate::types::world::WorldId;
use crate::weapons::combat::{CombatEntity, EntityKind, Hitbox};
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
    pub sprint: bool,
    pub slide: bool,
    pub shoot: bool,
    pub ads: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum YawMode {
    Fixed,      // stays at current yaw
    FacePlayer, // always rotates to face the local player
    Spin,       // continuously rotates
}

pub struct SimulatedPlayer {
    pub player: OtherPlayer,
    pub pattern: MovementPattern,
    pub action_state: ActionState,
    pub yaw_mode: YawMode,
    pub skin_override: Option<PlayerSkin>,
    movement_timer: f32,
    pattern_change_timer: f32,
    current_random_pattern: MovementPattern,
    jump_event_timer: f32,
    circle_center: Vec2,
}

impl SimulatedPlayer {
    pub fn new(id: u32, username: String, world: WorldId, spawn_pos: Vec3) -> Self {
        let mut player = OtherPlayer::new(id, username, world);
        player.x = spawn_pos.x;
        player.y = spawn_pos.y;
        player.z = spawn_pos.z;
        player.apply_snapshot(spawn_pos.x, spawn_pos.y, spawn_pos.z, player.yaw);

        Self {
            player,
            pattern: MovementPattern::None,
            action_state: ActionState {
                jump_spam: false,
                crouch: false,
                sprint: false,
                slide: false,
                shoot: false,
                ads: false,
            },
            yaw_mode: YawMode::Fixed,
            skin_override: None,
            movement_timer: 0.0,
            pattern_change_timer: gen_range(3.0, 8.0),
            current_random_pattern: MovementPattern::ForwardBack,
            jump_event_timer: 0.0,
            circle_center: vec2(spawn_pos.x, spawn_pos.z),
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

    pub fn set_yaw(&mut self, yaw: f32) {
        self.player.yaw = yaw;
    }

    pub fn simulate_yaw(&mut self, delta: f32, player_pos: Option<Vec3>) {
        match self.yaw_mode {
            YawMode::Fixed => {} // manual control only
            YawMode::FacePlayer => {
                if let Some(pp) = player_pos {
                    let dx = pp.x - self.player.x;
                    let dz = pp.z - self.player.z;
                    // Match gameplay yaw convention (forward = (cos(yaw), sin(yaw))).
                    self.player.yaw = dz.atan2(dx);
                }
            }
            YawMode::Spin => {
                self.player.yaw += 1.5 * delta; // ~90 deg/sec
            }
        }
    }

    pub fn simulate(&mut self, delta: f32) {
        self.movement_timer += delta;
        let prev_x = self.player.x;
        let prev_z = self.player.z;
        self.player.is_sprinting = self.action_state.sprint;
        self.player.is_crouching = self.action_state.crouch;
        self.player.is_ads = self.action_state.ads;

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

        // ensure we never match on MovementPattern::Random in the movement switch
        let resolved_pattern = if active_pattern == MovementPattern::Random {
            self.current_random_pattern
        } else {
            active_pattern
        };

        let move_speed = 3.0;
        let oscillation_period = 2.0;
        let forward = vec2(self.player.yaw.cos(), self.player.yaw.sin());
        let right = vec2(-forward.y, forward.x);

        match resolved_pattern {
            MovementPattern::None => {}
            MovementPattern::ForwardBack => {
                let t =
                    (self.movement_timer / oscillation_period * std::f32::consts::PI * 2.0).sin();
                self.player.x += forward.x * t * move_speed * delta;
                self.player.z += forward.y * t * move_speed * delta;
            }
            MovementPattern::SideToSide => {
                let t =
                    (self.movement_timer / oscillation_period * std::f32::consts::PI * 2.0).sin();
                self.player.x += right.x * t * move_speed * delta;
                self.player.z += right.y * t * move_speed * delta;
            }
            MovementPattern::Forward => {
                self.player.x += forward.x * move_speed * delta;
                self.player.z += forward.y * move_speed * delta;
            }
            MovementPattern::Back => {
                self.player.x -= forward.x * move_speed * delta;
                self.player.z -= forward.y * move_speed * delta;
            }
            MovementPattern::Left => {
                self.player.x -= right.x * move_speed * delta;
                self.player.z -= right.y * move_speed * delta;
            }
            MovementPattern::Right => {
                self.player.x += right.x * move_speed * delta;
                self.player.z += right.y * move_speed * delta;
            }
            MovementPattern::Circle => {
                // Keep circle movement close for easier visual/aim testing.
                let radius = 2.0;
                let angle = self.movement_timer / oscillation_period * std::f32::consts::PI * 2.0;
                self.player.x = self.circle_center.x + angle.cos() * radius;
                self.player.z = self.circle_center.y + angle.sin() * radius;
            }
            MovementPattern::Random => {} // handled by resolved_pattern — defensively do nothing here
        }

        self.player.x = self.player.x.clamp(-50.0, 50.0);
        self.player.z = self.player.z.clamp(-50.0, 50.0);

        let mut jump_started = false;
        if self.action_state.jump_spam {
            self.jump_event_timer -= delta;
            if self.jump_event_timer <= 0.0 {
                jump_started = true;
                self.jump_event_timer = 0.75;
            }
        } else {
            self.jump_event_timer = 0.0;
        }

        let anim_state = AnimState::from_movement(
            prev_x,
            prev_z,
            self.player.x,
            self.player.z,
            self.player.yaw,
            self.action_state.sprint,
            self.action_state.ads,
            self.action_state.crouch,
            false,
            self.action_state.slide,
            jump_started,
        );
        self.player.skin_type = AnimState::skin_from_state(anim_state, self.skin_override);

        // Keep render snapshot in sync so model draw position matches gameplay position.
        self.player
            .apply_snapshot(self.player.x, self.player.y, self.player.z, self.player.yaw);
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
