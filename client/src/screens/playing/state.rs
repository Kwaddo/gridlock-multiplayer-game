use crate::net::protocols::MODE_TEAM_DEATHMATCH;
use crate::physics::movement::{ MovementConfig, MovementState };
use crate::screens::target_dummy::TargetDummy;
use crate::types::ai::simulated_player::SimulatedPlayer;
use crate::types::animated_model::AnimatedModel;
use crate::types::player::OtherPlayer;
use crate::types::world::WorldId;
use crate::weapons::effects::{ BulletHole, BulletTracer, DamageNumber };
use crate::weapons::WeaponInventory;

use macroquad::models::Mesh;
use macroquad::prelude::*;
use rodio::{ OutputStream, OutputStreamHandle };
use std::collections::{ HashMap, HashSet };
use std::sync::Arc;

use super::super::dev_mode::DevModeState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandsPlaybackMode {
    IdleLoop,
    ReloadOnce,
    Stopped,
    // Knife-specific states (knife_animated.glb replaces hands entirely)
    KnifeIdle,
    KnifeSwing, // playing one of the swing segments, returns to KnifeIdle when done
    KnifeLower, // lowering animation when switching away from knife
    KnifeEquip, // raising animation when switching to knife
    // Sawnoff-specific states (sawnoff_animated.glb includes hands)
    SawnoffIdle,
    SawnoffShot, // playing shot animation, returns to SawnoffIdle when done
    SawnoffReload, // playing reload animation, returns to SawnoffIdle when done
    SawnoffEquip, // equip animation when switching to sawnoff
    SawnoffLower, // lowering animation when switching away from sawnoff
    // Minigun-specific states (minigun_animated.glb includes hands)
    MinigunIdle,
    MinigunShooting, // playing shoot loop
    MinigunReload, // playing reload animation
    MinigunEquip, // equip animation when switching to minigun
    MinigunLower, // lowering animation when switching away
}

pub struct PlayingState {
    // ---- pre‑allocated container hints to reduce reallocation spikes ----
    // typical counts observed in-game; tuning these avoids repeated grows
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub last_mouse_pos: (f32, f32),
    pub other_players: Vec<OtherPlayer>,
    /// mapping from player ID to index in `other_players` for fast lookup
    pub player_index: HashMap<u32, usize>,
    /// temporary reuse for world sync to avoid reallocating each tick
    pub seen_player_ids: HashSet<u32>,
    pub movement_state: MovementState,
    pub movement_config: MovementConfig,
    pub bob_timer: f32,
    pub audio_stream: Option<OutputStream>,
    pub audio_stream_handle: Option<OutputStreamHandle>,
    pub sound_samples: HashMap<String, Arc<[u8]>>,
    pub assets: HashMap<String, Texture2D>,
    // static meshes keyed by logical asset name; some glTF files contain multiple
    // primitives with different textures, so we now store a vector of meshes
    // instead of a single mesh.  rendering code will iterate when drawing.
    pub models: HashMap<String, Vec<Mesh>>,
    pub animated_models: HashMap<String, AnimatedModel>,
    pub asset_source_paths: HashMap<String, String>,
    pub move_dir: Vec3,
    pub jump_requested: bool,
    pub just_jumped: bool, // for precise jump animation timing
    pub just_landed: bool, // for precise landing animation timing
    pub current_world: WorldId,
    pub pending_world_change: Option<WorldId>,
    pub pending_start_request: bool,
    pub pending_start_gamemode: u8,
    pub pending_set_team: Option<u8>,
    pub lobby_console_open: bool,
    pub pending_broken_walls: Vec<(u8, u8)>,
    pub pending_respawn: bool,
    pub pending_hits: Vec<(u32, f32, bool)>, // victim_id, damage, backstab attempt?

    pub weapons: WeaponInventory,

    // countdown timers (server-synchronized)
    pub match_start_countdown_timer: f32,
    pub match_end_countdown_timer: f32,
    pub post_teleport_freeze_timer: f32, // seconds players cannot move immediately after teleport into Maze
    pub match_time_left: f32, // client-side remaining match time (seconds)
    pub match_end_winner: Option<String>,

    // authoritative/current match mode (None when not in a match)
    pub match_mode: Option<u8>,

    // temporarily hold a server-forced weapon slot until the player actually teleports into the Maze
    pub pending_forced_weapon_slot: Option<usize>,

