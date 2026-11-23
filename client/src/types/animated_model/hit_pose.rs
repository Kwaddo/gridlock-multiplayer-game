use super::pose::Nodes;
use super::{AnimatedModel, Pose};

/*
    the cheap pose for bodies nobody can see: only the bones the hitboxes are built from
    hitboxes read the joints with a role (head, chest, hips, feet, shoulders, elbows, hands), the look bend tilts
    the spine chain, and a bone's place needs every parent above it, so that set of nodes is all we work out
    fingers, toes and the head's end bone are left as they were, nothing reads them until the body is drawn again
    every step works node by node (see Nodes), so the picked nodes come out bit for bit the same as the full pose
*/

impl AnimatedModel {
    /// The nodes a body's hitboxes depend on: every joint with a role, the
    /// spine bones the look bend tilts, and all of their parents. Written to
    /// `out` (cleared first) ascending, each once.
    pub fn hit_nodes(&self, out: &mut Vec<usize>) {
        out.clear();
        let count = self.node_rest.len();
        let roles = self.joint_roles.iter().flatten().filter_map(|&j| self.joint_nodes.get(j).copied());
        let spine = self.spine_chain.iter().map(|&(node, _)| node);
        for start in roles.chain(spine) {
            let mut node = Some(start);
            // up to the root, or to a node already in, whose parents are then in too
            while let Some(i) = node {
                if i >= count || out.contains(&i) {
                    break;
                }
                out.push(i);
                node = self.node_parents[i];
            }
        }
        out.sort_unstable();
    }

    /// `finish_pose` for the hitbox joints only: where each joint with a role
    /// is. No skinning matrices, so a pose finished this way can't be drawn.
    pub fn finish_hit_pose(&self, out: &mut Pose) {
        for &j in self.joint_roles.iter().flatten() {
            let Some(&node) = self.joint_nodes.get(j) else { continue };
            let global = out.global(&self.node_parents, node);
            out.joint_positions[j] = global.w_axis.truncate(); // same sum as finish_pose
        }
    }

    /// `layer_upper_body` on only some nodes.
    pub fn layer_upper_body_nodes(&self, pose: &mut Pose, layer: &Pose, w: f32, nodes: Nodes) {
        pose.blend_masked_from_nodes(layer, &self.upper_mask, w, nodes);
    }
}

#[cfg(test)]
impl Pose {
    pub(crate) fn skin_mats_for_tests(&self) -> &[macroquad::prelude::Mat4] {
        &self.skin_mats
    }
}

