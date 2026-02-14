/* Runtime weapon state — ammo, cooldowns, recoil accumulation, ADS progress.
 One instance per weapon the player is carrying.
*/
use super::crosshair::CrosshairConfig;
use super::stats::{FireMode, WeaponKind, WeaponStats};

pub struct WeaponState {
    pub kind: WeaponKind,
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
}

impl WeaponState {
    pub fn new(kind: WeaponKind) -> Self {
        let stats = kind.stats();
        Self {
            kind,
            stats_data: stats.clone(),
            ammo: stats.magazine_size,
            reserve: stats.max_reserve,
            fire_cooldown: 0.0,
            reload_timer: 0.0,
            is_reloading: false,
            is_aiming: false,
            ads_progress: 0.0,
            jump_cancelled_ads: false,
            recoil_pitch: 0.0,
            recoil_yaw: 0.0,
            bloom: 0.0,
            computed_spread: 0.0,
            trigger_held: false,
            muzzle_flash_timer: 0.0,
            crosshair: CrosshairConfig::for_weapon(kind),
            fsa_timer: 0.0,
            crosshair_kick: 0.0,
            shot_index: 0,
            pattern_reset_timer: 0.0,
        }
    }

    pub fn stats(&self) -> &WeaponStats {
        &self.stats_data
    }

    // reset stats to factory defaults (from const definitions)
    pub fn reset_stats(&mut self) {
        self.stats_data = self.kind.stats().clone();
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
        let ads_time = self.stats_data.ads_time;
        if self.is_aiming {
            self.ads_progress = (self.ads_progress + delta / ads_time).min(1.0);
        } else {
            self.ads_progress = (self.ads_progress - delta / ads_time).max(0.0);
        }
    }

    // fire the weapon — consume ammo, set cooldown, return true if fired.
    pub fn fire(&mut self) -> bool {
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
            FireMode::FullAuto => {}
        }

        self.ammo -= 1;
        self.fire_cooldown = 1.0 / fire_rate;
        self.muzzle_flash_timer = 0.08; // 80ms flash

        // auto-reload on empty
        if self.ammo == 0 && self.reserve > 0 {
            self.start_reload();
        }

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
}

impl WeaponInventory {
    // start with all 5 weapons for now
    pub fn new() -> Self {
        let weapons = WeaponKind::ALL
            .iter()
            .map(|&kind| WeaponState::new(kind))
            .collect();
        Self {
            weapons,
            active_slot: 0,
            switch_cooldown: 0.0,
        }
    }

    pub fn active(&self) -> &WeaponState {
        &self.weapons[self.active_slot]
    }

    pub fn active_mut(&mut self) -> &mut WeaponState {
        &mut self.weapons[self.active_slot]
    }

    // switch to a specific slot (0-4). returns true if switched.
    pub fn switch_to(&mut self, slot: usize) -> bool {
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

    // scroll to next/prev weapon.
    pub fn scroll(&mut self, direction: i32) {
        let len = self.weapons.len() as i32;
        let new_slot = ((self.active_slot as i32) + direction).rem_euclid(len) as usize;
        self.switch_to(new_slot);
    }

    // tick switch cooldown + all weapon states.
    pub fn update(&mut self, delta: f32) {
        if self.switch_cooldown > 0.0 {
            self.switch_cooldown = (self.switch_cooldown - delta).max(0.0);
        }
        for w in &mut self.weapons {
            w.update(delta);
        }
    }
}
