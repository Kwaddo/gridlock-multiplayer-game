use macroquad::models::Mesh;
use macroquad::prelude::*;

use super::LocalTransform;
use super::playback::{sample_quat, sample_vec3};
use super::AnimatedModel;

/*
    what an animation looks like at a time, many bodies can hold their own pose from shared clip
    essentially a snapshot of a skeleton at one moment, animation is shared by everyone but each person gets their own pose
    to avoid the dumb issues we had from before
*/

#[derive(Default)]
pub struct Pose { // how eac join is bent/moved compared to its parent
    pub(super) local: Vec<LocalTransform>,
    global: Vec<Mat4>,
    pub(super) global_valid: Vec<bool>,
    pub(super) skin_mats: Vec<Mat4>,
    pub(super) joint_positions: Vec<Vec3>,
    // the mesh bent on the cpu for this pose, only made when the gpu cant do it
    // before this the bent mesh lived on the clip, so everyone on that clip shared one
    pub(super) cpu_meshes: Option<Vec<Mesh>>,
}

impl Pose {
    pub(super) fn new(rest: &[LocalTransform], joint_count: usize) -> Self {
        Self { // here we assume that the rest pose is valid, so we can use it to initialize the global transforms
            local: rest.to_vec(),
            global: vec![Mat4::IDENTITY; rest.len()],
            global_valid: vec![false; rest.len()],
            skin_mats: vec![Mat4::IDENTITY; joint_count],
            joint_positions: vec![Vec3::ZERO; joint_count],
            cpu_meshes: None,
        }
    }

    // mix another pose's bent joints into this one, w=0 all `from`, w=1 all this one
    // like ue's blend node, it mixes the local bends then finish_pose works out the rest
    // before this a clip change was an instant cut, nothing could mix two poses
    pub fn blend_from(&mut self, from: &Pose, w: f32) {
        for (mine, theirs) in self.local.iter_mut().zip(&from.local) {
            mine.translation = theirs.translation.lerp(mine.translation, w);
            mine.rotation = theirs.rotation.slerp(mine.rotation, w); // slerp takes the short way round
            mine.scale = theirs.scale.lerp(mine.scale, w);
        }
        self.global_valid.iter_mut().for_each(|v| *v = false); // the mix changed the bends, old answers are stale
    }

    // read only where is this node, only right after sample_pose, which works them all out
    // drawing only gets to read, so it cant use global() which saves answers as it goes
    pub(super) fn global_of(&self, node: usize) -> Mat4 {
        self.global[node]
    }

    // where is the joint actually? where is my parent bone?
    // ask the parent which asks its parent all the way to the top
    // save answer so nobody has to ask again, unless the pose changes
    pub(super) fn global(&mut self, parents: &[Option<usize>], node: usize) -> Mat4 {
        if self.global_valid[node] {
            return self.global[node];
        }
        let local = self.local[node].to_mat4();
        let global = match parents[node] {
            Some(parent) => self.global(parents, parent) * local,
            None => local,
        };
        self.global[node] = global;
        self.global_valid[node] = true;
        global
    }
}

/*
    clip keyframes are shared and never change, this is just to answer what 
    you look like at this current time for whatever pose you are in
*/

impl AnimatedModel {
    // blank pose the right size for the skeleton
    pub fn new_pose(&self) -> Pose {
        Pose::new(&self.node_rest, self.joint_nodes.len())
    }

    // the clip's own pose, for bodies that dont have their own yet
    pub fn own_pose(&self) -> &Pose {
        &self.pose
    }

    // sample a clip at a time and finish the pose, the simple path with no blending
    pub fn sample_pose(&self, time: f32, out: &mut Pose) {
        self.sample_local(time, out);
        self.finish_pose(out);
    }

    // only step 1 of sample_pose, how each joint is bent at this time, so two clips can be mixed before finishing
    // before this sampling and finishing were one step, so there was no gap to blend in
    pub fn sample_local(&self, time: f32, out: &mut Pose) {

        // first step is to find out how each joint is bent off its parent from the keyframes

        for i in 0..self.node_rest.len() { // loop over all nodes in the skeleton
            let mut p = self.node_rest[i];

            let track = &self.node_tracks[i];
            
            // if there is a translation track, sample it at the given time and update the pose's local transform
            if let Some(t) = &track.translation {
                p.translation = sample_vec3(t, time);
            }        
             if let Some(t) = &track.rotation {
                p.rotation = sample_quat(t, time);
            }
            if let Some(t) = &track.scale {
                p.scale = sample_vec3(t, time);
            }

            out.local[i] = p;
            out.global_valid[i] = false; // new frame, old answers are stale
        }
    }

    // step 2, turn the bent joints into where they actually are + the gpu matrices
    pub fn finish_pose(&self, out: &mut Pose) {
        // work out every node now not just joints, so drawing can just read them later
        // before this the cpu fallback worked out the non joint ones while drawing
        for node in 0..self.node_rest.len() {
            out.global(&self.node_parents, node);
        }

        // second step is to find out where each joint is in worldspace and the GPU matrix for it (for skinning)
        for (j, &node) in self.joint_nodes.iter().enumerate() {
            let global = out.global(&self.node_parents, node);
            out.joint_positions[j] = global.transform_point3(Vec3::ZERO); // for hitboxes
            out.skin_mats[j] = global * self.inverse_bind[j]; // for skinning
        }

    }
}
/*
    tests: a tiny fake skeleton (hips + an arm sticking up), no asset files needed.
    hips slide from x=0 to x=2 over 1 second, the arm rotates 90° over the same second
*/
#[cfg(test)]
mod tests {
    use super::super::gpu::GpuGeometry;
    use super::super::{KeyframesQuat, KeyframesVec3, NodeAnimationTrack, SharedGeometry};
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    fn rest(translation: Vec3) -> LocalTransform {
        LocalTransform { translation, rotation: Quat::IDENTITY, scale: Vec3::ONE }
    }

