use crate::core::GameConfig;
use crate::physics::collision;
use crate::screens::dev_mode::HandsAnimRequest;
use crate::types::animated_model::AnimatedModel;
use crate::types::maze::Maze;
use crate::types::player::{OtherPlayer, PlayerSkin};
use crate::types::world::WorldId;
use crate::weapons::combat::{default_player_hitboxes, CombatEntity, HitZone, Hitbox};
use crate::weapons::stats::{WeaponCategory, WeaponId};

use macroquad::prelude::*;
use std::collections::HashMap;

use super::state::HandsPlaybackMode;
use super::PlayingState;

// ── Footstep constants ──
/// Maximum distance (world units) at which footsteps are audible
const FOOTSTEP_MAX_DIST: f32 = 30.0;
/// Footstep intervals (seconds between steps)
const FOOTSTEP_INTERVAL_WALK: f32 = 0.50;
const FOOTSTEP_INTERVAL_SPRINT: f32 = 0.32;
const FOOTSTEP_INTERVAL_CROUCH: f32 = 0.72;
/// Base volumes per movement type (before distance attenuation)
const FOOTSTEP_VOLUME_WALK: f32 = 0.45;
const FOOTSTEP_VOLUME_SPRINT: f32 = 0.75;
const FOOTSTEP_VOLUME_CROUCH: f32 = 0.22;
/// Slide sound volume
const SLIDE_SOUND_VOLUME: f32 = 0.6;

const WORLD_PLAYER_RENDER_SCALE: f32 = 0.90;
const PLAYER_MODEL_YAW_OFFSET: f32 = std::f32::consts::FRAC_PI_2;
const HANDS_RELOAD_START_SEC: f32 = 0.0;
const HANDS_RELOAD_END_SEC: f32 = 1.85;
const PISTOL_RELOAD_START_SEC: f32 = 0.32;
const PISTOL_RELOAD_END_SEC: f32 = 2.20;
#[allow(dead_code)]
const PISTOL_EQUIP_START_SEC: f32 = 4.70;
#[allow(dead_code)]
const PISTOL_EQUIP_END_SEC: f32 = 5.90;
const SMG_RELOAD_START_SEC: f32 = 2.71;
const SMG_RELOAD_END_SEC: f32 = 4.32;
#[allow(dead_code)]
const SMG_EQUIP_START_SEC: f32 = 4.75;
#[allow(dead_code)]
const SMG_EQUIP_END_SEC: f32 = 5.81;
const SNIPER_RELOAD_START_SEC: f32 = 1.66;
const SNIPER_RELOAD_END_SEC: f32 = 3.26;
#[allow(dead_code)]
const SNIPER_EQUIP_START_SEC: f32 = 3.61;
#[allow(dead_code)]
const SNIPER_EQUIP_END_SEC: f32 = 4.86;

// Minigun hand clip segments (minigun_animated.glb)
#[allow(dead_code)]
const MINIGUN_IDLE_START_SEC: f32 = 0.00;
#[allow(dead_code)]
const MINIGUN_IDLE_END_SEC: f32 = 0.00;
const MINIGUN_SHOOT_LOOP_START_SEC: f32 = 0.01;
const MINIGUN_SHOOT_LOOP_END_SEC: f32 = 1.17;
const MINIGUN_RELOAD_START_SEC: f32 = 2.06;
const MINIGUN_RELOAD_END_SEC: f32 = 4.29;
#[allow(dead_code)]
const MINIGUN_EQUIP_START_SEC: f32 = 4.73;
#[allow(dead_code)]
const MINIGUN_EQUIP_END_SEC: f32 = 5.30;

// Sawnoff hand clip segments (sawnoff_animated.glb)
#[allow(dead_code)]
const SAWNOFF_IDLE_START_SEC: f32 = 0.00;
#[allow(dead_code)]
const SAWNOFF_IDLE_END_SEC: f32 = 0.00;
const SAWNOFF_SHOT_START_SEC: f32 = 0.01;
const SAWNOFF_SHOT_END_SEC: f32 = 0.41;
const SAWNOFF_RELOAD_START_SEC: f32 = 0.47;
const SAWNOFF_RELOAD_END_SEC: f32 = 3.57;
#[allow(dead_code)]
const SAWNOFF_EQUIP_START_SEC: f32 = 6.88;
#[allow(dead_code)]
const SAWNOFF_EQUIP_END_SEC: f32 = 7.30;
// Carbine all-in-one clip segments (fps_animated_carbine.glb).
#[allow(dead_code)]
const CARBINE_MELEE_HIT_START_SEC: f32 = 6.79;
#[allow(dead_code)]
const CARBINE_MELEE_HIT_END_SEC: f32 = 7.40;

const SLIDE_CLIP_START_SEC: f32 = 0.0;
const SLIDE_CLIP_END_SEC: f32 = 30.0 / 30.0; // frame 0..30 at 30 FPS

impl PlayingState {
    /// Player animations something is drawing now: other players, practice
    /// bots, and our own corpse during the death camera. Only these need
    /// skinning; their joints also drive those players' hitboxes.
    fn player_anims_in_use(&self) -> std::collections::HashSet<&'static str> {
        let skins = self
            .other_players
            .iter()
            .map(|p| p.skin_type)
            .chain(self.practice_bots.iter().map(|b| b.player.skin_type));
        let mut keys: std::collections::HashSet<&'static str> = skins
            .filter_map(|skin| skin.model_key())
            .map(|key| if self.animated_models.contains_key(key) { key } else { "anim_rifle_idle" })
            .collect();
        // Players drawn with a missing model fall back to the idle clip.
        keys.insert("anim_rifle_idle");       