    // damage / hit feedback (client-side)
    pub damage_flash_timer: f32, // short red flash when we take damage
    pub last_damage_attacker: Option<String>,
    pub stamina_depleted_vignette_alpha: f32,

    pub target_dummy: TargetDummy,
    pub hit_marker_timer: f32,
    pub hit_marker_headshot: bool,
    pub damage_numbers: Vec<DamageNumber>,
    pub tracers: Vec<BulletTracer>,
    pub bullet_holes: Vec<BulletHole>,

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
    pub local_deaths: u32,
    pub connected_players: Vec<(u32, String, u32, u32, u8, u8, String)>, // (id, name, kills, deaths, gun_level, team, weapon)

    // death screen: who killed us and with what weapon
    pub death_killer_name: Option<String>,
    pub death_weapon_name: Option<String>,

    // short bottom-left announcements (message, timer)
    pub announcements: Vec<(String, f32)>,
    pub lobby_console_message: Option<String>,
    pub lobby_console_message_timer: f32,
    pub hands_playback_mode: HandsPlaybackMode,
    pub hands_anim_initialized: bool,
    pub prev_hands_key: Option<String>, // tracks active hands rig key across swaps
    pub prev_was_reloading: bool, // edge detection for reload anim trigger
    pub prev_was_sliding: bool, // edge detection for slide start/end network events
    pub prev_any_slide_anim_active: bool, // edge detect for shared world slide clip playback

    pub knife_swing_index: u8, // cycles 0..2 for light slashes (swings 1-3)
    pub knife_swing_requested: bool, // set by input_processing when knife fires
    pub knife_heavy_swing_requested: bool, // set by input_processing for right-click heavy slash
    pub knife_anim_initialized: bool, // whether fp_hands_knife has been set up
    pub knife_attack_locked: bool, // true during swing
    pub knife_attack_lock_timer: f32, // seconds remaining in lock
    pub knife_miss_slow_timer: f32, // movement slow after miss
    pub knife_last_attack_was_heavy: bool, // for backstab tracking
    pub prev_weapon_was_knife: bool, // track weapon switches to/from knife

    pub sawnoff_shot_requested: bool, // set by input_processing when sawnoff fires
    pub sawnoff_anim_initialized: bool, // whether fp_hands_sawnoff has been set up
    pub prev_weapon_was_sawnoff: bool, // track weapon switches to/from sawnoff
    pub minigun_anim_initialized: bool,
    pub prev_weapon_was_minigun: bool,
    pub minigun_was_firing: bool, // tracks whether minigun was firing last frame

    // ── Search & Destroy state ──
    pub snd_round: u8,
    pub snd_max_rounds: u8,
    pub snd_red_wins: u8,
    pub snd_blue_wins: u8,
    pub snd_phase: u8,            // 0=freeze, 1=active, 2=bomb_planted, 3=round_over
    pub snd_attacking_team: u8,   // 1=Red, 2=Blue
    pub snd_bomb_carrier_id: u32, // 0 = no carrier
    pub snd_bomb_dropped: Option<(f32, f32, f32)>, // world pos if bomb on ground
    pub snd_bomb_site: (f32, f32, f32),
    pub snd_bomb_planted: bool,
    pub snd_plant_progress: f32,
    pub snd_defuse_progress: f32,
    pub snd_phase_timer: f32,
    pub snd_is_planting: bool,    // client is currently holding plant
    pub snd_is_defusing: bool,    // client is currently holding defuse

    // ceiling lights that have been shot out (cell coords)
    pub destroyed_lights: HashSet<(u8, u8)>,

    // cached strings used by render to avoid reallocating each frame
    pub(crate) match_start_cached_secs: i32,
    pub(crate) match_start_cached_text: String,
    pub(crate) post_freeze_cached_secs: i32,
    pub(crate) post_freeze_cached_text: String,

    // cached measurement for the static "TARGET DUMMY" HUD label
    pub target_dummy_label_dims: (f32, f32),
}

