use macroquad::prelude::*;

use super::{AnimatedModel, Pose};

/*
    look up/down on the body, like ue's aim offset but done in code instead of with poses
    the spine and neck tilt toward where you look, spread over 4 bones so it curves instead of folding at one joint
    runs after the clip (and any blend) is sampled and before the pose is finished, so hitboxes and the gun hand follow it
    before this the body always looked straight ahead no matter where you aimed
*/

// which bones bend and how much of the tilt each one takes, lower spine to neck
const CHAIN: [(&str, f32); 4] = [("spine", 0.2), ("spine1", 0.25), ("spine2", 0.3), ("neck", 0.25)];

// find the chain's bones by name once at load, (node index, share of the tilt)
// matches the end of the name, so mixamorig:spine doesnt also catch spine1 and spine2
pub(super) fn resolve_spine_chain(joint_names: &[String], joint_nodes: &[usize]) -> Vec<(usize, f32)> {
    let found: Vec<(usize, f32)> = CHAIN
        .iter()
        .filter_map(|(bone, share)| {
            joint_names
                .iter()
                .position(|n| n.rsplit(':').next() == Some(*bone))
                .map(|j| (joint_nodes[j], *share))
        })
        .collect();
    // if some bones are missing the rest share the whole tilt, so the bend is still full
    let total: f32 = found.iter().map(|(_, s)| s).sum();
    found.into_iter().map(|(n, s)| (n, s / total.max(0.0001))).collect()
}

impl AnimatedModel {
    // tilt this pose's upper body by `pitch` (radians, + = look up), call between sample_local and finish_pose
    pub fn bend_upper_body(&self, pose: &mut Pose, pitch: f32) {
        if pitch == 0.0 || self.spine_chain.is_empty() {
            return;
        }
        // the character faces +z in model space, so looking up = turning about the model's x (sideways) axis
        // every axis is read before any bone tilts: a tilt turns about the model's x, which leaves x where it is,
        // so a bone above sees the same axis before or after the ones below it tilt
        // before this each bone asked again after the one below it tilted, working the chain out from the root every time
        let mut axes = [Vec3::X; CHAIN.len()];
        for (axis, &(node, _)) in axes.iter_mut().zip(&self.spine_chain) {
            let parent_turn = match self.node_parents[node] {
                Some(parent) => pose.global(&self.node_parents, parent).to_scale_rotation_translation().1,
                None => Quat::IDENTITY,
            };
            // the sideways axis as this bone's parent sees it, so the tilt is the same on every bone
            *axis = (parent_turn.inverse() * Vec3::X).normalize_or_zero();
        }
        for (axis, &(node, share)) in axes.into_iter().zip(&self.spine_chain) {
            let tilt = Quat::from_axis_angle(axis, -pitch * share);
            pose.local[node].rotation = (tilt * pose.local[node].rotation).normalize();
        }
        pose.global_valid.iter_mut().for_each(|v| *v = false); // the chain and every bone above it moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn finds_the_mixamo_chain_and_shares_add_up() {
        let n = names(&["mixamorig:hips", "mixamorig:spine", "mixamorig:spine1", "mixamorig:spine2", "mixamorig:neck", "mixamorig:head"]);
        let nodes: Vec<usize> = (10..16).collect();
        let chain = resolve_spine_chain(&n, &nodes);
        assert_eq!(chain.iter().map(|c| c.0).collect::<Vec<_>>(), vec![11, 12, 13, 14]);
        assert!((chain.iter().map(|c| c.1).sum::<f32>() - 1.0).abs() < 1e-5);
    }

    // the old way, every bone asking its parent again after the one below it tilted, to check the one pass gives the same bend
    fn bend_one_bone_at_a_time(model: &AnimatedModel, pose: &mut Pose, pitch: f32) {
        for &(node, share) in &model.spine_chain {
            let parent_turn = match model.node_parents[node] {
                Some(parent) => pose.global(&model.node_parents, parent).to_scale_rotation_translation().1,
                None => Quat::IDENTITY,
            };
            let axis = (parent_turn.inverse() * Vec3::X).normalize_or_zero();
            let tilt = Quat::from_axis_angle(axis, -pitch * share);
            pose.local[node].rotation = (tilt * pose.local[node].rotation).normalize();
            pose.global_valid.iter_mut().for_each(|v| *v = false);
        }
    }