        if !self.mirrors.is_empty() {
            if let Some(key) = self.self_avatar_skin().model_key() {
                if self.animated_models.contains_key(key) {
                    keys.insert(key);
                }
            }
        }

        if !matches!(self.death_camera_phase, super::state::DeathCameraPhase::None) {
            keys.extend(
                [PlayerSkin::StandDeath, PlayerSkin::CrouchDeath, PlayerSkin::ProneDeath]
                    .into_iter()
                    .filter_map(|skin| skin.model_key()),
            );
        }
        keys
    }

    /// The first-person hands rig `id` is drawn with (the asset loader
    /// loads it before the player spawns with that weapon).
    pub fn preferred_hands_key_for_weapon(id: WeaponId) -> &'static str {
        if id == WeaponId::Minigun {
            return "fp_hands_minigun";
        }
        if id == WeaponId::Sawnoff {
            return "fp_hands_sawnoff";
        }
        match id.category() {
            WeaponCategory::Pistol => "fp_hands_pistol",
            WeaponCategory::Smg => "fp_hands_smg",
            WeaponCategory::Sniper => "fp_hands_sniper",
            _ => "fp_hands",
        }
    }

    fn reload_segment_for_hands_key(hands_key: &str) -> (f32, f32) {
        match hands_key {
            "fp_hands_pistol" => (PISTOL_RELOAD_START_SEC, PISTOL_RELOAD_END_SEC),
            "fp_hands_smg" => (SMG_RELOAD_START_SEC, SMG_RELOAD_END_SEC),
            "fp_hands_sniper" => (SNIPER_RELOAD_START_SEC, SNIPER_RELOAD_END_SEC),
            "fp_hands_minigun" => (MINIGUN_RELOAD_START_SEC, MINIGUN_RELOAD_END_SEC),
            "fp_hands_sawnoff" => (SAWNOFF_RELOAD_START_SEC, SAWNOFF_RELOAD_END_SEC),
            _ => (HANDS_RELOAD_START_SEC, HANDS_RELOAD_END_SEC),
        }
    }

    fn is_all_in_one_hands_source(path: &str) -> bool {
        path.contains("fps_animated_carbine.glb")
            || path.contains("fps_animated_smg.glb")
            || path.contains("sniper_animated.glb")
            || path.contains("pistol_animated.glb")
            || path.contains("minigun_animated.glb")
            || path.contains("sawnoff_animated.glb")
    }

    #[inline(always)]
    pub fn simulate(&mut self, config: &GameConfig, maze: Option<&Maze>) {
        let raw_delta = get_frame_time();
        if raw_delta > 0.20 {
            self.move_dir = Vec3::ZERO;
            self.jump_requested = false;
            self.just_jumped = false;
            self.just_landed = false;
            self.movement_state.velocity = Vec3::ZERO;
            return;
        }

        let delta: f32 = raw_delta.min(0.05);
        self.just_jumped = false;
        self.just_landed = false;

        if self.dev_mode.ruler_set_start_requested {
            self.dev_mode.ruler_start = Some(self.position);
            self.dev_mode.ruler_set_start_requested = false;
        }
        if self.dev_mode.ruler_set_end_requested {
            self.dev_mode.ruler_end = Some(self.position);
            self.dev_mode.ruler_set_end_requested = false;
        }

        // update kill feed timers and drop expired entries in a single pass
        self.kill_feed.retain_mut(|entry| {
            entry.timer -= delta;
            entry.timer > 0.0
        });

        // tick kill confirmation timer
        if self.kill_confirm_timer > 0.0 {
            self.kill_confirm_timer = (self.kill_confirm_timer - delta).max(0.0);
        }

        // tick death animation timers on other players; remove when expired
        let mut need_rebuild = false;
        self.other_players.retain_mut(|p| {
            if let Some(ref mut t) = p.death_timer {
                *t -= delta;
                if *t <= 0.0 {
                    need_rebuild = true;
                    return false; // remove
                }
            }
            true
        });
        if need_rebuild {
            self.rebuild_player_index();
        }

        if self.is_dead {
            if self.death_timer > 0.0 {
                self.death_timer -= delta;
            } else if self.current_world == WorldId::Maze
                && self.match_mode != Some(crate::net::protocols::MODE_SEARCH_AND_DESTROY)
            {
                // Auto-respawn once the death timer expires (not in S&D — round start handles it)
                self.pending_respawn = true;
            }

            // Tick death camera phase
            use super::state::DeathCameraPhase;
            match self.death_camera_phase {
                DeathCameraPhase::ZoomOut { timer, corpse_pos, corpse_yaw } => {
                    let new_timer = timer + delta;
                    if new_timer >= 0.5 {
                        self.death_camera_phase = DeathCameraPhase::Watching {
                            timer: 0.0,
                            corpse_pos,
                            corpse_yaw,
                        };
                    } else {
                        self.death_camera_phase = DeathCameraPhase::ZoomOut {
                            timer: new_timer,
                            corpse_pos,
                            corpse_yaw,
                        };
                    }
                }
                DeathCameraPhase::Watching { timer, corpse_pos, corpse_yaw } => {
                    let new_timer = timer + delta;
                    if new_timer >= 2.0 {
                        self.death_camera_phase = DeathCameraPhase::Done;
                    } else {
                        self.death_camera_phase = DeathCameraPhase::Watching {
                            timer: new_timer,
                            corpse_pos,
                            corpse_yaw,
                        };
                    }
                }
                _ => {}
            }
            // Keep other players' positions smoothed and their animations running
            // while we're spectating — skipping this made everyone look frozen.
            for p in &mut self.other_players {
                p.update(delta);
            }
            let in_use = self.player_anims_in_use();
            for (key, model) in self.animated_models.iter_mut() {
                if in_use.contains(key.as_str()) {
                    model.update(delta);
                } else if key.starts_with("anim_") {
                    model.advance(delta);
                }
            }
            for bot in &mut self.practice_bots {
                bot.simulate_yaw(delta, Some(self.position));
                bot.update(delta);
            }
            return;
        }

        self.weapons.update(delta);

        // Auto-switch to another weapon when current is empty (all modes except Gun Game)
        if self.match_mode != Some(crate::net::protocols::MODE_GUN_GAME) {
            self.weapons.auto_switch_on_empty();
        }

        let active_weapon_id = self.weapons.active().id;
        let reloading_started = self.weapons.active().is_reloading && !self.prev_was_reloading;
        let uses_special_hands_machine = matches!(
            active_weapon_id,
            WeaponId::Knife | WeaponId::Sawnoff | WeaponId::Minigun
        );

        if !uses_special_hands_machine {
            let preferred_hands_key = Self::preferred_hands_key_for_weapon(active_weapon_id);
            let hands_key = if self.animated_models.contains_key(preferred_hands_key)
                || self.models.contains_key(preferred_hands_key)
            {
                preferred_hands_key
            } else {
                "fp_hands"
            };
            let hands_is_all_in_one = self
                .asset_source_paths
                .get(hands_key)
                .map(|p| Self::is_all_in_one_hands_source(p))
                .unwrap_or(false);
            let (reload_start, reload_end) = Self::reload_segment_for_hands_key(hands_key);
            let hands_key_changed = self.prev_hands_key.as_deref() != Some(hands_key);

            if let Some(hands) = self.animated_models.get_mut(hands_key) {
                if !self.hands_anim_initialized || hands_key_changed {
                    if hands_is_all_in_one {
                        // This file has a single combined clip ("allanims"), so idle is a held pose.
                        hands.stop_at_start();
                        self.hands_playback_mode = HandsPlaybackMode::Stopped;
                    } else {
                        hands.play_from_start(true);
                        self.hands_playback_mode = HandsPlaybackMode::IdleLoop;
                    }
                    self.hands_anim_initialized = true;
                }

                if let Some(request) = self.dev_mode.hands_anim_request.take() {
                    match request {
                        HandsAnimRequest::IdleLoop => {
                            if hands_is_all_in_one {
                                hands.stop_at_start();
                                self.hands_playback_mode = HandsPlaybackMode::Stopped;
                            } else {
                                hands.play_from_start(true);
                                self.hands_playback_mode = HandsPlaybackMode::IdleLoop;
                            }
                        }
                        HandsAnimRequest::ReloadOnce => {
                            if active_weapon_id.category() != WeaponCategory::Pistol {
                                hands.play_segment(reload_start, reload_end, false);
                                self.hands_playback_mode = HandsPlaybackMode::ReloadOnce;
                            }
                        }
                        HandsAnimRequest::Stop => {
                            hands.stop_at_start();
                            self.hands_playback_mode = HandsPlaybackMode::Stopped;
                        }
                    }
                }

                if reloading_started && active_weapon_id.category() != WeaponCategory::Pistol {
                    hands.play_segment(reload_start, reload_end, false);
                    self.hands_playback_mode = HandsPlaybackMode::ReloadOnce;
                }

                if self.hands_playback_mode == HandsPlaybackMode::ReloadOnce
                    && hands.is_playback_finished()
                {
                    if hands_is_all_in_one {
                        hands.stop_at_start();
                        self.hands_playback_mode = HandsPlaybackMode::Stopped;
                    } else {
                        hands.play_from_start(true);
                        self.hands_playback_mode = HandsPlaybackMode::IdleLoop;
                    }
                }
            }
        }

        // --- Sawnoff animation state machine ---
        // Like the knife, sawnoff uses its own animated model that includes hands.
        let is_sawnoff = self.weapons.active().id == WeaponId::Sawnoff;
        let was_sawnoff = self.prev_weapon_was_sawnoff;

        if is_sawnoff {
            if let Some(sawnoff) = self.animated_models.get_mut("fp_hands_sawnoff") {
                // First time equipping sawnoff, or switching TO sawnoff
                if !self.sawnoff_anim_initialized || (!was_sawnoff && is_sawnoff) {
                    sawnoff.stop_at_start();
                    self.hands_playback_mode = HandsPlaybackMode::SawnoffIdle;
                    self.sawnoff_anim_initialized = true;
                }

                // Shot request
                if self.sawnoff_shot_requested {
                    self.sawnoff_shot_requested = false;
                    sawnoff.play_segment(SAWNOFF_SHOT_START_SEC, SAWNOFF_SHOT_END_SEC, false);
                    self.hands_playback_mode = HandsPlaybackMode::SawnoffShot;
                }
                if reloading_started {
                    self.sawnoff_shot_requested = false;
                    sawnoff.play_segment(SAWNOFF_RELOAD_START_SEC, SAWNOFF_RELOAD_END_SEC, false);
                    self.hands_playback_mode = HandsPlaybackMode::SawnoffReload;
                }

                // Transition back to idle when shot/equip/reload finishes
                match self.hands_playback_mode {
                    HandsPlaybackMode::SawnoffShot
                    | HandsPlaybackMode::SawnoffEquip => {
                        if sawnoff.is_playback_finished() {
                            sawnoff.stop_at_start();
                            self.hands_playback_mode = HandsPlaybackMode::SawnoffIdle;
                        }
                    }
                    HandsPlaybackMode::SawnoffReload => {
                        if sawnoff.is_playback_finished() {
                            sawnoff.stop_at_start();
                            self.hands_playback_mode = HandsPlaybackMode::SawnoffIdle;
                        }
                    }
                    HandsPlaybackMode::SawnoffIdle => {} // held pose, nothing to do
                    _ => {
                        sawnoff.stop_at_start();
                        self.hands_playback_mode = HandsPlaybackMode::SawnoffIdle;
                    }
                }
            }
        } else if was_sawnoff {
            // Just switched away from sawnoff — no lower segment defined yet,
            // so just reset state immediately.
            self.sawnoff_anim_initialized = false;
            self.sawnoff_shot_requested = false;
            self.hands_playback_mode = HandsPlaybackMode::SawnoffLower;
            // Immediately transition out since there is no lower anim clip.
            let preferred_hands_key = Self::preferred_hands_key_for_weapon(self.weapons.active().id);
            let hands_key = if self.animated_models.contains_key(preferred_hands_key)
                || self.models.contains_key(preferred_hands_key)
            {
                preferred_hands_key
            } else {
                "fp_hands"
            };
            let hands_is_all_in_one = self
                .asset_source_paths
                .get(hands_key)
                .map(|p| Self::is_all_in_one_hands_source(p))
                .unwrap_or(false);
            if let Some(hands) = self.animated_models.get_mut(hands_key) {
                if hands_is_all_in_one {
                    hands.stop_at_start();
                    self.hands_playback_mode = HandsPlaybackMode::Stopped;
                } else {
                    hands.play_from_start(true);
                    self.hands_playback_mode = HandsPlaybackMode::IdleLoop;
                }
            } else {
                self.hands_playback_mode = HandsPlaybackMode::IdleLoop;
            }
        } else {
            self.sawnoff_shot_requested = false;
        }
        self.prev_weapon_was_sawnoff = is_sawnoff;

        // --- Minigun animation state machine ---
        let is_minigun = self.weapons.active().id == WeaponId::Minigun;
        let was_minigun = self.prev_weapon_was_minigun;

        if is_minigun {
            if let Some(minigun) = self.animated_models.get_mut("fp_hands_minigun") {
                if !self.minigun_anim_initialized || (!was_minigun && is_minigun) {
                    minigun.stop_at_start();
                    self.hands_playback_mode = HandsPlaybackMode::MinigunIdle;
                    self.minigun_anim_initialized = true;
                    self.minigun_was_firing = false;
                }

                // Shooting loop — play when firing, stop when not
                let is_firing = self.weapons.active().fire_cooldown > 0.0
                    && !self.weapons.active().is_reloading;
                if is_firing && !self.minigun_was_firing
                    && self.hands_playback_mode != HandsPlaybackMode::MinigunReload
                {
                    minigun.play_segment(
                        MINIGUN_SHOOT_LOOP_START_SEC,
                        MINIGUN_SHOOT_LOOP_END_SEC,
                        true,
                    );
                    self.hands_playback_mode = HandsPlaybackMode::MinigunShooting;
                }
                if reloading_started {
                    minigun.play_segment(MINIGUN_RELOAD_START_SEC, MINIGUN_RELOAD_END_SEC, false);
                    self.hands_playback_mode = HandsPlaybackMode::MinigunReload;
                }
                self.minigun_was_firing = is_firing;

                // Transitions
                match self.hands_playback_mode {
                    HandsPlaybackMode::MinigunEquip => {
                        if minigun.is_playback_finished() {
                            minigun.stop_at_start();
                            self.hands_playback_mode = HandsPlaybackMode::MinigunIdle;
                        }
                    }
                    HandsPlaybackMode::MinigunReload => {
                        if minigun.is_playback_finished() {
                            minigun.stop_at_start();
                            self.hands_playback_mode = HandsPlaybackMode::MinigunIdle;
                        }
                    }
                    HandsPlaybackMode::MinigunShooting => {
                        if !is_firing {
                            minigun.stop_at_start();
                            self.hands_playback_mode = HandsPlaybackMode::MinigunIdle;
                        }
                    }
                    HandsPlaybackMode::MinigunIdle => {}
                    _ => {
                        minigun.stop_at_start();
                        self.hands_playback_mode = HandsPlaybackMode::MinigunIdle;
                    }
                }
            }
        } else if was_minigun {
            self.minigun_anim_initialized = false;
            self.minigun_was_firing = false;
            self.hands_playback_mode = HandsPlaybackMode::MinigunLower;
            let preferred_hands_key = Self::preferred_hands_key_for_weapon(self.weapons.active().id);
            let hands_key = if self.animated_models.contains_key(preferred_hands_key)
                || self.models.contains_key(preferred_hands_key)
            {
                preferred_hands_key
            } else {
                "fp_hands"
            };
            let hands_is_all_in_one = self
                .asset_source_paths
                .get(hands_key)
                .map(|p| Self::is_all_in_one_hands_source(p))
                .unwrap_or(false);
            if let Some(hands) = self.animated_models.get_mut(hands_key) {
                if hands_is_all_in_one {
                    hands.stop_at_start();
                    self.hands_playback_mode = HandsPlaybackMode::Stopped;
                } else {
                    hands.play_from_start(true);
                    self.hands_playback_mode = HandsPlaybackMode::IdleLoop;
                }
            } else {
                self.hands_playback_mode = HandsPlaybackMode::IdleLoop;
            }
        }
        self.prev_weapon_was_minigun = is_minigun;

        // Advance world/player clips plus only the active first-person hands model.
        let active_hands_key = if is_sawnoff {
            Some("fp_hands_sawnoff")
        } else if is_minigun {
            Some("fp_hands_minigun")
        } else {
            let preferred = Self::preferred_hands_key_for_weapon(active_weapon_id);
            if self.animated_models.contains_key(preferred) || self.models.contains_key(preferred) {
                Some(preferred)
            } else {
                Some("fp_hands")
            }
        };

        let in_use = self.player_anims_in_use();
        for (key, model) in self.animated_models.iter_mut() {
            // Player animations nobody shows right now only keep time (skinning
            // all of them was most of the frame).
            if key.starts_with("anim_") && !in_use.contains(key.as_str()) {
                model.advance(delta);
                continue;
            }
            let should_update = if key.starts_with("anim_") {
                true
            } else if key == "fp_hands_sawnoff" {
                is_sawnoff
            } else if key == "fp_hands_minigun" {
                is_minigun
            } else if key == "fp_hands"
                || key == "fp_hands_pistol"
                || key == "fp_hands_smg"
                || key == "fp_hands_sniper"
            {
                !uses_special_hands_machine && active_hands_key == Some(key.as_str())
            } else {
                false
            };
            if should_update {
                model.update(delta);
            }
        }
        self.prev_hands_key = active_hands_key.map(|k| k.to_string());

        self.target_dummy.update(delta);

        // update other players and collect footstep sounds to play
        let mut footstep_sounds: Vec<(Vec3, Vec3, f32)> = Vec::new(); // (pos, vel, volume)
        let mut slide_sounds: Vec<(Vec3, Vec3)> = Vec::new();
        for p in &mut self.other_players {
            p.update(delta);

            // skip dead players
            if p.hp <= 0.0 {
                continue;
            }

            // determine movement state for footstep sounds
            let is_moving = p.last_move_dir != crate::types::animation_state::MoveDir::None;
            let is_sliding = p.is_sliding;

            if is_sliding {
                // slide sound plays once at start (timer acts as "played" flag)
                if p.footstep_timer <= 0.0 {
                    let pos = p.render_position();
                    let vel = vec3(p.render_x - p.prev_x, 0.0, p.render_z - p.prev_z) / delta.max(0.001);
                    slide_sounds.push((pos, vel));
                    p.footstep_timer = 1.0; // prevent re-triggering during same slide
                }
            } else if is_moving {
                p.footstep_timer -= delta;
                if p.footstep_timer <= 0.0 {
                    let (interval, volume) = if p.is_sprinting {
                        (FOOTSTEP_INTERVAL_SPRINT, FOOTSTEP_VOLUME_SPRINT)
                    } else if p.is_crouching {
                        (FOOTSTEP_INTERVAL_CROUCH, FOOTSTEP_VOLUME_CROUCH)
                    } else {
                        (FOOTSTEP_INTERVAL_WALK, FOOTSTEP_VOLUME_WALK)
                    };
                    let pos = p.render_position();
                    let vel = vec3(p.render_x - p.prev_x, 0.0, p.render_z - p.prev_z) / delta.max(0.001);
                    footstep_sounds.push((pos, vel, volume));
                    p.footstep_timer = interval;
                }
            } else {
                // not moving — reset timer so first step plays immediately on next move
                p.footstep_timer = 0.0;
            }
        }

        // play collected footstep sounds (outside mutable borrow)
        for (pos, vel, volume) in footstep_sounds {
            self.play_spatial_sound("footstep", pos, vel, volume, config);
        }
        for (pos, vel) in slide_sounds {
            self.play_spatial_sound("slide", pos, vel, SLIDE_SOUND_VOLUME, config);
        }

        let local_pos = Some(self.position);
        for bot in &mut self.practice_bots {
            bot.simulate_yaw(delta, local_pos);
            bot.update(delta);
        }
        let any_slide_skin_active =
            self.other_players.iter().any(|p| p.skin_type == PlayerSkin::Slide) ||
            self.practice_bots.iter().any(|b| b.player.skin_type == PlayerSkin::Slide);
        if let Some(slide_anim) = self.animated_models.get_mut("anim_slide") {
            if any_slide_skin_active && !self.prev_any_slide_anim_active {
                slide_anim.play_segment(SLIDE_CLIP_START_SEC, SLIDE_CLIP_END_SEC, false);
            } else if !any_slide_skin_active {
                // Reset to frame 0 when no one is sliding, so the next slide starts cleanly.
                slide_anim.stop_at_start();
            }
        }
        self.prev_any_slide_anim_active = any_slide_skin_active;

        let animated_models = &self.animated_models;
        for p in &mut self.other_players {
            // here we refresh the hitbox positions every frame based on the current animation pose of the player so hit detection is accurate if they crouch, prone, slide, etc.
            refresh_player_hitboxes(animated_models, p);
        }
        for bot in &mut self.practice_bots {
            refresh_player_hitboxes(animated_models, &mut bot.player);
        }
        self.practice_bots
            .retain(|bot: &crate::types::ai::simulated_player::SimulatedPlayer| !bot.is_dead());
        if self.hit_marker_timer > 0.0 {
            self.hit_marker_timer = (self.hit_marker_timer - delta).max(0.0);
        }

        // Damage flash timer (short red vignette + optional attacker name)
        if self.damage_flash_timer > 0.0 {
            self.damage_flash_timer = (self.damage_flash_timer - delta).max(0.0);
            if self.damage_flash_timer <= 0.0 {
                self.last_damage_attacker = None;
            }
        }

        // Respawn feedback timer
        if self.spawn_feedback_timer > 0.0 {
            self.spawn_feedback_timer = (self.spawn_feedback_timer - delta).max(0.0);
            if self.spawn_feedback_timer <= 0.0 {
                self.spawn_feedback_message = None;
            }
        }

        // Lobby console feedback timer
        if self.lobby_console_message_timer > 0.0 {
            self.lobby_console_message_timer = (self.lobby_console_message_timer - delta).max(0.0);
            if self.lobby_console_message_timer <= 0.0 {
                self.lobby_console_message = None;
            }
        }

        // Match-start countdown (server-initiated, display only)
        // The actual freeze timer is set when the MSG_WORLD_CHANGE arrives from the
        // server (with an authoritative freeze duration) so that all clients are
        // synchronized regardless of connection speed.
        if self.match_start_countdown_timer > 0.0 {
            self.match_start_countdown_timer = (self.match_start_countdown_timer - delta).max(0.0);
            if self.match_start_countdown_timer <= 0.0 {
                self.announcements.push(("GO!".to_string(), 1.5));
            }
        }

        // Match-end countdown (server-initiated)
        if self.match_end_countdown_timer > 0.0 {
            self.match_end_countdown_timer = (self.match_end_countdown_timer - delta).max(0.0);
            if self.match_end_countdown_timer <= 0.0 {
                // clear winner toast once countdown completes
                self.match_end_winner = None;
            }
        }

        // Post-teleport movement freeze timer (client-side display + input gating)
        if self.post_teleport_freeze_timer > 0.0 {
            self.post_teleport_freeze_timer = (self.post_teleport_freeze_timer - delta).max(0.0);
            if self.post_teleport_freeze_timer <= 0.0 {
                self.announcements
                    .push(("Movement unlocked".to_string(), 2.0));
            }
        }

        // Match timer (client-side) — started when client enters Maze
        if self.current_world == WorldId::Maze && self.match_time_left > 0.0 {
            self.match_time_left = (self.match_time_left - delta).max(0.0);
        } else if self.current_world != WorldId::Maze {
            // ensure timer is cleared outside of match
            self.match_time_left = 0.0;
        }

        // Announcements timers
        for ann in &mut self.announcements {
            ann.1 = (ann.1 - delta).max(0.0);
        }
        self.announcements.retain(|ann| ann.1 > 0.0);
        self.chat.tick(delta);

        self.damage_numbers.retain_mut(|dn| {
            dn.lifetime -= delta;
            dn.screen_y -= 60.0 * delta;
            dn.lifetime > 0.0
        });

        self.tracers.retain_mut(|t| {
            if t.progress < 1.0 {
                let dist = (t.end - t.start).length();
                t.progress = (t.progress + (t.speed * delta) / dist).min(1.0);
            }
            t.lifetime -= delta;
            t.lifetime > 0.0
        });

        self.world_muzzle_flashes.retain_mut(|f| {
            f.lifetime -= delta;
            f.lifetime > 0.0
        });

        // bullet hole lifetime decay + remove holes on broken walls
        self.bullet_holes.retain_mut(|h| {
            h.lifetime -= delta;
            if h.lifetime <= 0.0 {
                return false;
            }
            // if this hole is on a breakable wall, check if the wall still exists
            if let Some((cx, cy)) = h.wall_cell {
                if let Some(m) = maze {
                    if !m.is_wall(cx, cy) {
                        return false;
                    }
                }
            }
            true
        });

        self.update_stamina(config, delta);
        if self.movement_state.prone_transition_timer > 0.0 {
            self.movement_state.prone_transition_timer =
                (self.movement_state.prone_transition_timer - delta).max(0.0);
        }
        if self.movement_state.slide_cooldown_timer > 0.0 {
            self.movement_state.slide_cooldown_timer =
                (self.movement_state.slide_cooldown_timer - delta).max(0.0);
        }
        self.update_crouch_height(config, delta);
        let was_grounded = self.movement_state.is_grounded;
        // Only allow jumping when not frozen after teleport
        if self.post_teleport_freeze_timer <= 0.0 {
            self.apply_jump();
        } else {
            // clear any pending jump while frozen
            self.jump_requested = false;
            self.just_jumped = false;
        }
        self.apply_gravity(delta);

        match self.current_world {
            WorldId::Lobby => {
                collision::resolve_vertical(
                    &mut self.position,
                    &mut self.movement_state.velocity,
                    self.movement_state.height,
                    &mut self.movement_state.is_grounded,
                );
                self.apply_movement(delta);
                collision::resolve_horizontal(
                    &mut self.position,
                    &mut self.movement_state.velocity,
                    self.movement_state.height,
                    self.movement_state.is_grounded,
                );
            }
            WorldId::Maze => {
                if self.position.y <= 0.0 {
                    self.position.y = 0.0;
                    self.movement_state.is_grounded = true;
                    self.movement_state.velocity.y = 0.0;
                }
                let prev_position = self.position;

                // while in the short post-teleport freeze, prevent movement on the client
                if self.post_teleport_freeze_timer <= 0.0 {
                    self.apply_movement(delta);
                } else {
                    // keep player stationary horizontally while still allowing gravity to apply
                    self.movement_state.velocity.x = 0.0;
                    self.movement_state.velocity.z = 0.0;
                }

                if let Some(m) = maze {
                    let clamped =
                        m.clamp_move(prev_position, self.position, collision::PLAYER_RADIUS);
                    if (clamped.x - self.position.x).abs() > f32::EPSILON {
                        self.movement_state.velocity.x = 0.0;
                    }
                    if (clamped.z - self.position.z).abs() > f32::EPSILON {
                        self.movement_state.velocity.z = 0.0;
                    }
                    self.position = clamped;
                }
            }
        }
        // mirrors are solid in every world
        collision::resolve_mirrors(
            &mut self.position,
            &mut self.movement_state.velocity,
            self.movement_state.height,
            &self.mirrors,
        );

        if !was_grounded && self.movement_state.is_grounded {
            self.just_landed = true;
        }
        self.prev_was_reloading = self.weapons.active().is_reloading;

        // ── Local player footstep sounds ──
        if !self.is_dead && self.movement_state.is_grounded {
            let is_moving = self.move_dir.length_squared() > 0.01;
            let is_sliding = self.movement_state.is_sliding;

            if is_sliding {
                if self.local_footstep_timer <= 0.0 {
                    self.play_local_sound("slide", SLIDE_SOUND_VOLUME, config);
                    self.local_footstep_timer = 1.0;
                }
            } else if is_moving {
                self.local_footstep_timer -= delta;
                if self.local_footstep_timer <= 0.0 {
                    let (interval, volume) = if self.movement_state.is_sprinting {
                        (FOOTSTEP_INTERVAL_SPRINT, FOOTSTEP_VOLUME_SPRINT)
                    } else if self.movement_state.is_crouching {
                        (FOOTSTEP_INTERVAL_CROUCH, FOOTSTEP_VOLUME_CROUCH)
                    } else {
                        (FOOTSTEP_INTERVAL_WALK, FOOTSTEP_VOLUME_WALK)
                    };
                    self.play_local_sound("footstep", volume, config);
                    self.local_footstep_timer = interval;
                }
            } else {
                self.local_footstep_timer = 0.0;
            }
        } else {
            self.local_footstep_timer = 0.0;
        }

        /*
            this is the function that actually updates the players hitbox
            The way it works is by finding the animated model for the player based on their skin, then finds the position if important joints and builds hitboxes based on these
            it is possible due to the skinning system and the fact that we cache joint positions every frame in the animated model.
        */

        fn refresh_player_hitboxes(
            animated_models: &HashMap<String, AnimatedModel>,
            player: &mut OtherPlayer,
        ) {
            let desired_key = player.skin_type.model_key();
            let anim = desired_key
                .and_then(|k| animated_models.get(k))
                .or_else(|| animated_models.get("anim_rifle_idle"));

            let Some(anim) = anim else {
                player.hitboxes = default_player_hitboxes();
                return;
            };

            /*
                this is the scale factor for the hitbox positions, it is based on the normalize scale of the animation (which is based on the original model size)
                and a tunable render scale factor to adjust how big the hitboxes are overall
                makes it easy to fine tune without needing to adjust the original model or animation.
                We also apply the same scale factor to the y_offset which is used to adjust the hitbox positions up or down based on the players height (crouching, standing, etc)
            */
            let scale = anim.normalize_scale * WORLD_PLAYER_RENDER_SCALE;
            let y_offset = anim.y_offset * WORLD_PLAYER_RENDER_SCALE;
            if let Some(hitboxes) =
                build_standing_hitboxes_from_joints(anim, player.render_yaw, scale, y_offset)
            {
                player.hitboxes = hitboxes;
            } else {
                player.hitboxes = default_player_hitboxes();
            }
        }

        fn build_standing_hitboxes_from_joints(
            anim: &AnimatedModel,
            yaw: f32,
            scale: f32,
            y_offset: f32,
        ) -> Option<Vec<Hitbox>> {
            fn any_joint(anim: &AnimatedModel, patterns: &[&[&str]]) -> Option<Vec3> {
                patterns
                    .iter()
                    .find_map(|tokens| anim.joint_position_with_tokens(tokens))
            }
            /*
                Here we are using the same model scaling as the render so visual hitbox indicators line up with the actual hitboxes, and we also apply the same scaling to
                the y_offset which is used to adjust the hitbox positions up or down based on the players height (crouching, standing, etc)
            */
            let to_offset = |p: Vec3| -> Vec3 {
                let rotated = Mat3::from_rotation_y(-yaw + PLAYER_MODEL_YAW_OFFSET) * (p * scale);
                rotated + vec3(0.0, y_offset, 0.0)
            };

            let head = any_joint(anim, &[&["head"], &["neck", "head"]])?;
            let chest = any_joint(anim, &[&["spine2"], &["chest"], &["spine1"]])?;
            let hips = any_joint(anim, &[&["hips"], &["pelvis"]])?;

            let l_foot = any_joint(anim, &[&["left", "foot"], &["left", "toe"]]).unwrap_or(hips);
            let r_foot = any_joint(anim, &[&["right", "foot"], &["right", "toe"]]).unwrap_or(hips);
            let l_shoulder =
                any_joint(anim, &[&["left", "shoulder"], &["left", "arm"]]).unwrap_or(chest);
            let r_shoulder =
                any_joint(anim, &[&["right", "shoulder"], &["right", "arm"]]).unwrap_or(chest);
            let l_elbow = any_joint(
                anim,
                &[
                    &["left", "forearm"],
                    &["left", "lowerarm"],
                    &["left", "elbow"],
                ],
            )
            .unwrap_or(l_shoulder);
            let r_elbow = any_joint(
                anim,
                &[
                    &["right", "forearm"],
                    &["right", "lowerarm"],
                    &["right", "elbow"],
                ],
            )
            .unwrap_or(r_shoulder);
            let l_hand =
                any_joint(anim, &[&["left", "hand"], &["left", "wrist"]]).unwrap_or(l_elbow);
            let r_hand =
                any_joint(anim, &[&["right", "hand"], &["right", "wrist"]]).unwrap_or(r_elbow);

            let head_o = to_offset(head);
            let chest_o = to_offset(chest);
            let hips_o = to_offset(hips);
            let l_foot_o = to_offset(l_foot);
            let r_foot_o = to_offset(r_foot);
            let l_shoulder_o = to_offset(l_shoulder);
            let r_shoulder_o = to_offset(r_shoulder);
            let l_elbow_o = to_offset(l_elbow);
            let r_elbow_o = to_offset(r_elbow);
            let l_hand_o = to_offset(l_hand);
            let r_hand_o = to_offset(r_hand);

            /*
                Here we are bulding AABB hitboxes from a bone segment (hand, leg, etc) then we also add padding so it works for each state (stand, crouch, etc)
                and we also set a minimum size so hitboxes dont get too small if the model has very thin arms or legs.
            */
            let segment_box =
                |a: Vec3, b: Vec3, pad: Vec3, min_half: Vec3, zone: HitZone| -> Hitbox {
                    let center = (a + b) * 0.5;
                    let axis_half = (b - a).abs() * 0.5;
                    let half = vec3(
                        (axis_half.x + pad.x).max(min_half.x),
                        (axis_half.y + pad.y).max(min_half.y),
                        (axis_half.z + pad.z).max(min_half.z),
                    );
                    Hitbox {
                        center_offset: center,
                        half_extents: half,
                        zone,
                    }
                };

            let shoulder_width = (l_shoulder_o - r_shoulder_o).length().clamp(0.25, 0.65);
            let head_radius = ((head_o - chest_o).length() * 0.40).clamp(0.12, 0.20);
            let torso_min = vec3(shoulder_width * 0.24, 0.12, shoulder_width * 0.20);
            let limb_min = vec3(0.10, 0.10, 0.10);
            let head_up = (head_o - chest_o).normalize_or_zero();
            let head_center = head_o + head_up * (head_radius * 0.25);

            let mut hitboxes = vec![
                Hitbox {
                    center_offset: head_center,
                    half_extents: vec3(head_radius, head_radius, head_radius),
                    zone: HitZone::Head,
                },
                segment_box(
                    chest_o,
                    hips_o,
                    vec3(0.08, 0.08, 0.08),
                    torso_min,
                    HitZone::Torso,
                ),
                segment_box(
                    l_shoulder_o,
                    hips_o,
                    vec3(0.05, 0.05, 0.05),
                    torso_min * 0.8,
                    HitZone::Torso,
                ),
                segment_box(
                    r_shoulder_o,
                    hips_o,
                    vec3(0.05, 0.05, 0.05),
                    torso_min * 0.8,
                    HitZone::Torso,
                ),
                segment_box(
                    hips_o,
                    l_foot_o,
                    vec3(0.05, 0.05, 0.05),
                    limb_min,
                    HitZone::Limbs,
                ),
                segment_box(
                    hips_o,
                    r_foot_o,
                    vec3(0.05, 0.05, 0.05),
                    limb_min,
                    HitZone::Limbs,
                ),
            ];

            let arm_min = vec3(0.08, 0.08, 0.08);
            let arm_pad = vec3(0.04, 0.04, 0.04);
            /*
                For each segment we creeated earlier, we push into the hitbox list by creating an AABB that encapsulates all of it
            */
            hitboxes.push(segment_box(
                l_shoulder_o,
                l_elbow_o,
                arm_pad,
                arm_min,
                HitZone::Limbs,
            ));
            hitboxes.push(segment_box(
                l_elbow_o,
                l_hand_o,
                arm_pad,
                arm_min,
                HitZone::Limbs,
            ));
            hitboxes.push(segment_box(
                r_shoulder_o,
                r_elbow_o,
                arm_pad,
                arm_min,
                HitZone::Limbs,
            ));
            hitboxes.push(segment_box(
                r_elbow_o,
                r_hand_o,
                arm_pad,
                arm_min,
                HitZone::Limbs,
            ));

            Some(hitboxes)
        }
    }
}