pub struct KillFeedEntry {
    pub killer_id: u32,
    pub victim_id: u32,
    pub killer_name: String,
    pub victim_name: String,
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
            other_players: Vec::with_capacity(Self::INIT_OTHER_PLAYERS),
            player_index: HashMap::with_capacity(Self::INIT_OTHER_PLAYERS),
            seen_player_ids: HashSet::with_capacity(Self::INIT_OTHER_PLAYERS),
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
            pending_start_gamemode: MODE_TEAM_DEATHMATCH,
            pending_set_team: None,
            lobby_console_open: false,
            pending_broken_walls: Vec::new(),
            pending_respawn: false,
            pending_hits: Vec::new(),
            weapons: WeaponInventory::new(),
            target_dummy: TargetDummy::new(vec3(-3.0, 0.0, -3.0)),
            hit_marker_timer: 0.0,
            hit_marker_headshot: false,
            damage_numbers: Vec::with_capacity(Self::INIT_DAMAGE_NUMBERS),
            tracers: Vec::with_capacity(Self::INIT_TRACERS),
            bullet_holes: Vec::with_capacity(Self::INIT_BULLET_HOLES),
            hit_marker_pellet_mask: 0,
            dev_mode: DevModeState::new(),
            practice_bots: Vec::with_capacity(Self::INIT_PRACTICE_BOTS),
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
            local_deaths: 0,
            death_killer_name: None,
            death_weapon_name: None,
            // countdown timers
            match_start_countdown_timer: 0.0,
            match_end_countdown_timer: 0.0,
            post_teleport_freeze_timer: 0.0,
            match_time_left: 0.0,
            match_end_winner: None,

            // damage / hit feedback (client-side)
            damage_flash_timer: 0.0,
            last_damage_attacker: None,
            stamina_depleted_vignette_alpha: 0.0,

            spawn_feedback_message: None,
            spawn_feedback_timer: 0.0,
            lobby_console_message: None,
            lobby_console_message_timer: 0.0,
            announcements: Vec::with_capacity(Self::INIT_ANNOUNCEMENTS),
            connected_players: Vec::new(),
            hands_playback_mode: HandsPlaybackMode::IdleLoop,
            hands_anim_initialized: false,
            prev_hands_key: None,
            prev_was_reloading: false,
            prev_was_sliding: false,
            prev_any_slide_anim_active: false,
            knife_swing_index: 0,
            knife_swing_requested: false,
            knife_heavy_swing_requested: false,
            knife_anim_initialized: false,
            knife_attack_locked: false,
            knife_attack_lock_timer: 0.0,
            knife_miss_slow_timer: 0.0,
            knife_last_attack_was_heavy: false,
            prev_weapon_was_knife: false,
            sawnoff_shot_requested: false,
            sawnoff_anim_initialized: false,
            prev_weapon_was_sawnoff: false,
            minigun_anim_initialized: false,
            prev_weapon_was_minigun: false,
            minigun_was_firing: false,

            // match mode unknown until server indicates otherwise
            match_mode: None,

            // pending forced weapon slot (applied after teleport into Maze)
            pending_forced_weapon_slot: None,

            // S&D state
            snd_round: 0,
            snd_max_rounds: 7,
            snd_red_wins: 0,
            snd_blue_wins: 0,
            snd_phase: 0,
            snd_attacking_team: 0,
            snd_bomb_carrier_id: 0,
            snd_bomb_dropped: None,
            snd_bomb_site: (0.0, 0.0, 0.0),
            snd_bomb_planted: false,
            snd_plant_progress: 0.0,
            snd_defuse_progress: 0.0,
            snd_phase_timer: 0.0,
            snd_is_planting: false,
            snd_is_defusing: false,

            destroyed_lights: HashSet::new(),

            match_start_cached_secs: -1,
            match_start_cached_text: String::new(),
            post_freeze_cached_secs: -1,
            post_freeze_cached_text: String::new(),
            target_dummy_label_dims: {
                let m = crate::ui::fonts::measure_game_text("TARGET DUMMY", 14);
                (m, 0.0) // width only used currently; height can be computed if needed
            },
        }
    }
}

// associated constants for preallocated container hints
impl PlayingState {
    pub const INIT_OTHER_PLAYERS: usize = 32;
    pub const INIT_PRACTICE_BOTS: usize = 16;
    pub const INIT_DAMAGE_NUMBERS: usize = 64;
    pub const INIT_TRACERS: usize = 128;
    pub const INIT_BULLET_HOLES: usize = 256;
    pub const INIT_ANNOUNCEMENTS: usize = 8;
}

// helper methods that don't involve constants
impl PlayingState {
    /// rebuild the fast lookup map after modifying the `other_players` vector.
    pub fn rebuild_player_index(&mut self) {
        self.player_index.clear();
        for (i, p) in self.other_players.iter().enumerate() {
            self.player_index.insert(p.id, i);
        }
    }
}
