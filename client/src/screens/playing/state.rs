use std::collections::HashMap;

use crate::physics::movement::{MovementConfig, MovementState};
use crate::screens::dev_mode::DevModeState;
use crate::screens::target_dummy::TargetDummy;
use crate::types::ai::simulated_player::SimulatedPlayer;
use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;
use crate::weapons::effects::{BulletTracer, DamageNumber};
use crate::weapons::state::WeaponInventory;
use macroquad::models::Mesh;
use macroquad::prelude::*;

pub struct KillFeedEntry {
    pub killer_id: u32,
    pub victim_id: u32,
    pub weapon: String,
    pub timer: f32,
}

pub struct PlayingState {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub last_mouse_pos: (f32, f32),
    pub other_players: Vec<OtherPlayer>,
    pub movement_state: MovementState,
    pub movement_config: MovementConfig,
    pub bob_timer: f32,
    pub assets: HashMap<String, Texture2D>,
    pub models: HashMap<String, Mesh>,
    pub move_dir: Vec3,
    pub jump_requested: bool,
    pub current_world: WorldId,
    pub pending_world_change: Option<WorldId>,
    pub pending_broken_walls: Vec<(u8, u8)>,
    pub pending_respawn: bool,
    pub weapons: WeaponInventory,
    pub target_dummy: TargetDummy,
    pub hit_marker_timer: f32,
    pub hit_marker_headshot: bool,
    pub damage_numbers: Vec<DamageNumber>,
    pub tracers: Vec<BulletTracer>,
    pub hit_marker_pellet_mask: u16,
    pub dev_mode: DevModeState,
    pub practice_bots: Vec<SimulatedPlayer>,
    pub next_bot_id: u32,
    pub hp: f32,
    pub shield: f32,
    pub max_shield: f32,
    pub max_hp: f32,
    pub is_dead: bool,
    pub death_timer: f32,
    pub invulnerability_timer: f32,
    pub kill_feed: Vec<KillFeedEntry>,
}

impl PlayingState {
    pub async fn new() -> Self {
        Self {
            position: vec3(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            last_mouse_pos: (0.0, 0.0),
            other_players: Vec::new(),
            movement_state: MovementState::new(),
            movement_config: MovementConfig::new(),
            bob_timer: 0.0,
            assets: HashMap::new(),
            models: HashMap::new(),
            move_dir: Vec3::ZERO,
            jump_requested: false,
            current_world: WorldId::Lobby,
            pending_world_change: None,
            pending_broken_walls: Vec::new(),
            pending_respawn: false,
            weapons: WeaponInventory::new(),
            target_dummy: TargetDummy::new(vec3(-3.0, 0.0, -3.0)),
            hit_marker_timer: 0.0,
            hit_marker_headshot: false,
            damage_numbers: Vec::new(),
            tracers: Vec::new(),
            hit_marker_pellet_mask: 0,
            dev_mode: DevModeState::new(),
            practice_bots: Vec::new(),
            next_bot_id: 9000,
            hp: 100.0,
            max_hp: 100.0,
            shield: 0.0,
            max_shield: 100.0,
            is_dead: false,
            death_timer: 0.0,
            invulnerability_timer: 0.0,
            kill_feed: Vec::new(),
        }
    }
}