// ── Spatial audio helpers ──
impl PlayingState {
    /// Play a sound with distance-based volume falloff and doppler pitch shift.
    /// `source_pos` is the world position of the emitter.
    /// `source_vel` is the emitter's velocity (world units/sec) — used for doppler.
    /// `base_volume` is the volume at distance 0 (before attenuation).
    pub fn play_spatial_sound(
        &self,
        sound_key: &str,
        source_pos: Vec3,
        source_vel: Vec3,
        base_volume: f32,
        config: &GameConfig,
    ) {
        if config.mute_audio {
            return;
        }
        if !self.audio.has(sound_key) {
            return;
        }

        let listener_pos = self.position;
        let diff = source_pos - listener_pos;
        let dist = diff.length();
        if dist > FOOTSTEP_MAX_DIST {
            return; // too far, don't bother
        }

        // distance attenuation: inverse-linear falloff to zero at max dist
        let attenuation = 1.0 - (dist / FOOTSTEP_MAX_DIST);
        let volume = base_volume * attenuation * attenuation; // squared for more natural falloff

        // doppler effect: pitch shifts based on relative velocity toward/away from listener
        let speed_of_sound = 50.0; // tuned for game units (not real-world 343 m/s)
        let doppler_pitch = if dist > 0.1 {
            // unit vector from source toward listener
            let dir_to_listener = diff / dist;
            // positive = source moving toward listener
            let v_source = source_vel.dot(-dir_to_listener);
            // listener velocity: approximate from position (we don't track it, so use 0)
            // doppler: f' = f * (v_sound) / (v_sound + v_source)
            // v_source positive when moving AWAY from listener
            let denom = (speed_of_sound + v_source).max(1.0);
            speed_of_sound / denom
        } else {
            1.0
        };

        // add small random variation to avoid robotic repetition
        let pitch_variation = macroquad::rand::gen_range(0.95f32, 1.05f32);
        let final_pitch = (doppler_pitch * pitch_variation).clamp(0.5, 2.0);

        self.audio.play(sound_key, final_pitch, volume);
    }

    /// Play a sound at the local player's position (no distance attenuation, no doppler).
    fn play_local_sound(&self, sound_key: &str, volume: f32, config: &GameConfig) {
        if config.mute_audio {
            return;
        }
        let pitch = macroquad::rand::gen_range(0.95f32, 1.05f32);
        self.audio.play(sound_key, pitch, volume);
    }
}
