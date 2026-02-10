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

    // vertical kick (always up)
    weapon.recoil_pitch += weapon.stats_data.recoil_vertical;

    // horizontal kick (random left/right, seeded by a float 0..1)
    let h = (rng_seed - 0.5) * 2.0 * weapon.stats_data.recoil_horizontal;
    weapon.recoil_yaw += h;

    // bloom accumulation
    weapon.bloom += weapon.stats_data.spread_per_shot;

    // crosshair kick
    weapon.crosshair_kick += weapon.crosshair.kick_per_shot;
    if weapon.crosshair.kick_max > 0.0 {
        weapon.crosshair_kick = weapon.crosshair_kick.min(weapon.crosshair.kick_max);
    }

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
    if weapon.bloom > 0.0 {
        let decay = spread_recovery * delta;
        weapon.bloom = (weapon.bloom - decay).max(0.0);
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
        weapon.crosshair_kick *= (1.0 - weapon.crosshair.kick_decay_rate * delta).max(0.0); // exponential decay
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
