/* Runtime weapon state — ammo, cooldowns, recoil accumulation, ADS progress.
 One instance per weapon the player is carrying.
*/
use super::crosshair::CrosshairConfig;
use super::loadout::Loadout;
use super::stats::{CrosshairStyle, FireMode, WeaponId, WeaponStats};

pub struct WeaponState {
    pub id: WeaponId,
    pub stats_data: WeaponStats, // owned mutable copy (const defaults used for reset)

    // -- Ammo
    pub ammo: u32,
    pub reserve: u32,

    // -- Timers
    pub fire_cooldown: f32, // seconds until next shot allowed
    pub reload_timer: f32,  // seconds left in reload (0 = not reloading)
    pub is_reloading: bool,

    // -- ADS
    pub is_aiming: bool,
    pub ads_progress: f32,        // 0.0 = hip, 1.0 = fully aimed
    pub ads_time_multiplier: f32, // movement modifiers (e.g. sliding) scale ADS in/out time
    pub jump_cancelled_ads: bool, // true when jump cancelled ADS, prevents re-aim until button released

    // -- Recoil accumulation (applied to camera)
    pub recoil_pitch: f32, // accumulated vertical recoil (radians)
    pub recoil_yaw: f32,   // accumulated horizontal recoil (radians)

    // -- Bloom (spread accumulation beyond base)
    pub bloom: f32, // extra spread from firing (radians)

    // -- Computed spread (updated each frame, includes bloom + movement penalties)
    pub computed_spread: f32,

    // -- Firing flag (used for semi-auto gate)
    pub trigger_held: bool, // was fire button held last frame?

    // -- Visual feedback
    pub muzzle_flash_timer: f32, // >0 means flash is visible (seconds remaining)

    // -- Crosshair (runtime config, will be editable in settings later)
    pub crosshair: CrosshairConfig,
    pub crosshair_kick: f32, // visual gap expansion from shooting, decays back to 0 over time, separate from bloom which affects actual spread
    pub fsa_timer: f32, // time spent not firing when stationary, when >= fsa_time, bloom starts to recover

    // -- more recoil stats
    pub shot_index: u32, // which bullet in current spray (for recoil patterns)
    pub pattern_reset_timer: f32, // time since triggerer released so it can go back to normal after a while

    // -- Burst fire
    pub burst_remaining: u32, // shots left in current burst (0 = not bursting)

    // -- Dev: pair lock (hand + gun move together)
    pub pair_locked: bool,
    pub pair_offset_pos: (f32, f32, f32), // additive offset applied to both weapon and hands
    pub pair_offset_rot: (f32, f32, f32), // additive rotation (pitch, yaw, roll) applied to both

    // -- Dev: ADS viewmodel offsets (additive, lerped by ads_progress, independent per weapon)
    pub ads_pair_locked: bool, // when true, ADS pos/rot sliders move gun+hands together
    pub ads_pos_offset: (f32, f32, f32), // default (0.08, 0.03, 0.0) — matches old hardcoded shift
    pub ads_rot_offset: (f32, f32, f32), // rotation delta at full ADS
    pub ads_hands_pos_offset: (f32, f32, f32), // hands position delta at full ADS
    pub ads_hands_rot_offset: (f32, f32, f32), // hands rotation delta at full ADS
}

