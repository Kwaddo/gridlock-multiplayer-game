use crate::screens::dev_mode::HandsAnimRequest;
use crate::weapons::stats::{WeaponCategory, WeaponId};

use super::super::state::HandsPlaybackMode;
use super::constants::{
    HANDS_RELOAD_END_SEC, HANDS_RELOAD_START_SEC, MINIGUN_RELOAD_END_SEC, MINIGUN_RELOAD_START_SEC,
    MINIGUN_SHOOT_LOOP_END_SEC, MINIGUN_SHOOT_LOOP_START_SEC, PISTOL_RELOAD_END_SEC,
    PISTOL_RELOAD_START_SEC, SAWNOFF_RELOAD_END_SEC, SAWNOFF_RELOAD_START_SEC, SAWNOFF_SHOT_END_SEC,
    SAWNOFF_SHOT_START_SEC, SMG_RELOAD_END_SEC, SMG_RELOAD_START_SEC, SNIPER_RELOAD_END_SEC,
    SNIPER_RELOAD_START_SEC,
};
use super::PlayingState;

impl PlayingState {
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

    /// Drive the first-person hands rigs (generic, sawnoff, minigun) for
    /// the active weapon and advance the animated models in use.
    pub(super) fn update_hands_animations(&mut self, delta: f32) {
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

        for (key, model) in self.animated_models.iter_mut() {
            // player clips are never ticked here anymore, every body has its own clock (PlayerAnim)
            // before this every player clip had one shared clock ticked here, used by everyone on it
            let should_update = if key.starts_with("anim_") {
                false
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
        // only allocate a new key string when the active hands rig changes
        if self.prev_hands_key.as_deref() != active_hands_key {
            self.prev_hands_key = active_hands_key.map(|k| k.to_string());
        }
    }
}
