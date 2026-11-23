use std::collections::HashMap;

use macroquad::prelude::*;

use crate::types::animated_model::{test_rig, AnimatedModel, JointRole};
use crate::types::player::PlayerSkin;

use super::{AnimInput, PlayerAnim, PoseDetail};

// every clip a body can play, each its own made up clip on one shared test skeleton
fn models() -> HashMap<String, AnimatedModel> {
    use PlayerSkin::*;
    let skins = [
        RifleIdle, RifleAimIdle, RifleWalk, WalkBack, WalkLeft, WalkRight, SprintForward, SprintLeft,
        SprintRight, Crouch, CrouchAim, CrouchForward, CrouchBack, CrouchLeft, CrouchRight, Slide,
        ProneIdle, ProneForward, SlowWalkForward, DeathFront, RifleFire,
    ];
    skins
        .iter()
        .enumerate()
        .map(|(i, skin)| (skin.model_key().unwrap().to_string(), test_rig(i as u32 + 1, 0.6 + i as f32 * 0.07)))
        .collect()
}

// a made up few seconds of play: walking about, strafing, sprinting, crouching, aiming, shooting,
// a slide, going prone, dying and respawning, looking up and down
fn input_at(frame: u32) -> AnimInput {
    use PlayerSkin::*;
    let t = frame as f32 / 60.0;
    let phase = (frame / 45) % 10;
    let skin = match phase {
        0 | 1 => RifleWalk,
        2 => RifleIdle,
        3 => CrouchForward,
        4 => Slide,
        5 => ProneForward,
        6 => SprintForward,
        7 => DeathFront,
        _ => RifleIdle,
    };
    // speed and direction change through the phases so the blend space mixes several clips
    let speed = [3.0, 1.5, 0.0, 2.5, 6.0, 1.0, 8.0, 0.0, 0.5, 4.0][phase as usize];
    let heading = t * 0.9;
    AnimInput {
        skin,
        pos: vec3(heading.cos(), 0.0, heading.sin()) * speed * t * 0.2 + vec3(speed * t, 0.0, 0.0),
        pitch: (t * 1.7).sin() * 0.8,
        yaw: (t * 0.6).sin() * 2.0,
        aiming: (frame / 30) % 3 == 1,
        crouching: phase == 3,
        fired: frame % 11 == 0 && phase != 7,
    }
}

fn bits(p: Option<Vec3>) -> Option<[u32; 3]> {
    p.map(|v| v.to_array().map(f32::to_bits))
}

// the hitbox joints of a body posed only for its hitboxes (some frames, or every frame) are the
// same, bit for bit, as a body posed in full every frame, so a hidden body's hitboxes are exact
#[test]
fn hidden_body_hitbox_joints_match_the_full_pose() {
    let models = models();
    let rig = &models["anim_rifle_idle"];
    let mut full = PlayerAnim::new();
    let mut always_hidden = PlayerAnim::new();
    let mut sometimes_hidden = PlayerAnim::new();
    // the made up play has to go through clip fades, the upper layer and blend space mixes, or it proves little
    let (mut fades, mut layers, mut mixes) = (0, 0, 0);
    for frame in 0..900 {
        let dt = if frame % 7 == 0 { 1.0 / 30.0 } else { 1.0 / 60.0 };
        let input = input_at(frame);
        full.tick(&input, dt, &models);
        fades += full.prev.is_some() as u32;
        layers += (full.upper_weight > 0.0) as u32;
        mixes += (crate::types::blend_space::weights(&crate::types::blend_space::STANDING, full.local_vel).len() > 1) as u32;
        always_hidden.advance(&input, dt, &models);
        always_hidden.evaluate(&models, PoseDetail::Hitboxes);
        sometimes_hidden.advance(&input, dt, &models);
        // hidden in runs, and sometimes posed twice in a frame (hitboxes, then full once seen)
        let hidden = (frame / 20) % 3 != 0;
        sometimes_hidden.evaluate(&models, if hidden { PoseDetail::Hitboxes } else { PoseDetail::Full });
        if frame % 13 == 0 {
            sometimes_hidden.evaluate(&models, PoseDetail::Full);
        }
        for role in JointRole::ALL {
            let want = bits(rig.joint_role_position_in(full.hit_pose().unwrap(), role));
            assert!(want.is_some());
            for (name, body) in [("always", &always_hidden), ("sometimes", &sometimes_hidden)] {
                let got = bits(rig.joint_role_position_in(body.hit_pose().unwrap(), role));
                assert_eq!(got, want, "frame {frame}, {name} hidden, {role:?}");
            }
        }
    }
    assert!(fades > 0 && layers > 0 && mixes > 0, "fades {fades} layers {layers} mixes {mixes}");
}

// a body that was hidden and is seen again draws the same pose as one that was drawn all along
// (the fade from the blend space's still is the one exception, its fingers and toes can be older)
#[test]
fn seen_again_draws_the_same_pose() {
    let models = models();
    let (mut full, mut hidden) = (PlayerAnim::new(), PlayerAnim::new());
    for frame in 0..900 {
        let input = input_at(frame);
        full.tick(&input, 1.0 / 60.0, &models);
        hidden.advance(&input, 1.0 / 60.0, &models);
        let seen = (frame / 25) % 2 == 0;
        hidden.evaluate(&models, if seen { PoseDetail::Full } else { PoseDetail::Hitboxes });
        let fading_from_still = matches!(hidden.prev, Some(super::FadeFrom::Still));
        if seen && !fading_from_still {
            let (a, b) = (full.pose().unwrap(), hidden.pose().unwrap());
            assert_eq!(a.skin_mats_for_tests(), b.skin_mats_for_tests(), "frame {frame}");
        }
    }
}

