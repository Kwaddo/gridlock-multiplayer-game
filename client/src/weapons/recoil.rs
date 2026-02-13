/* 
    Recoil and bloom (spread) math.
    Handles accumulation from firing and recovery over time.
    Designed so future features (mantling, sliding, etc.) can hook in
    as additional multipliers without changing this core logic.
*/
use super::state::WeaponState; //use super means "use from the parent module" (weapons::state)

/*  
    Context flags that affect spread calculation.
    This is the extension point — will add new fields here when we add
     mantling, sliding, wall-running, etc. if we ever do
*/
pub struct SpreadContext {
    pub is_moving: bool,
    pub is_sprinting: bool,
    pub is_crouching: bool,
    pub is_airborne: bool,
    // future: pub is_mantling: bool,
    // future: pub is_sliding: bool,
}

// apply recoil kick from a single shot — called right after firing.
pub fn apply_shot_recoil(weapon: &mut WeaponState, rng_seed: f32) -> bool {
    let fsa_was_active = weapon.stats_data.fsa_time.map_or(false, |t| weapon.fsa_timer >= t);

    // reset FSA timer on shot (must happen after the check above)
    weapon.fsa_timer = 0.0;

    // FSA first shot: no recoil, no bloom, no crosshair kick — perfectly accurate
    if fsa_was_active {
        return true;
    }

    let pattern = weapon.stats_data.recoil_pattern;
    let idx = weapon.shot_index as usize;

    // check which pattern entry you are in based on idx
    let (ptrn_pitch, ptrn_yaw) = if pattern.is_empty() {
        // no pattern for shotgun and sniper and other future weapons, just use pure randomness each shot
        (weapon.stats_data.recoil_vertical, 0.0)
    } else {
        pattern[idx.min(pattern.len() - 1)] // if idx exceeds pattern length, just use the last entry (many games do this to avoid punishing long sprays with super crazy recoil)
    };

    // calculate randomness, increasing mroe as you shoot more in a spray, added on top of existing pattern values
    let rand = (weapon.stats_data.pattern_randomness
    + weapon.shot_index as f32 * weapon.stats_data.randomness_growth)
    .clamp(0.0, 1.0); // randomness should be between 0 and 1

    // vertical kick

    let random_pitch = weapon.stats_data.recoil_vertical;
    let pitch_kick = ptrn_pitch * (1.0 - rand) + random_pitch * rand; // 1.0 - rand since its clamped to 0..1, so more randomness means pattern has less influence
    weapon.recoil_pitch += pitch_kick;

    // horizontal kick (random left/right, seeded by a float 0..1)
    let ptrn_h = ptrn_yaw * weapon.stats_data.recoil_horizontal;
    let rand_h = (rng_seed - 0.5) * 2.0 * weapon.stats_data.recoil_horizontal;
    let h_kick = ptrn_h * (1.0 - rand) + rand_h * rand; // h kick is calculated by getting total randomness (1.0 - rand) and applying it to the multiplication of horizontal and effective randomness (we multiply to get a value that is scaled by how much randomness there is, so if randomness is 0 then it has no effect, and if randomness is 1 then it has full effect, and in between it scales linearly)
    weapon.recoil_yaw += h_kick;


    // bloom accumulation
    weapon.bloom += weapon.stats_data.spread_per_shot;

    // crosshair kick
    let kick_per_shot = weapon.stats().crosshair_kick_per_shot;
    let kick_max = weapon.stats().crosshair_kick_max;
    weapon.crosshair_kick += kick_per_shot;
    if kick_max > 0.0 {
        weapon.crosshair_kick = weapon.crosshair_kick.min(kick_max);
    }

    // go to next pattern index for next shot
    weapon.shot_index += 1;
    weapon.pattern_reset_timer = 0.0; // reset pattern reset timer since we just fired

    false
}

