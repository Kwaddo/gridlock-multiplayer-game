use crate::physics::movement::{MovementConfig, MovementState};
use crate::screens::target_dummy::TargetDummy;
use crate::types::ai::simulated_player::SimulatedPlayer;
use crate::types::animated_model::AnimatedModel;
use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;
use crate::weapons::effects::{BulletTracer, DamageNumber};
use crate::weapons::WeaponInventory;

use macroquad::models::Mesh;
use macroquad::prelude::*;
use rodio::{OutputStream, OutputStreamHandle};
use std::collections::HashMap;
use std::sync::Arc;

use super::super::dev_mode::DevModeState;

pub struct PlayingState {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub last_mouse_pos: (f32, f32),
    pub other_players: Vec<OtherPlayer>,
    pub movement_state: MovementState,
    pub movement_config: MovementConfig,
    pub bob_timer: f32,
    pub audio_stream: Option<OutputStream>,
    pub audio_stream_handle: Option<OutputStreamHandle>,
    pub sound_samples: HashMap<String, Arc<[u8]>>,
    pub assets: HashMap<String, Texture2D>,
    pub models: HashMap<String, Mesh>,
    pub animated_models: HashMap<String, AnimatedModel>,
    pub asset_source_paths: HashMap<String, String>,
    pub move_dir: Vec3,
    pub jump_requested: bool,
    pub just_jumped: bool, // for precise jump animation timing
    pub just_landed: bool, // for precise landing animation timing
    pub current_world: WorldId,
    pub pending_world_change: Option<WorldId>,
    pub pending_start_request: bool,
    pub lobby_console_open: bool,
    pub pending_broken_walls: Vec<(u8, u8)>,
    pub pending_respawn: bool,
    pub pending_hits: Vec<(u32, f32)>,

    pub weapons: WeaponInventory,

    // countdown timers (server-synchronized)
    pub match_start_countdown_timer: f32,
    pub match_end_countdown_timer: f32,
    pub post_teleport_freeze_timer: f32, // seconds players cannot move immediately after teleport into Maze
    pub match_end_winner: Option<String>,

    pub target_dummy: TargetDummy,
    pub hit_marker_timer: f32,
    pub hit_marker_headshot: bool,
    pub damage_numbers: Vec<DamageNumber>,
    pub tracers: Vec<BulletTracer>,

    pub hit_marker_pellet_mask: u16,

    pub dev_mode: DevModeState,

    // respawn feedback (e.g. teleporter fallback message)
    pub spawn_feedback_message: Option<String>,
    pub spawn_feedback_timer: f32,

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
    pub local_kills: u32,
    pub connected_players: Vec<(u32, String, u32)>,

    // short bottom-left announcements (message, timer)
    pub announcements: Vec<(String, f32)>,
    pub lobby_console_message: Option<String>,
    pub lobby_console_message_timer: f32,
}

pub struct KillFeedEntry {
    pub killer_id: u32,
    pub victim_id: u32,
    pub weapon: String,
    pub timer: f32,
}

impl PlayingState {
    pub async fn new() -> Self {
        let (audio_stream, audio_stream_handle) = match OutputStream::try_default() {
            Ok((stream, handle)) => (Some(stream), Some(handle)),
            Err(err) => {
                println!("Failed to initialize audio output stream: {:?}", err);
                (None, None)
            }
        };

        Self {
            position: vec3(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            last_mouse_pos: (0.0, 0.0),
            other_players: Vec::new(),
            movement_state: MovementState::new(),
            movement_config: MovementConfig::new(),
            bob_timer: 0.0,
            audio_stream,
            audio_stream_handle,
            sound_samples: HashMap::new(),
            assets: HashMap::new(),
            models: HashMap::new(),
            animated_models: HashMap::new(),
            asset_source_paths: HashMap::new(),
            move_dir: Vec3::ZERO,
            jump_requested: false,
            just_jumped: false,
            just_landed: false,
            current_world: WorldId::Lobby,
            pending_world_change: None,
            pending_start_request: false,
            lobby_console_open: false,
            pending_broken_walls: Vec::new(),
            pending_respawn: false,
            pending_hits: Vec::new(),
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
            local_kills: 0,
            // countdown timers
            match_start_countdown_timer: 0.0,
            match_end_countdown_timer: 0.0,
            post_teleport_freeze_timer: 0.0,
            match_end_winner: None,
            spawn_feedback_message: None,
            spawn_feedback_timer: 0.0,
            lobby_console_message: None,
            lobby_console_message_timer: 0.0,
            announcements: Vec::new(),
            connected_players: Vec::new(),
        }
    }
}