impl WeaponState {
    pub fn new(id: WeaponId) -> Self {
        let stats = id.stats();
        let category = id.category();
        Self {
            id,
            stats_data: stats.clone(),
            ammo: stats.magazine_size,
            reserve: stats.max_reserve,
            fire_cooldown: 0.0,
            reload_timer: 0.0,
            is_reloading: false,
            is_aiming: false,
            ads_progress: 0.0,
            ads_time_multiplier: 1.0,
            jump_cancelled_ads: false,
            recoil_pitch: 0.0,
            recoil_yaw: 0.0,
            bloom: 0.0,
            computed_spread: 0.0,
            trigger_held: false,
            muzzle_flash_timer: 0.0,
            crosshair: if stats.crosshair_style == CrosshairStyle::Standard
                && category == super::stats::WeaponCategory::Sniper
            {
                // only snipers need the generic crosshair fallback; other weapon types
                // (including shotguns) should still receive their category-specific
                // crosshair config. previously all Standard-style weapons were forced
                // to use `WeaponCategory::Other` which caused shotguns to show a
                // plain crosshair instead of the intended "four curves" preset.
                CrosshairConfig::for_category(super::stats::WeaponCategory::Other)
            } else {
                CrosshairConfig::for_category(category)
            },
            fsa_timer: 0.0,
            crosshair_kick: 0.0,
            shot_index: 0,
            pattern_reset_timer: 0.0,
            burst_remaining: 0,
            pair_locked: false,
            pair_offset_pos: stats.viewmodel_pair_offset_pos,
            pair_offset_rot: stats.viewmodel_pair_offset_rot,
            ads_pair_locked: false,
            ads_pos_offset: stats.viewmodel_ads_pos_offset,
            ads_rot_offset: stats.viewmodel_ads_rot_offset,
            ads_hands_pos_offset: stats.viewmodel_ads_hands_pos_offset,
            ads_hands_rot_offset: stats.viewmodel_ads_hands_rot_offset,
        }
    }

    pub fn stats(&self) -> &WeaponStats {
        &self.stats_data
    }

    // reset stats to factory defaults (from const definitions)
    pub fn reset_stats(&mut self) {
        self.stats_data = self.id.stats().clone();
    }

    // can we fire right now?
    pub fn can_fire(&self) -> bool {
        !self.is_reloading && self.ammo > 0 && self.fire_cooldown <= 0.0
    }

    // start a reload if we're not already reloading and magazine isn't full, AND we have reserve ammo AND attempting to fire
    pub fn start_reload(&mut self) {
        let mag = self.stats_data.magazine_size;
        let reload_t = self.stats_data.reload_time;
        if !self.is_reloading && self.ammo < mag && self.reserve > 0 {
            self.is_reloading = true;
            self.reload_timer = reload_t;
        }
    }

    // tick timers. Call once per frame.
    pub fn update(&mut self, delta: f32) {
        // fire cooldown
        if self.fire_cooldown > 0.0 {
            self.fire_cooldown = (self.fire_cooldown - delta).max(0.0);
        }

        // reload
        if self.is_reloading {
            self.reload_timer -= delta;
            if self.reload_timer <= 0.0 {
                self.finish_reload();
            }
        }

        // muzzle flash decay
        if self.muzzle_flash_timer > 0.0 {
            self.muzzle_flash_timer = (self.muzzle_flash_timer - delta).max(0.0);
        }

        // ADS interpolation
        let ads_time = (self.stats_data.ads_time * self.ads_time_multiplier).max(0.0001);
        if self.is_aiming {
            self.ads_progress = (self.ads_progress + delta / ads_time).min(1.0);
        } else {
            self.ads_progress = (self.ads_progress - delta / ads_time).max(0.0);
        }
    }

    // fire the weapon — consume ammo, set cooldown, return true if fired.
    pub fn fire(&mut self) -> bool {
        // If empty, pressing fire should auto-reload when we have reserve.
        if self.ammo == 0 {
            self.burst_remaining = 0; // cancel any in-progress burst
            if self.reserve > 0 {
                self.start_reload();
            }
            return false;
        }

        if !self.can_fire() {
            return false;
        }

        let fire_mode = self.stats_data.fire_mode;
        let fire_rate = self.stats_data.fire_rate;

        // semi-auto / bolt-action gate: must release trigger between shots, as in u have to stop shooting and click again. full auto has no such requirement.
        match fire_mode {
            FireMode::SemiAuto | FireMode::BoltAction => {
                if self.trigger_held {
                    return false;
                }
            }
            FireMode::Burst(count) => {
                // Starting a new burst: must release trigger first
                if self.burst_remaining == 0 {
                    if self.trigger_held {
                        return false;
                    }
                    self.burst_remaining = count;
                }
                // Continuing a burst: auto-fire remaining shots
                self.burst_remaining -= 1;
            }
            FireMode::FullAuto => {}
        }

        self.ammo -= 1;
        self.fire_cooldown = 1.0 / fire_rate;

        // Extra pause after the last shot of a burst — prevents instant re-burst
        if matches!(fire_mode, FireMode::Burst(_)) && self.burst_remaining == 0 {
            self.fire_cooldown += 0.3;
        }

        self.muzzle_flash_timer = 0.08; // 80ms flash

        true
    }