    // node 0 = hips (root), node 1 = arm, 1 up from the hips
    fn test_clip() -> AnimatedModel {
        let hips_track = NodeAnimationTrack {
            translation: Some(KeyframesVec3 {
                times: vec![0.0, 1.0],
                values: vec![Vec3::ZERO, vec3(2.0, 0.0, 0.0)],
            }),
            ..Default::default()
        };
        let arm_track = NodeAnimationTrack {
            rotation: Some(KeyframesQuat {
                times: vec![0.0, 1.0],
                values: vec![Quat::IDENTITY, Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)],
            }),
            ..Default::default()
        };
        let node_rest = vec![rest(Vec3::ZERO), rest(vec3(0.0, 1.0, 0.0))];
        AnimatedModel {
            geometry: Rc::new(SharedGeometry { parts: Vec::new(), gpu: RefCell::new(GpuGeometry::Untried) }),
            gpu_drawn: Cell::new(false),
            pose: Pose::new(&node_rest, 2),
            node_rest,
            node_parents: vec![None, Some(0)],
            node_tracks: vec![hips_track, arm_track],
            joint_nodes: vec![0, 1],
            joint_names: vec!["hips".into(), "arm".into()],
            joint_roles: [None; super::super::joints::JOINT_ROLE_COUNT],
            spine_chain: Vec::new(),
            inverse_bind: vec![Mat4::IDENTITY; 2],
            duration: 1.0,
            time: 0.0,
            skin_accum: 0.0,
            playback_loop: true,
            playback_paused: false,
            playback_finished: false,
            playback_start: 0.0,
            playback_end: 1.0,
            normalize_scale: 1.0,
            y_offset: 0.0,
        }
    }

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    // the old way (the clip's own clock) and the new way (any time) give the exact same answer
    #[test]
    fn own_clock_and_sample_pose_agree() {
        let mut clip = test_clip();
        clip.time = 0.3;
        clip.pose();
        let mut mine = clip.new_pose();
        clip.sample_pose(0.3, &mut mine);
        assert_eq!(clip.pose.joint_positions, mine.joint_positions);
        assert_eq!(clip.pose.skin_mats, mine.skin_mats);
    }

    // asking for 0.5 gives 0.5, no matter where the clip's own clock is
    #[test]
    fn sampling_ignores_the_clips_own_clock() {
        let mut clip = test_clip();
        clip.time = 0.8;
        clip.pose();
        let mut mine = clip.new_pose();
        clip.sample_pose(0.5, &mut mine);
        assert!(close(mine.joint_positions[0], vec3(1.0, 0.0, 0.0)), "hips halfway: {:?}", mine.joint_positions[0]);
        // and sampling didn't touch the clip's own pose (still at 0.8)
        assert!(close(clip.pose.joint_positions[0], vec3(1.6, 0.0, 0.0)));
    }

    // two bodies on the same clip at different times don't mess with each other (the clone bug)
    #[test]
    fn two_bodies_dont_affect_each_other() {
        let clip = test_clip();
        let mut a = clip.new_pose();
        let mut b = clip.new_pose();
        clip.sample_pose(0.25, &mut a);
        clip.sample_pose(0.75, &mut b);
        assert!(close(a.joint_positions[0], vec3(0.5, 0.0, 0.0)), "a moved: {:?}", a.joint_positions[0]);
        assert!(close(b.joint_positions[0], vec3(1.5, 0.0, 0.0)));
    }

    // the arm rides along with the hips: hips at x=1 means the arm's base is at (1, 1, 0)
    #[test]
    fn child_follows_parent() {
        let clip = test_clip();
        let mut pose = clip.new_pose();
        clip.sample_pose(0.5, &mut pose);
        assert!(close(pose.joint_positions[1], vec3(1.0, 1.0, 0.0)), "{:?}", pose.joint_positions[1]);
        // the arm's own spin shows up in its matrix: halfway = 45°, so a point 2 out along the arm
        // tilts toward -x, measured from the arm's base at (1, 1, 0)
        let tip = pose.skin_mats[1].transform_point3(vec3(0.0, 2.0, 0.0));
        let half = std::f32::consts::FRAC_PI_4;
        assert!(close(tip, vec3(1.0 - 2.0 * half.sin(), 1.0 + 2.0 * half.cos(), 0.0)), "{tip:?}");
    }

    // past the end of the clip it just holds the last frame
    #[test]
    fn past_the_end_holds_the_last_frame() {
        let clip = test_clip();
        let mut pose = clip.new_pose();
        clip.sample_pose(5.0, &mut pose);
        assert!(close(pose.joint_positions[0], vec3(2.0, 0.0, 0.0)));
    }
}
