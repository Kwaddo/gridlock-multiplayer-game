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
    // which character's mesh that copy is of, so a character switch makes a fresh one
    pub(super) cpu_meshes_for: usize,
}

/// Which nodes a pose step works on. Hidden bodies only need the nodes their
/// hitboxes are built from (see `AnimatedModel::hit_nodes`), and every step
/// works node by node with the same sums whichever nodes it is given, so
/// those nodes come out bit for bit the same as with `All`.
#[derive(Clone, Copy)]
pub enum Nodes<'a> {
    All,
    Only(&'a [usize]),
}

// f(node, mine, theirs) for each picked node both poses have, All in order like a zip
#[inline]
fn pair_each<A, B>(mine: &mut [A], theirs: &[B], nodes: Nodes, mut f: impl FnMut(usize, &mut A, &B)) {
    match nodes {
        Nodes::All => mine.iter_mut().zip(theirs).enumerate().for_each(|(i, (m, t))| f(i, m, t)),
        Nodes::Only(list) => {
            for &i in list {
                if let (Some(m), Some(t)) = (mine.get_mut(i), theirs.get(i)) {
                    f(i, m, t);
                }
            }
        }
    }
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
            cpu_meshes_for: 0,
        }
    }

    // mix another pose's bent joints into this one, w=0 all 'from', w=1 all this one
    // like ue's blend node, it mixes the local bends then finish_pose works out the rest
    // before this a clip change was an instant cut, nothing could mix two poses
    // the blends use nlerp (Quat::lerp, normalised) not slerp, like ue does: same ends, the short way round too,
    // within about a degree mid blend for bends under 90 degrees apart, and no acos + 3 sin per joint
    pub fn blend_from(&mut self, from: &Pose, w: f32) {
        self.blend_from_nodes(from, w, Nodes::All);
    }

    // blend_from on only some nodes, same sums on each, see Nodes
    pub fn blend_from_nodes(&mut self, from: &Pose, w: f32, nodes: Nodes) {
        pair_each(&mut self.local, &from.local, nodes, |_, mine, theirs| {
            mine.translation = theirs.translation.lerp(mine.translation, w);
            mine.rotation = theirs.rotation.lerp(mine.rotation, w); // nlerp takes the short way round too
            mine.scale = theirs.scale.lerp(mine.scale, w);
        });
        self.global_valid.iter_mut().for_each(|v| *v = false); // the mix changed the bends, old answers are stale
    }

    // move this pose toward the other pose by t (0 = stay, 1 = become other), for mixing many clips by weight
    // mixing a, b, c = start at a, mix_in(b, wb/(wa+wb)), then mix_in(c, wc/(wa+wb+wc))
    pub fn mix_in(&mut self, other: &Pose, t: f32) {
        self.mix_in_nodes(other, t, Nodes::All);
    }

    // mix_in on only some nodes, same sums on each, see Nodes
    pub fn mix_in_nodes(&mut self, other: &Pose, t: f32, nodes: Nodes) {
        if t <= 0.0 {
            return;
        }
        pair_each(&mut self.local, &other.local, nodes, |_, mine, theirs| {
            mine.translation = mine.translation.lerp(theirs.translation, t);
            mine.rotation = mine.rotation.lerp(theirs.rotation, t); // nlerp, see blend_from
            mine.scale = mine.scale.lerp(theirs.scale, t);
        });
        self.global_valid.iter_mut().for_each(|v| *v = false);
    }

    // copy bends of another pose to fade from later
    pub fn copy_from(&mut self, other: &Pose) {
        self.copy_from_nodes(other, Nodes::All);
    }

    // copy_from on only some nodes
    pub fn copy_from_nodes(&mut self, other: &Pose, nodes: Nodes) {
        match nodes {
            Nodes::All => self.local.clone_from(&other.local),
            Nodes::Only(_) => pair_each(&mut self.local, &other.local, nodes, |_, mine, theirs| *mine = *theirs),
        }
        self.global_valid.iter_mut().for_each(|v| *v = false);
    }

    // like blend_from but only where mask > 0, and toward the layer
    pub(super) fn blend_masked_from(&mut self, layer: &Pose, mask: &[f32], w: f32) {
        self.blend_masked_from_nodes(layer, mask, w, Nodes::All);
    }

    // blend_masked_from on only some nodes, same sums on each, see Nodes
    pub(super) fn blend_masked_from_nodes(&mut self, layer: &Pose, mask: &[f32], w: f32, nodes: Nodes) {
        pair_each(&mut self.local, &layer.local, nodes, |i, mine, theirs| {
            let Some(m) = mask.get(i) else { return };
            let t = m * w;
            if t <= 0.0 {
                return;
            }
            // fully the layer's (arms, fingers, head while aiming), just take it, before this it slerped to the same answer
            if t >= 1.0 {
                *mine = *theirs;
                return;
            }
            // slerp kept here, the mask's partial weights hold for as long as the layer shows, so they stay exact
            mine.translation = mine.translation.lerp(theirs.translation, t);
            mine.rotation = mine.rotation.slerp(theirs.rotation, t);
            mine.scale = mine.scale.lerp(theirs.scale, t);
        });
        self.global_valid.iter_mut().for_each(|v| *v = false);
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

    // whether a pose is the right size for this skeleton, switching to a skin with more bones (ty) makes old poses too small
    pub fn pose_fits(&self, pose: &Pose) -> bool {
        pose.local.len() == self.node_rest.len() && pose.skin_mats.len() == self.joint_nodes.len()
    }

    // sample a clip at a time and finish the pose, the simple path with no blending
    pub fn sample_pose(&self, time: f32, out: &mut Pose) {
        self.sample_local(time, out);
        self.finish_pose(out);
    }

    // only step 1 of sample_pose, how each joint is bent at this time, so two clips can be mixed before finishing
    // before this sampling and finishing were one step, so there was no gap to blend in
    pub fn sample_local(&self, time: f32, out: &mut Pose) {
        self.sample_local_nodes(time, out, Nodes::All);
    }

    // sample_local on only some nodes, each node is sampled on its own so the picked ones come out the same
    pub fn sample_local_nodes(&self, time: f32, out: &mut Pose, nodes: Nodes) {
        match nodes {
            Nodes::All => (0..self.node_rest.len()).for_each(|i| self.sample_node(i, time, out)), // loop over all nodes in the skeleton
            Nodes::Only(list) => list
                .iter()
                .filter(|&&i| i < self.node_rest.len())
                .for_each(|&i| self.sample_node(i, time, out)),
        }
    }

    // first step is to find out how each joint is bent off its parent from the keyframes
    #[inline]
    fn sample_node(&self, i: usize, time: f32, out: &mut Pose) {
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

    // step 2, turn the bent joints into where they actually are + the gpu matrices
    pub fn finish_pose(&self, out: &mut Pose) {
        // work out every node now not just joints, so drawing can just read them later
        // before this the cpu fallback worked out the non joint ones while drawing
        // only the cpu fallback's rigid (unskinned) vertices read non joint nodes, so a mesh without any skips them
        if self.geometry.parts.iter().any(|p| !p.rigid_vertices.is_empty()) {
            for node in 0..self.node_rest.len() {
                out.global(&self.node_parents, node);
            }
        }

        // second step is to find out where each joint is in worldspace and the GPU matrix for it (for skinning)
        for (j, &node) in self.joint_nodes.iter().enumerate() {
            let global = out.global(&self.node_parents, node);
            out.joint_positions[j] = global.w_axis.truncate(); // for hitboxes, where the joint's origin lands
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
            upper_mask: Vec::new(),
            inverse_bind: vec![Mat4::IDENTITY; 2],
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

    // the test clip but with a mesh of 'verts' vertices all hanging off the arm bone
    fn with_mesh(verts: usize) -> AnimatedModel {
        use super::super::{AnimatedPart, SkinVertex};
        use macroquad::models::Vertex;
        let mut clip = test_clip();
        let vertices: Vec<Vertex> = (0..verts).map(|i| Vertex::new(i as f32, 0.0, 0.0, 0.0, 0.0, WHITE)).collect();
        let skin = (0..verts)
            .map(|i| SkinVertex {
                bind_position: vec3(i as f32, 0.0, 0.0),
                bind_normal: Vec3::Y,
                joints: [1, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            })
            .collect();
        let indices = (0..verts as u16 / 3 * 3).collect();
        let part = AnimatedPart {
            mesh: Mesh { vertices, indices, texture: None },
            skin_vertices: skin,
            rigid_vertices: Vec::new(),
            skinned_vertex_count: verts,
        };
        clip.geometry = Rc::new(SharedGeometry { parts: vec![part], gpu: RefCell::new(GpuGeometry::Untried) });
        clip
    }

    // mixing three poses in a row with the running weights lands on their weighted average
    #[test]
    fn mix_in_gives_the_weighted_average() {
        let at = |x: f32| {
            let mut p = Pose::new(&[rest(Vec3::ZERO)], 0);
            p.local[0].translation = vec3(x, 0.0, 0.0);
            p
        };
        let (a, b, c) = (at(0.0), at(10.0), at(100.0));
        let (wa, wb, wc) = (0.5, 0.3, 0.2);
        let mut mix = a;
        mix.mix_in(&b, wb / (wa + wb));
        mix.mix_in(&c, wc / (wa + wb + wc));
        // 0*0.5 + 10*0.3 + 100*0.2 = 23
        assert!((mix.local[0].translation.x - 23.0).abs() < 1e-4, "{:?}", mix.local[0].translation);
    }

    // nlerp in the blends stays within a degree of slerp for bends up to 90 degrees apart, ends exact
    #[test]
    fn blends_stay_close_to_slerp() {
        let pose_at = |q: Quat| {
            let mut p = Pose::new(&[rest(Vec3::ZERO)], 0);
            p.local[0].rotation = q;
            p
        };
        let a = Quat::from_euler(EulerRot::YXZ, 0.3, -0.2, 0.1);
        let b = a * Quat::from_axis_angle(vec3(1.0, 2.0, -0.5).normalize(), std::f32::consts::FRAC_PI_2);
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            let mut mixed = pose_at(a);
            mixed.mix_in(&pose_at(b), t);
            let mut faded = pose_at(b);
            faded.blend_from(&pose_at(a), t);
            let want = a.slerp(b, t);
            for got in [mixed.local[0].rotation, faded.local[0].rotation] {
                assert!(got.angle_between(want).to_degrees() < 1.0, "t {t}: {got:?} vs {want:?}");
                assert!(got.is_normalized());
            }
        }
    }

    // the amy crash: one pose bent on one character's mesh, then on a bigger one after a character switch
    // before the fix the pose kept the first mesh copy and indexed past its end
    #[test]
    fn switching_character_rebuilds_the_cpu_mesh_copy() {
        let (small, big) = (with_mesh(1030), with_mesh(1500));
        let mut pose = small.new_pose();
        small.sample_pose(0.5, &mut pose);
        small.skin_pose_on_cpu(&mut pose);
        big.sample_pose(0.5, &mut pose);
        big.skin_pose_on_cpu(&mut pose); // panicked here before
        assert_eq!(big.pose_meshes(&pose).map(|m| m.vertices.len()).sum::<usize>(), 1500);
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