// recover recoil and bloom over time — called every frame.
pub fn recover(weapon: &mut WeaponState, delta: f32, is_moving: bool) {
    let recoil_recovery = weapon.stats_data.recoil_recovery;
    let spread_recovery = weapon.stats_data.spread_recovery;
    let fsa_time = weapon.stats_data.fsa_time;

    // pitch recoil recovery (settles back to 0)
    if weapon.recoil_pitch > 0.0 {
        let recovery = recoil_recovery * delta;
        weapon.recoil_pitch = (weapon.recoil_pitch - recovery).max(0.0);
    }

    // yaw recoil recovery (settles back to 0)
    if weapon.recoil_yaw.abs() > 0.0001 {
        let sign = weapon.recoil_yaw.signum();
        let recovery = recoil_recovery * delta;
        weapon.recoil_yaw -= sign * recovery.min(weapon.recoil_yaw.abs());
    }

    // bloom decay
    // initially a fast decay
    if weapon.bloom > 0.0 {
        let decay = spread_recovery * 3.0; // initial fast decay
        weapon.bloom *= (1.0 - decay * delta).max(0.0); // exponential decay
        // snap to zero when tiny
        if weapon.bloom < 0.001 {
            weapon.bloom = 0.0;
        }
    }

    // shot idx decay, when we are not firing it slowly decays to zero
    // in other words, short bursts restart from start of pattern

    if !weapon.trigger_held && weapon.shot_index > 0 {
        let reset_time = weapon.stats_data.pattern_reset_time;

        weapon.pattern_reset_timer += delta;
        if reset_time > 0.0 {
            // exponential decay, fast then slow
            let progress = (weapon.pattern_reset_timer / reset_time).clamp(0.0, 1.0);
            let decayed = ((weapon.shot_index as f32) * (1.0 - progress* progress)) as u32;
            weapon.shot_index = decayed;
        }
        if weapon.pattern_reset_timer >= reset_time {
            weapon.shot_index = 0;
            // we dont reset the pattern timer here because it will just stay at the reset time until the next shot, at which point it will be reset to 0, so this just ensures that if you wait long enough between bursts, you always start at the first pattern entry with no bloom or extra randomness from shot index
        }
    }


    // First shot accuracy logic
    // when timer reaches fsa_time, force bloom to 0
    if let Some(fsa_time) = fsa_time {
        if !weapon.trigger_held && !is_moving {
            weapon.fsa_timer += delta;
            if weapon.fsa_timer >= fsa_time {
                weapon.bloom = 0.0;
            }
        } else {
            weapon.fsa_timer = 0.0; // reset FSA when moving or firing
        }
    } else {
        weapon.fsa_timer = 0.0; // if weapon doesn't have FSA, just keep timer at 0
    }

    // crosshair kick decay
    if weapon.crosshair_kick > 0.0 { 
        weapon.crosshair_kick *= (1.0 - weapon.stats().crosshair_kick_decay * delta).max(0.0); // exponential decay
    } else {
        weapon.crosshair_kick = 0.0;
    }
}

// calculate spread angle (radians) for the current frame.
// this is what the crosshair size and bullet deviation are based on.
pub fn current_spread(weapon: &WeaponState, ctx: &SpreadContext) -> f32 {
    let stats = weapon.stats();

    // base spread (interpolate between hip and ADS based on ads_progress)
    let base = lerp(stats.hip_spread, stats.ads_spread, weapon.ads_progress);

    // bloom on top
    let with_bloom = base + weapon.bloom;

    // situational multipliers
    let mut mult = 1.0;

    if ctx.is_sprinting {
        mult *= stats.sprint_spread_mult;
    } else if ctx.is_moving {
        mult *= stats.move_spread_mult;
    }

    if ctx.is_crouching {
        mult *= stats.crouch_spread_mult;
    }

    if ctx.is_airborne {
        mult *= 1.5; // universal airborne penalty
    }

    // future: if ctx.is_mantling { mult *= 0.7; } // example: mantling tightens spread

    with_bloom * mult
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