    fn finish_reload(&mut self) {
        let needed = self.stats_data.magazine_size - self.ammo;
        let available = needed.min(self.reserve);
        self.ammo += available;
        self.reserve -= available;
        self.is_reloading = false;
        self.reload_timer = 0.0;
    }
}

// the player's weapon inventory — holds all weapon states, tracks active slot.
pub struct WeaponInventory {
    pub weapons: Vec<WeaponState>,
    pub active_slot: usize,
    pub switch_cooldown: f32, // prevents rapid switching but its not that noticeable and it simplifies input handling
    /// When set, only this slot may be selected by the local client (used for Gun Game).
    pub locked_slot: Option<usize>,
    /// Knife is its own thing, separate from loadout slots (like Fortnite pickaxe).
    /// X toggles between knife and the last active gun slot.
    pub knife: WeaponState,
    pub knife_active: bool,
    /// when true the knife may not be put away (used during final Gun Game level)
    pub knife_locked: bool,
}

impl WeaponInventory {
    // build inventory from a loadout (5 slots)
    pub fn from_loadout(loadout: &Loadout) -> Self {
        let weapons = loadout
            .slots
            .iter()
            .map(|&id| WeaponState::new(id))
            .collect();
        Self {
            weapons,
            active_slot: 0,
            switch_cooldown: 0.0,
            locked_slot: None,
            knife: WeaponState::new(WeaponId::Knife),
            knife_active: false,
            knife_locked: false,
        }
    }

    // default inventory for backwards compat
    pub fn new() -> Self {
        Self::from_loadout(&Loadout::default_loadout())
    }

    // find the first weapon slot matching a category, or fall back to slot 0.
    pub fn index_for_category(&self, cat: super::stats::WeaponCategory) -> usize {
        self.weapons
            .iter()
            .position(|w| w.id.category() == cat)
            .unwrap_or(0)
    }

    pub fn active(&self) -> &WeaponState {
        if self.knife_active {
            &self.knife
        } else {
            &self.weapons[self.active_slot]
        }
    }

    pub fn active_mut(&mut self) -> &mut WeaponState {
        if self.knife_active {
            &mut self.knife
        } else {
            &mut self.weapons[self.active_slot]
        }
    }

    /// Replace the weapon in a given slot with a new WeaponId (used by Gun Game progression).
    pub fn replace_weapon(&mut self, slot: usize, new_id: WeaponId) {
        if slot < self.weapons.len() {
            self.weapons[slot] = WeaponState::new(new_id);
        }
    }

    /// Enforce allowed weapon slot for game modes like Gun Game. Pass `None` to clear the lock.
    pub fn set_locked_slot(&mut self, slot: Option<usize>) {
        self.locked_slot = slot;
        // if locking to a different slot, force-active to match the lock immediately
        if let Some(s) = slot {
            if s < self.weapons.len() && s != self.active_slot {
                // bypass cooldown on forced server-driven switch
                self.active_slot = s;
                self.switch_cooldown = 0.0;
            }
        }
    }

