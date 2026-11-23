use std::collections::HashMap;

use crate::types::animated_model::AnimatedModel;
use crate::types::blend_space::STANDING;
use crate::types::player::PlayerSkin;

// the clip rules a body's animation follows: which clips loop, hold, fade, layer, and how fast they play

// only the start of the fire clip plays, the kick, every shot restarts it so auto fire reads as recoil
pub(super) const FIRE_KICK: f32 = 0.25;

// the model for a skin, missing clip falls back to idle, same as drawing does
pub(super) fn find(models: &HashMap<String, AnimatedModel>, skin: PlayerSkin) -> Option<&AnimatedModel> {
    skin.model_key()
        .and_then(|k| models.get(k))
        .or_else(|| models.get("anim_rifle_idle"))
}

// move a clip's time forward, one shots stop on their last frame, everything else loops
pub(super) fn advance(skin: PlayerSkin, time: f32, step: f32, end: f32) -> f32 {
    let t = time + step;
    if is_one_shot(skin) {
        t.min(end) // deaths and slides stop on their last frame
    } else if end > 0.0 {
        t.rem_euclid(end) // loop the clip, so walking and running keep going
    } else {
        0.0
    }
}

// soft start and end for the fade, 0..1 in, 0..1 out
pub(super) fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

// clips that play once and hold, instead of looping
pub(super) fn is_one_shot(skin: PlayerSkin) -> bool {
    matches!(
        skin,
        PlayerSkin::StandDeath
            | PlayerSkin::CrouchDeath
            | PlayerSkin::ProneDeath
            | PlayerSkin::Slide
            // the chooser's deaths, before this they looped so bodies kept dying
            | PlayerSkin::DeathFront
            | PlayerSkin::DeathBack
            | PlayerSkin::DeathSide
            | PlayerSkin::DeathFrontHeadshot
            | PlayerSkin::DeathBackHeadshot
            | PlayerSkin::RifleFire
    )
}

// the death clips, every one stays down on its last frame
pub(super) fn is_death(skin: PlayerSkin) -> bool {
    is_one_shot(skin) && !matches!(skin, PlayerSkin::Slide | PlayerSkin::RifleFire)
}

// lying on the floor, no look up/down bend here
pub(super) fn is_prone(skin: PlayerSkin) -> bool {
    use PlayerSkin::*;
    matches!(skin, ProneIdle | ProneForward | ProneBack | ProneTurnLeft | ProneTurnRight)
}

// standing clips the blend space drives, idle too so starting and stopping blend by speed
// before this tapping w played the walk at full stride
pub(super) fn in_space(skin: PlayerSkin) -> bool {
    use PlayerSkin::*;
    matches!(skin, RifleIdle | RifleWalk | WalkBack | WalkLeft | WalkRight | SprintForward | SprintLeft | SprintRight)
}

// one step's length for this mix, the moving clips' lengths by weight, None when its all idle
pub(super) fn stride_length(mix: &[(PlayerSkin, f32)], length_of: impl Fn(PlayerSkin) -> Option<f32>) -> Option<f32> {
    let (mut len, mut moving) = (0.0, 0.0);
    for &(skin, w) in mix.iter().filter(|(s, _)| *s != STANDING.idle) {
        if let Some(l) = length_of(skin) {
            len += l * w;
            moving += w;
        }
    }
    (moving > 0.0 && len > 0.0).then(|| len / moving)
}

// looping clips where the feet move, switching between two of these keeps the stride
pub(super) fn is_moving_loop(skin: PlayerSkin) -> bool {
    use PlayerSkin::*;
    matches!(
        skin,
        RifleWalk | WalkBack | WalkLeft | WalkRight | SlowWalkForward
            | SprintForward | SprintLeft | SprintRight
            | CrouchForward | CrouchBack | CrouchLeft | CrouchRight
            | ProneForward | ProneBack | ProneTurnLeft | ProneTurnRight
    )
}

// how fast to play a moving clip, real speed / the speed the clip is made for (the movement config speeds)
// so coasting to a stop slows the steps down instead of swinging full strides, 1.0 for everything else
pub(super) fn playback_rate(skin: PlayerSkin, speed: f32) -> f32 {
    use PlayerSkin::*;
    let made_for = match skin {
        RifleWalk | WalkBack | WalkLeft | WalkRight => 5.0, // walk_speed
        SlowWalkForward => 1.8, // slow_walk_speed
        SprintForward | SprintLeft | SprintRight => 8.0, // sprint_speed
        CrouchForward | CrouchBack | CrouchLeft | CrouchRight => 2.5, // crouch_speed
        ProneForward | ProneBack | ProneTurnLeft | ProneTurnRight => 1.5, // prone_speed
        _ => return 1.0,
    };
    (speed / made_for).clamp(0.5, 1.5)
}

// where a clip ends, the slide file is longer than the slide, only the first second is used, same as before
pub(super) fn clip_end(skin: PlayerSkin, duration: f32) -> f32 {
    match skin {
        PlayerSkin::Slide => duration.min(1.0),
        PlayerSkin::RifleFire => duration.min(FIRE_KICK),
        _ => duration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_shots_hold_and_loops_wrap() {
        assert_eq!(advance(PlayerSkin::StandDeath, 1.9, 0.5, 2.0), 2.0);
        assert!((advance(PlayerSkin::RifleWalk, 0.9, 0.3, 1.0) - 0.2).abs() < 1e-5);
    }

    #[test]
    fn walk_slows_down_when_barely_moving() {
        assert_eq!(playback_rate(PlayerSkin::RifleWalk, 5.0), 1.0);
        assert_eq!(playback_rate(PlayerSkin::RifleWalk, 0.5), 0.5); // coasting, half speed steps
        assert_eq!(playback_rate(PlayerSkin::RifleIdle, 0.0), 1.0); // idle isnt speed matched
    }

    #[test]
    fn every_death_holds_its_last_frame() {
        use PlayerSkin::*;
        for death in [StandDeath, CrouchDeath, ProneDeath, DeathFront, DeathBack, DeathSide, DeathFrontHeadshot, DeathBackHeadshot] {
            assert_eq!(advance(death, 1.9, 0.5, 2.0), 2.0, "{death:?} looped");
        }
    }

    #[test]
    fn fire_plays_just_the_kick_and_holds() {
        assert_eq!(clip_end(PlayerSkin::RifleFire, 1.2), FIRE_KICK);
        assert_eq!(clip_end(PlayerSkin::RifleFire, 0.1), 0.1); // a short clip plays whole
        assert_eq!(advance(PlayerSkin::RifleFire, 0.2, 0.2, FIRE_KICK), FIRE_KICK); // stops at the end, no loop
    }

    #[test]
    fn stride_length_mixes_the_moving_clips() {
        use PlayerSkin::*;
        let len = |s| Some(match s { RifleWalk => 1.0, SprintForward => 0.6, _ => 2.0 });
        assert_eq!(stride_length(&[(RifleIdle, 1.0)], len), None); // standing still, stride waits
        let mid = stride_length(&[(RifleWalk, 0.5), (SprintForward, 0.5)], len).unwrap();
        assert!((mid - 0.8).abs() < 1e-5);
        let slow = stride_length(&[(RifleIdle, 0.5), (RifleWalk, 0.5)], len).unwrap();
        assert!((slow - 1.0).abs() < 1e-5); // idle doesnt stretch the step
    }

    #[test]
    fn fade_starts_and_ends_soft() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(1.0), 1.0);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-5);
    }
}
