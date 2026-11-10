use crate::types::animated_model::{AnimatedModel, JointRole, Pose};
use crate::types::player::OtherPlayer;
use crate::weapons::combat::{default_player_hitboxes, HitZone, Hitbox};

use macroquad::prelude::*;
use std::collections::HashMap;

use super::constants::{PLAYER_MODEL_YAW_OFFSET, WORLD_PLAYER_RENDER_SCALE};

/*
    this is the function that actually updates the players hitbox
    The way it works is by finding the animated model for the player based on their skin, then finds the position if important joints and builds hitboxes based on these
    it is possible due to the skinning system and the fact that we cache joint positions every frame in the animated model.
*/

pub(super) fn refresh_player_hitboxes(
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
        build_standing_hitboxes_from_joints(anim, player.anim.pose(), player.render_yaw, scale, y_offset)
    {
        // reuse the vec's allocation rather than building a new one every frame
        player.hitboxes.clear();
        player.hitboxes.extend(hitboxes);
    } else {
        player.hitboxes = default_player_hitboxes();
    }
}

fn build_standing_hitboxes_from_joints(
    anim: &AnimatedModel,
    pose: Option<&Pose>,
    yaw: f32,
    scale: f32,
    y_offset: f32,
) -> Option<[Hitbox; 10]> {
    // joint indices are matched by name once at load (see `JointRole::patterns`)
    // read this body's own pose, before this every body read the shared clip's pose so hitboxes were clones too
    let joint = |role: JointRole| match pose {
        Some(pose) => anim.joint_role_position_in(pose, role),
        None => anim.joint_role_position(role), // no pose yet (first frame), the clip's own like before
    };
    /*
        Here we are using the same model scaling as the render so visual hitbox indicators line up with the actual hitboxes, and we also apply the same scaling to
        the y_offset which is used to adjust the hitbox positions up or down based on the players height (crouching, standing, etc)
    */
    let to_offset = |p: Vec3| -> Vec3 {
        let rotated = Mat3::from_rotation_y(-yaw + PLAYER_MODEL_YAW_OFFSET) * (p * scale);
        rotated + vec3(0.0, y_offset, 0.0)
    };

    let head = joint(JointRole::Head)?;
    let chest = joint(JointRole::Chest)?;
    let hips = joint(JointRole::Hips)?;

    let l_foot = joint(JointRole::LeftFoot).unwrap_or(hips);
    let r_foot = joint(JointRole::RightFoot).unwrap_or(hips);
    let l_shoulder = joint(JointRole::LeftShoulder).unwrap_or(chest);
    let r_shoulder = joint(JointRole::RightShoulder).unwrap_or(chest);
    let l_elbow = joint(JointRole::LeftElbow).unwrap_or(l_shoulder);
    let r_elbow = joint(JointRole::RightElbow).unwrap_or(r_shoulder);
    let l_hand = joint(JointRole::LeftHand).unwrap_or(l_elbow);
    let r_hand = joint(JointRole::RightHand).unwrap_or(r_elbow);

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

    let body = [
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
    let arms = [
        segment_box(
            l_shoulder_o,
            l_elbow_o,
            arm_pad,
            arm_min,
            HitZone::Limbs,
        ),
        segment_box(
            l_elbow_o,
            l_hand_o,
            arm_pad,
            arm_min,
            HitZone::Limbs,
        ),
        segment_box(
            r_shoulder_o,
            r_elbow_o,
            arm_pad,
            arm_min,
            HitZone::Limbs,
        ),
        segment_box(
            r_elbow_o,
            r_hand_o,
            arm_pad,
            arm_min,
            HitZone::Limbs,
        ),
    ];

    let [head_box, torso, l_torso, r_torso, l_leg, r_leg] = body;
    let [l_upper_arm, l_forearm, r_upper_arm, r_forearm] = arms;
    Some([
        head_box, torso, l_torso, r_torso, l_leg, r_leg, l_upper_arm, l_forearm, r_upper_arm,
        r_forearm,
    ])
}