    // a twisted spine (hips, 4 chain bones, head) so every parent has its own turn
    fn twisted_spine() -> AnimatedModel {
        use super::super::gpu::GpuGeometry;
        use super::super::{LocalTransform, SharedGeometry};
        use std::cell::{Cell, RefCell};
        use std::rc::Rc;
        let turns = [
            Quat::from_euler(EulerRot::YXZ, 0.6, 0.1, 0.2),
            Quat::from_euler(EulerRot::YXZ, -0.3, 0.2, 0.1),
            Quat::from_euler(EulerRot::YXZ, 0.2, -0.1, 0.3),
            Quat::from_euler(EulerRot::YXZ, 0.4, 0.3, -0.2),
            Quat::from_euler(EulerRot::YXZ, -0.5, 0.1, 0.1),
            Quat::from_euler(EulerRot::YXZ, 0.1, 0.2, 0.3),
        ];
        let node_rest: Vec<LocalTransform> = turns
            .iter()
            .map(|&rotation| LocalTransform { translation: vec3(0.0, 0.3, 0.05), rotation, scale: Vec3::splat(1.2) })
            .collect();
        AnimatedModel {
            geometry: Rc::new(SharedGeometry { parts: Vec::new(), gpu: RefCell::new(GpuGeometry::Untried) }),
            gpu_drawn: Cell::new(false),
            pose: Pose::new(&node_rest, 6),
            node_rest,
            node_parents: vec![None, Some(0), Some(1), Some(2), Some(3), Some(4)],
            node_tracks: vec![Default::default(); 6],
            joint_nodes: (0..6).collect(),
            joint_names: names(&["hips", "spine", "spine1", "spine2", "neck", "head"]),
            joint_roles: [None; super::super::joints::JOINT_ROLE_COUNT],
            spine_chain: vec![(1, 0.2), (2, 0.25), (3, 0.3), (4, 0.25)],
            upper_mask: Vec::new(),
            inverse_bind: vec![Mat4::IDENTITY; 6],
            duration: 1.0,
            time: 0.0,
            skin_accum: 0.0,
            posed_time: None,
            cpu_skinned_time: None,
            playback_loop: true,
            playback_paused: false,
            playback_finished: false,
            playback_start: 0.0,
            playback_end: 1.0,
            normalize_scale: 1.0,
            y_offset: 0.0,
        }
    }

    #[test]
    fn one_pass_bend_matches_bending_one_bone_at_a_time() {
        let model = twisted_spine();
        for pitch in [-1.2, -0.4, 0.3, 1.4] {
            let (mut fast, mut slow) = (model.new_pose(), model.new_pose());
            model.sample_local(0.0, &mut fast);
            model.sample_local(0.0, &mut slow);
            model.bend_upper_body(&mut fast, pitch);
            bend_one_bone_at_a_time(&model, &mut slow, pitch);
            model.finish_pose(&mut fast);
            model.finish_pose(&mut slow);
            for (a, b) in fast.joint_positions.iter().zip(&slow.joint_positions) {
                assert!((*a - *b).length() < 1e-4, "{pitch}: {a:?} vs {b:?}");
            }
            for (a, b) in fast.skin_mats.iter().zip(&slow.skin_mats) {
                assert!(a.abs_diff_eq(*b, 1e-4), "{pitch}: {a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn missing_bones_still_give_the_full_tilt() {
        let n = names(&["hips", "spine", "head"]);
        let chain = resolve_spine_chain(&n, &[0, 1, 2]);
        assert_eq!(chain, vec![(1, 1.0)]);
        assert!(resolve_spine_chain(&names(&["bone_0"]), &[0]).is_empty());
    }
}
