use std::collections::HashMap;

use crate::types::animated_model::{AnimatedModel, Nodes, Pose};
use crate::types::blend_space::{weights, STANDING};

use super::clips::{find, in_space, smoothstep};
use super::{FadeFrom, PlayerAnim, PoseDetail, CPU_SKIN_INTERVAL, FADE_TIME};

/*
    the pose for where advance left the clocks: the clip or the blend space's mix, the fade, the upper layer,
    the look bend, then finished. Reads the clocks, never moves them, so it can run twice in one frame
    (only the hitbox bones first, then in full once the camera turns out to see the body) and give the same answer
*/

impl PlayerAnim {
    /// Work out the pose for where `advance` left the clocks. `Full` is for
    /// drawing; `Hitboxes` only does the bones the hitboxes are built from,
    /// with the same numbers there, and leaves the drawn pose as it was.
    pub fn evaluate(&mut self, models: &HashMap<String, AnimatedModel>, detail: PoseDetail) {
        if !self.posable {
            return; // no model for this skin, so no pose
        }
        let Some(model) = find(models, self.clip) else { return };
        let mut list = std::mem::take(&mut self.hit_nodes);
        // the hitbox nodes, for a hidden body, or for a full pose fading from a still only they are right in
        if detail == PoseDetail::Hitboxes || (self.still_partial && matches!(self.prev, Some(FadeFrom::Still))) {
            model.hit_nodes(&mut list);
        }
        let nodes = match detail {
            PoseDetail::Full => Nodes::All,
            PoseDetail::Hitboxes => Nodes::Only(&list),
        };
        let slot = match detail {
            PoseDetail::Full => &mut self.pose,
            PoseDetail::Hitboxes => &mut self.hit_pose,
        };
        // lent out of self while it's worked out, so the layers can use self too, handed back at the end
        let mut owned = slot.take().unwrap_or_else(|| model.new_pose()); // make a pose if we dont have one yet
        if !model.pose_fits(&owned) {
            owned = model.new_pose(); // the character changed to one with a different skeleton
        }
        self.pose_locals(model, models, &mut owned, nodes, &list);
        match detail {
            PoseDetail::Full => {
                model.finish_pose(&mut owned);
                // if gpu skinning is off, bend the mesh on the cpu for this pose so we can draw it
                // capped per body, before this every body did it every frame and the fallback tanked fps
                if !model.drawn_on_gpu() && (self.cpu_skin_accum >= CPU_SKIN_INTERVAL || model.needs_cpu_skin(&owned)) {
                    self.cpu_skin_accum = 0.0;
                    model.skin_pose_on_cpu(&mut owned);
                }
                self.pose = Some(owned);
                self.hit_fresh = false;
                self.full_at = self.ticks;
                self.view.posed_full();
            }
            PoseDetail::Hitboxes => {
                model.finish_hit_pose(&mut owned);
                self.hit_pose = Some(owned);
                self.hit_fresh = true;
            }
        }
        self.hit_nodes = list;
    }

    // every step before finishing, each on the picked nodes only (`hit` is the hitbox nodes when the fade needs them)
    fn pose_locals(&mut self, model: &AnimatedModel, models: &HashMap<String, AnimatedModel>, pose: &mut Pose, nodes: Nodes, hit: &[usize]) {
        let now_space = in_space(self.clip);
        if now_space {
            self.sample_space(models, pose, nodes);
        } else {
            model.sample_local_nodes(self.time, pose, nodes);
        }

        // still fading out of the old clip or the blend space's still, mix it in
        // (advance dropped the fade once it was done, or when there's nothing to fade from)
        if let Some(from) = self.prev {
            let w = smoothstep(self.fade / FADE_TIME);
            match from {
                FadeFrom::Still => {
                    if let Some(still) = self.still.as_ref().filter(|still| model.pose_fits(still)) {
                        // a still last written hidden is only right in the hitbox nodes, its fingers, toes and head end
                        // are from whenever it was last drawn: those come straight from the new clip, not faded from it
                        // (the hitbox nodes fade with the same sums either way, so hitboxes don't change)
                        let fade = if self.still_partial { Nodes::Only(hit) } else { nodes };
                        pose.blend_from_nodes(still, w, fade);
                    }
                }
                FadeFrom::Clip(prev_skin, prev_time) => {
                    if let Some(prev_model) = find(models, prev_skin) {
                        let prev_pose = self.prev_pose.get_or_insert_with(|| prev_model.new_pose());
                        if !prev_model.pose_fits(prev_pose) {
                            *prev_pose = prev_model.new_pose();
                        }
                        prev_model.sample_local_nodes(prev_time, prev_pose, nodes);
                        pose.blend_from_nodes(prev_pose, w, nodes);
                    }
                }
            }
        }

        // a still of the blend space before the aim and look bends, leaving it fades from this
        // (only written in the blend space, only read out of it, so working a frame out twice is fine)
        if now_space {
            let still = self.still.get_or_insert_with(|| model.new_pose());
            if !model.pose_fits(still) {
                *still = model.new_pose();
            }
            still.copy_from_nodes(pose, nodes);
            self.still_partial = matches!(nodes, Nodes::Only(_));
        }

        // the upper layer at the clock and weight advance gave it
        if self.upper_weight > 0.0 {
            if let Some(upper_model) = self.upper_clip.and_then(|clip| find(models, clip)) {
                let layer = self.upper_pose.get_or_insert_with(|| upper_model.new_pose());
                if !upper_model.pose_fits(layer) {
                    *layer = upper_model.new_pose();
                }
                upper_model.sample_local_nodes(self.upper_time, layer, nodes);
                model.layer_upper_body_nodes(pose, layer, smoothstep(self.upper_weight), nodes);
            }
        }

        // look up/down, not while dying or lying prone (bending a body on the floor looks broken)
        if self.bend {
            model.bend_upper_body(pose, self.pitch);
        }
    }

    // standing moves, mixes the blend space's clips by how they really move, at the stride and idle clock advance gave
    fn sample_space(&mut self, models: &HashMap<String, AnimatedModel>, pose: &mut Pose, nodes: Nodes) {
        let mix = weights(&STANDING, self.local_vel);
        let mut total = 0.0;
        for &(skin, w) in mix.iter() {
            let Some(m) = find(models, skin) else { continue };
            let t = if skin == STANDING.idle { self.time } else { self.stride * m.duration() };
            let first = total == 0.0;
            total += w;
            if first {
                m.sample_local_nodes(t, pose, nodes);
                continue;
            }
            let scratch = self.mix_pose.get_or_insert_with(|| m.new_pose());
            if !m.pose_fits(scratch) {
                *scratch = m.new_pose();
            }
            m.sample_local_nodes(t, scratch, nodes);
            pose.mix_in_nodes(scratch, w / total, nodes);
        }
    }
}