/// A made up Mixamo-like rig for tests elsewhere: armature, hips, spine
/// chain, head with an end bone, arms with two finger bones each, legs with
/// toes. Every node is a joint. `seed` varies the tracks, so each seed is a
/// different clip on the same skeleton.
#[cfg(test)]
pub(crate) fn test_rig(seed: u32, duration: f32) -> AnimatedModel {
    use super::{KeyframesQuat, KeyframesVec3, LocalTransform, NodeAnimationTrack, SharedGeometry};
    use macroquad::prelude::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[rustfmt::skip]
    let bones: [(&str, Option<usize>); 28] = [
        ("armature", None), ("mixamorig:hips", Some(0)), ("mixamorig:spine", Some(1)),
        ("mixamorig:spine1", Some(2)), ("mixamorig:spine2", Some(3)), ("mixamorig:neck", Some(4)),
        ("mixamorig:head", Some(5)), ("mixamorig:headtop_end", Some(6)),
        ("mixamorig:leftshoulder", Some(4)), ("mixamorig:leftarm", Some(8)), ("mixamorig:leftforearm", Some(9)),
        ("mixamorig:lefthand", Some(10)), ("mixamorig:lefthandindex1", Some(11)), ("mixamorig:lefthandindex2", Some(12)),
        ("mixamorig:rightshoulder", Some(4)), ("mixamorig:rightarm", Some(14)), ("mixamorig:rightforearm", Some(15)),
        ("mixamorig:righthand", Some(16)), ("mixamorig:righthandindex1", Some(17)), ("mixamorig:righthandindex2", Some(18)),
        ("mixamorig:leftupleg", Some(1)), ("mixamorig:leftleg", Some(20)), ("mixamorig:leftfoot", Some(21)),
        ("mixamorig:lefttoebase", Some(22)),
        ("mixamorig:rightupleg", Some(1)), ("mixamorig:rightleg", Some(24)), ("mixamorig:rightfoot", Some(25)),
        ("mixamorig:righttoebase", Some(26)),
    ];
    let names: Vec<String> = bones.iter().map(|b| b.0.to_string()).collect();
    let node_parents: Vec<Option<usize>> = bones.iter().map(|b| b.1).collect();
    // a small deterministic number from the seed, node and key, -1..1
    let wobble = |node: usize, key: usize, axis: u32| {
        let h = (seed.wrapping_mul(2_654_435_761) ^ (node as u32 * 97 + key as u32 * 13 + axis * 7))
            .wrapping_mul(0x9e37_79b9);
        (h >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
    };
    let node_rest: Vec<LocalTransform> = (0..bones.len())
        .map(|i| LocalTransform {
            translation: vec3(wobble(i, 99, 0) * 0.05, 0.1 + wobble(i, 99, 1).abs() * 0.2, wobble(i, 99, 2) * 0.05),
            rotation: Quat::from_euler(EulerRot::YXZ, wobble(i, 98, 0) * 0.3, wobble(i, 98, 1) * 0.3, 0.0),
            scale: if i == 0 { Vec3::splat(0.01) } else { Vec3::ONE },
        })
        .collect();
    let keys = 5;
    let times: Vec<f32> = (0..keys).map(|k| k as f32 * duration / (keys - 1) as f32).collect();
    let node_tracks = (0..bones.len())
        .map(|i| NodeAnimationTrack {
            translation: (i == 1).then(|| KeyframesVec3 {
                times: times.clone(),
                values: (0..keys).map(|k| vec3(wobble(i, k, 0), 1.0 + wobble(i, k, 1) * 0.1, wobble(i, k, 2))).collect(),
            }),
            rotation: (i > 0).then(|| KeyframesQuat {
                times: times.clone(),
                values: (0..keys)
                    .map(|k| Quat::from_euler(EulerRot::YXZ, wobble(i, k, 0), wobble(i, k, 1), wobble(i, k, 2)))
                    .collect(),
            }),
            scale: None,
        })
        .collect();
    let joint_nodes: Vec<usize> = (0..bones.len()).collect();
    let inverse_bind = (0..bones.len()).map(|i| Mat4::from_translation(vec3(0.0, -(i as f32) * 0.1, 0.0))).collect();
    AnimatedModel {
        geometry: Rc::new(SharedGeometry::new(Vec::new())),
        gpu_drawn: Cell::new(true), // as if drawn on the gpu, so no cpu skinning in tests
        pose: Pose::new(&node_rest, bones.len()),
        joint_roles: super::joints::resolve_joint_roles(&names),
        spine_chain: super::aim_bend::resolve_spine_chain(&names, &joint_nodes),
        upper_mask: super::upper_body::upper_body_mask(&names, &node_parents),
        node_rest,
        node_parents,
        node_tracks,
        joint_nodes,
        joint_names: names,
        inverse_bind,
        duration,
        time: 0.0,
        skin_accum: 0.0,
        posed_time: None,
        cpu_skinned_time: None,
        playback_loop: true,
        playback_paused: false,
        playback_finished: false,
        playback_start: 0.0,
        playback_end: duration,
        normalize_scale: 1.0,
        y_offset: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::animated_model::JointRole;

    #[test]
    fn hit_nodes_are_the_role_joints_spine_and_their_parents() {
        let rig = test_rig(1, 1.0);
        let mut nodes = Vec::new();
        rig.hit_nodes(&mut nodes);
        // no head end bone (7), no fingers (12, 13, 18, 19), no toes (23, 27)
        let want: Vec<usize> =
            vec![0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 14, 15, 16, 17, 20, 21, 22, 24, 25, 26];
        assert_eq!(nodes, want);
        for role in JointRole::ALL {
            assert!(rig.joint_roles[role as usize].is_some(), "{role:?} not found in the test rig");
        }
    }

    // sampling, bending and finishing only the hit nodes gives the same joints, bit for bit
    #[test]
    fn hit_pose_joints_match_the_full_pose() {
        let rig = test_rig(7, 1.3);
        let mut nodes = Vec::new();
        rig.hit_nodes(&mut nodes);
        let (mut full, mut hit) = (rig.new_pose(), rig.new_pose());
        for step in 0..40 {
            let t = step as f32 * 0.037;
            rig.sample_local(t, &mut full);
            rig.sample_local_nodes(t, &mut hit, Nodes::Only(&nodes));
            let pitch = (step as f32 * 0.21).sin();
            rig.bend_upper_body(&mut full, pitch);
            rig.bend_upper_body(&mut hit, pitch);
            rig.finish_pose(&mut full);
            rig.finish_hit_pose(&mut hit);
            for role in JointRole::ALL {
                let (a, b) = (rig.joint_role_position_in(&full, role), rig.joint_role_position_in(&hit, role));
                assert_eq!(a.map(|v| v.to_array().map(f32::to_bits)), b.map(|v| v.to_array().map(f32::to_bits)), "{role:?}");
            }
        }
    }
}
