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
pub fn apply_shot_recoil(weapon: &mut WeaponState, rng_seed: f32) {
    let stats = weapon.stats();

    // vertical kick (always up)
    weapon.recoil_pitch += stats.recoil_vertical;

    // horizontal kick (random left/right, seeded by a float 0..1)
    let h = (rng_seed - 0.5) * 2.0 * stats.recoil_horizontal;
    weapon.recoil_yaw += h;

    // bloom accumulation
    weapon.bloom += stats.spread_per_shot; // bloom is additive on top of base spread
}

// recover recoil and bloom over time — called every frame.
pub fn recover(weapon: &mut WeaponState, delta: f32) {
    let stats = weapon.stats();

    // pitch recoil recovery (settles back to 0)
    if weapon.recoil_pitch > 0.0 {
        let recovery = stats.recoil_recovery * delta;
        weapon.recoil_pitch = (weapon.recoil_pitch - recovery).max(0.0);
    }

    // yaw recoil recovery (settles back to 0)
    if weapon.recoil_yaw.abs() > 0.0001 {
        let sign = weapon.recoil_yaw.signum();
        let recovery = stats.recoil_recovery * delta;
        weapon.recoil_yaw -= sign * recovery.min(weapon.recoil_yaw.abs());
    }

    // bloom decay
    if weapon.bloom > 0.0 {
        let decay = stats.spread_recovery * delta;
        weapon.bloom = (weapon.bloom - decay).max(0.0);
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