    /// Toggle knife on/off. X key behavior: if holding a gun → switch to knife.
    /// If holding knife → switch back to the current gun slot.
    pub fn toggle_knife(&mut self) -> bool {
        if self.switch_cooldown > 0.0 {
            return false;
        }
        if self.knife_active {
            // if locked we cannot put it away
            if self.knife_locked {
                return false;
            }
            // put knife away, go back to gun
            self.knife_active = false;
            self.switch_cooldown = crate::weapons::stats::melee::KNIFE_SWITCH_FROM_TIME;
            true
        } else {
            // put gun away, pull out knife (faster than pulling gun from knife)
            self.knife_active = true;
            self.switch_cooldown = crate::weapons::stats::melee::KNIFE_SWITCH_TO_TIME;
            true
        }
    }

    // switch to a specific slot (0-4). returns true if switched.
    pub fn switch_to(&mut self, slot: usize) -> bool {
        // honor server-enforced lock (e.g. Gun Game)
        if let Some(locked) = self.locked_slot {
            if slot != locked {
                return false;
            }
        }

        // If knife is active, switching to a gun slot puts the knife away
        if self.knife_active {
            if self.knife_locked {
                return false;
            }
            if slot < self.weapons.len() && self.switch_cooldown <= 0.0 {
                self.knife_active = false;
                self.active_slot = slot;
                self.switch_cooldown = 
                crate::weapons::stats::melee::KNIFE_SWITCH_FROM_TIME;
                return true;
            }
            return false;
        }

        if slot < self.weapons.len() && slot != self.active_slot && self.switch_cooldown <= 0.0 {
            // cancel reload on old weapon
            let old = &mut self.weapons[self.active_slot];
            old.is_reloading = false;
            old.reload_timer = 0.0;

            self.active_slot = slot;
            self.switch_cooldown = 0.3; // 300ms switch delay
            true
        } else {
            false
        }
    }

    // scroll to next/prev weapon (scrolling puts knife away if active).
    pub fn scroll(&mut self, direction: i32) {
        // scrolling is not allowed when a locked slot is active
        if self.locked_slot.is_some() {
            return;
        }
        // If knife is out, scrolling normally puts it away and returns to current gun slot
        if self.knife_active {
            if self.knife_locked {
                // knife locked, do nothing
                return;
            }
            if self.switch_cooldown <= 0.0 {
                self.knife_active = false;
                self.switch_cooldown = crate::weapons::stats::melee::KNIFE_SWITCH_FROM_TIME;
            }
            return;
        }
        let len = self.weapons.len() as i32;
        let new_slot = ((self.active_slot as i32) + direction).rem_euclid(len) as usize;
        self.switch_to(new_slot);
    }

    /// Auto-switch weapon when current gun is completely empty (ammo + reserve = 0).
    /// Tries the other gun slot first, then falls back to knife.
    pub fn auto_switch_on_empty(&mut self) {
        if self.knife_active {
            return;
        }
        let current = &self.weapons[self.active_slot];
        if current.ammo > 0 || current.reserve > 0 {
            return;
        }
        // Try other gun slot
        let other_slot = if self.active_slot == 0 { 1 } else { 0 };
        if other_slot < self.weapons.len() {
            let other = &self.weapons[other_slot];
            if other.ammo > 0 || other.reserve > 0 {
                self.active_slot = other_slot;
                self.switch_cooldown = 0.3;
                return;
            }
        }
        // All guns empty — switch to knife
        self.knife_active = true;
        self.switch_cooldown = crate::weapons::stats::melee::KNIFE_SWITCH_TO_TIME;
    }

    /// Add reserve ammo to the active gun (used for kill-resupply in TDM).
    pub fn resupply_active(&mut self, amount: u32) {
        if self.knife_active {
            return;
        }
        let weapon = &mut self.weapons[self.active_slot];
        let max = weapon.stats_data.max_reserve;
        weapon.reserve = (weapon.reserve + amount).min(max);
    }

    // tick switch cooldown + all weapon states.
    pub fn update(&mut self, delta: f32) {
        if self.switch_cooldown > 0.0 {
            self.switch_cooldown = (self.switch_cooldown - delta).max(0.0);
        }
        for w in &mut self.weapons {
            w.update(delta);
        }
        self.knife.update(delta);
        // unlock knife when cooldown is expired if we changed contexts? (not needed)
    }
}
