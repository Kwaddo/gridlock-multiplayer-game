/* Runtime weapon state — ammo, cooldowns, recoil accumulation, ADS progress.
 One instance per weapon the player is carrying.
*/
use super::stats::{ FireMode, WeaponKind, WeaponStats };

pub struct WeaponState {
    pub kind: WeaponKind,

    // -- Ammo
    pub ammo: u32,
    pub reserve: u32,

    // -- Timers
    pub fire_cooldown: f32, // seconds until next shot allowed
    pub reload_timer: f32, // seconds left in reload (0 = not reloading)
    pub is_reloading: bool,

    // -- ADS
    pub is_aiming: bool,
    pub ads_progress: f32, // 0.0 = hip, 1.0 = fully aimed

    // -- Recoil accumulation (applied to camera)
    pub recoil_pitch: f32, // accumulated vertical recoil (radians)
    pub recoil_yaw: f32, // accumulated horizontal recoil (radians)

    // -- Bloom (spread accumulation beyond base)
    pub bloom: f32, // extra spread from firing (radians)

    // -- Computed spread (updated each frame, includes bloom + movement penalties)
    pub computed_spread: f32,

    // -- Firing flag (used for semi-auto gate)
    pub trigger_held: bool, // was fire button held last frame?

    // -- Visual feedback
    pub muzzle_flash_timer: f32, // >0 means flash is visible (seconds remaining)

    // -- Viewmodel animation
    pub viewmodel_frame: usize,
    pub viewmodel_timer: f32,
    pub viewmodel_playing: bool,
}

impl WeaponState {
    pub fn new(kind: WeaponKind) -> Self {
        let stats = kind.stats();
        Self {
            kind,
            ammo: stats.magazine_size,
            reserve: stats.max_reserve,
            fire_cooldown: 0.0,
            reload_timer: 0.0,
            is_reloading: false,
            is_aiming: false,
            ads_progress: 0.0,
            recoil_pitch: 0.0,
            recoil_yaw: 0.0,
            bloom: 0.0,
            computed_spread: 0.0,
            trigger_held: false,
            muzzle_flash_timer: 0.0,
            viewmodel_frame: 0,
            viewmodel_timer: 0.0,
            viewmodel_playing: false,
        }
    }

    pub fn stats(&self) -> &'static WeaponStats {
        self.kind.stats()
    }

    // can we fire right now?
    pub fn can_fire(&self) -> bool {
        !self.is_reloading && self.ammo > 0 && self.fire_cooldown <= 0.0
    }

    // start a reload if we're not already reloading and magazine isn't full, AND we have reserve ammo AND attempting to fire
    pub fn start_reload(&mut self) {
        let stats = self.stats();
        if !self.is_reloading && self.ammo < stats.magazine_size && self.reserve > 0 {
            self.is_reloading = true;
            self.reload_timer = stats.reload_time;
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
        let stats = self.stats();
        if self.is_aiming {
            self.ads_progress = (self.ads_progress + delta / stats.ads_time).min(1.0);
        } else {
            self.ads_progress = (self.ads_progress - delta / stats.ads_time).max(0.0);
        }

        // viewmodel animation
        if self.viewmodel_playing {
            let stats = self.stats();
            self.viewmodel_timer += delta;
            if self.viewmodel_timer >= stats.viewmodel_frame_time {
                self.viewmodel_timer = 0.0;
                self.viewmodel_frame += 1;
                if self.viewmodel_frame >= stats.viewmodel_frame_keys.len() {
                    self.viewmodel_playing = false;
                    self.viewmodel_frame = 0;
                }
            }
        }
    }

    // fire the weapon — consume ammo, set cooldown, return true if fired.
    pub fn fire(&mut self) -> bool {
        if !self.can_fire() {
            return false;
        }

        let stats = self.stats();

        // semi-auto / bolt-action gate: must release trigger between shots, as in u have to stop shooting and click again. full auto has no such requirement.
        match stats.fire_mode {
            FireMode::SemiAuto | FireMode::BoltAction => {
                if self.trigger_held {
                    return false;
                }
            }
            FireMode::FullAuto => {}
        }

        self.ammo -= 1;
        self.fire_cooldown = 1.0 / stats.fire_rate;
        self.muzzle_flash_timer = 0.08; // 80ms flash

        // viewmodel animation: restart from first frame
        if !stats.viewmodel_frame_keys.is_empty() {
            self.viewmodel_frame = 0;
            self.viewmodel_timer = 0.0;
            self.viewmodel_playing = true;
        }

        // auto-reload on empty
        if self.ammo == 0 && self.reserve > 0 {
            self.start_reload();
        }

        true
    }

    fn finish_reload(&mut self) {
        let stats = self.stats();
        let needed = stats.magazine_size - self.ammo;
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
        let weapons = WeaponKind::ALL.iter()
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
