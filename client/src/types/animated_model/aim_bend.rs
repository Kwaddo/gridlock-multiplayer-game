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
        for &(node, share) in &self.spine_chain {
            let parent_turn = match self.node_parents[node] {
                Some(parent) => pose.global(&self.node_parents, parent).to_scale_rotation_translation().1,
                None => Quat::IDENTITY,
            };
            // the sideways axis as this bone's parent sees it, so the tilt is the same on every bone
            let axis = (parent_turn.inverse() * Vec3::X).normalize_or_zero();
            let tilt = Quat::from_axis_angle(axis, -pitch * share);
            pose.local[node].rotation = (tilt * pose.local[node].rotation).normalize();
            pose.global_valid.iter_mut().for_each(|v| *v = false); // bones above this one moved too
        }
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

    #[test]
    fn missing_bones_still_give_the_full_tilt() {
        let n = names(&["hips", "spine", "head"]);
        let chain = resolve_spine_chain(&n, &[0, 1, 2]);
        assert_eq!(chain, vec![(1, 1.0)]);
        assert!(resolve_spine_chain(&names(&["bone_0"]), &[0]).is_empty());
    }
}
