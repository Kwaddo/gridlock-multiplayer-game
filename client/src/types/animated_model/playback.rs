use macroquad::models::Mesh;
use macroquad::prelude::*;

use super::constants::SKIN_INTERVAL;
use super::{AnimatedModel, KeyframesQuat, KeyframesVec3, Pose};

impl AnimatedModel {
    pub fn mesh(&self) -> &Mesh {
        self.meshes().next().expect("an animated model has at least one part")
    }

    // meshes to draw for the clip's own pose, same as before
    pub fn meshes(&self) -> impl Iterator<Item = &Mesh> {
        self.pose_meshes(&self.pose)
    }

    // meshes to draw for any pose, its cpu bent copy if it has one, otherwise the plain unbent mesh
    pub fn pose_meshes<'a>(&'a self, pose: &'a Pose) -> impl Iterator<Item = &'a Mesh> {
        let skinned = pose.cpu_meshes.as_deref();
        let bind = self.geometry.parts.iter().map(|p| &p.mesh);
        skinned.into_iter().flatten().chain(bind.filter(move |_| skinned.is_none()))
    }

    pub fn has_any_texture(&self) -> bool {
        self.geometry.parts.iter().any(|p| p.mesh.texture.is_some())
    }

    pub fn total_vertex_count(&self) -> usize {
        self.geometry.parts.iter().map(|p| p.mesh.vertices.len()).sum()
    }

    /// Advance the clip and bring its pose up to date: every frame when the
    /// GPU skins it, otherwise re-skinning on the CPU at most ~30 times a second.
    pub fn update(&mut self, dt: f32) {
        self.advance(dt);
        if self.gpu_drawn.get() {
            self.skin_accum = 0.0;
            self.pose();
        } else if self.skin_accum >= SKIN_INTERVAL {
            self.skin_accum = 0.0;
            self.pose();
            self.skin_on_cpu();
        }
    }

    /// Advance the clip's clock without re-skinning: cheap, for a model nobody
    /// is drawing right now. The next `update` re-skins it at once.
    pub fn advance(&mut self, dt: f32) {
        self.skin_accum += dt;
        if self.duration > 0.0 && !self.playback_paused {
            let start = self.playback_start.clamp(0.0, self.duration);
            let end = self.playback_end.clamp(start, self.duration);
            if self.playback_loop {
                let range = (end - start).max(0.0001);
                let mut t = self.time + dt;
                if t < start || t > end {
                    t = start;
                }
                while t > end {
                    t = start + (t - end);
                }
                if t < start {
                    t = start + (t - start).rem_euclid(range);
                }
                self.time = t;
                self.playback_finished = false;
            } else {
                self.time += dt;
                if self.time >= end {
                    self.time = end;
                    self.playback_paused = true;
                    self.playback_finished = true;
                }
            }
        }
    }

    /// Pose the skeleton at the current time: joint matrices for skinning and
    /// joint positions for hitboxes. Cheap (one matrix per joint).
    // the clip's own pose at its own clock, now just "sample me at my time"
    pub(super) fn pose(&mut self) {
        let mut pose = std::mem::take(&mut self.pose); // lend it out so we can read self while filling it
        self.sample_pose(self.time, &mut pose);
        self.pose = pose; // hand it back
    }

    // the clip's own pose, lend it out and bend the mesh for it, same as before
    fn skin_on_cpu(&mut self) {
        let mut pose = std::mem::take(&mut self.pose);
        self.skin_pose_on_cpu(&mut pose);
        self.pose = pose;
    }

    // bend the mesh on the cpu for this pose, only when the gpu cant do it
    // before this it only worked on the clip's own pose and kept the result on the clip
    pub fn skin_pose_on_cpu(&self, pose: &mut Pose) {
        let geometry = &self.geometry;
        let skinned = pose
            .cpu_meshes
            .get_or_insert_with(|| geometry.parts.iter().map(|p| clone_mesh(&p.mesh)).collect());

        for (part, mesh) in geometry.parts.iter().zip(skinned.iter_mut()) {
            for (i, vtx) in part.skin_vertices.iter().enumerate() {
                let mut p = Vec3::ZERO;
                let mut n = Vec3::ZERO;
                let mut total_w = 0.0;
                let bind_p = vtx.bind_position.extend(1.0);
                let bind_n = vtx.bind_normal.extend(0.0);

                for j in 0..4 {
                    let w = vtx.weights[j];
                    if w <= 0.0 {
                        continue;
                    }
                    let joint_idx = vtx.joints[j] as usize;
                    if joint_idx >= pose.skin_mats.len() {
                        continue;
                    }
                    let m = pose.skin_mats[joint_idx];
                    p += (m * bind_p).truncate() * w;
                    n += (m * bind_n).truncate() * w;
                    total_w += w;
                }

                let out_p = if total_w > 0.0 {
                    p
                } else {
                    vtx.bind_position
                };
                let out_n = if n.length_squared() > 0.0 {
                    n.normalize()
                } else {
                    vtx.bind_normal
                };
                mesh.vertices[i].position = out_p;
                mesh.vertices[i].normal = out_n.extend(0.0);
            }
        }

        // Rigid (unskinned) primitives still follow node animation transforms.
        for part_idx in 0..geometry.parts.len() {
            let part = &geometry.parts[part_idx];
            for (i, vtx) in part.rigid_vertices.iter().enumerate() {
                let out_idx = part.skinned_vertex_count + i;
                let global = pose.global_of(vtx.node_idx); // already worked out in sample_pose
                let out_p = global.transform_point3(vtx.bind_position);
                let out_n = (global * vtx.bind_normal.extend(0.0))
                    .truncate()
                    .normalize_or_zero();
                let Some(out) = pose.cpu_meshes.as_mut().and_then(|m| m[part_idx].vertices.get_mut(out_idx)) else {
                    continue;
                };
                out.position = out_p;
                out.normal = out_n.extend(0.0);
            }
        }
    }

    // lets us find a joint position by matching tokens within the joint name (head, hips, etc) so we can create hitboxes for the player
    pub fn joint_position_with_tokens(&self, tokens: &[&str]) -> Option<Vec3> {
        super::joints::find_joint(&self.joint_names, tokens)
            .and_then(|i| self.pose.joint_positions.get(i).copied())
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn play_from_start(&mut self, looping: bool) {
        self.time = 0.0;
        self.playback_loop = looping;
        self.playback_paused = false;
        self.playback_finished = false;
        self.playback_start = 0.0;
        self.playback_end = self.duration.max(0.0);
        self.skin_accum = SKIN_INTERVAL;
    }

    pub fn stop_at_start(&mut self) {
        self.time = 0.0;
        self.playback_loop = false;
        self.playback_paused = true;
        self.playback_finished = false;
        self.playback_start = 0.0;
        self.playback_end = self.duration.max(0.0);
        self.skin_accum = SKIN_INTERVAL;
    }

    pub fn is_playback_finished(&self) -> bool {
        self.playback_finished
    }

    pub fn play_segment(&mut self, start: f32, end: f32, looping: bool) {
        let s = start.max(0.0);
        let e = end.max(s).min(self.duration.max(0.0));
        self.time = s;
        self.playback_loop = looping;
        self.playback_paused = false;
        self.playback_finished = false;
        self.playback_start = s;
        self.playback_end = e;
        self.skin_accum = SKIN_INTERVAL;
    }
}

fn sample_interval(times: &[f32], t: f32) -> (usize, usize, f32) {
    if times.len() <= 1 {
        return (0, 0, 0.0);
    }
    if t <= times[0] {
        return (0, 0, 0.0);
    }
    let last = times.len() - 1;
    if t >= times[last] {
        return (last, last, 0.0);
    }
    let mut lo = 0usize;
    let mut hi = last;
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if times[mid] <= t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let t0 = times[lo];
    let t1 = times[hi];
    let alpha = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
    (lo, hi, alpha.clamp(0.0, 1.0))
}

pub(super) fn sample_vec3(track: &KeyframesVec3, t: f32) -> Vec3 {
    let (i0, i1, a) = sample_interval(&track.times, t);
    if i0 == i1 {
        return track.values[i0];
    }
    track.values[i0].lerp(track.values[i1], a)
}

pub(super)fn sample_quat(track: &KeyframesQuat, t: f32) -> Quat {
    let (i0, i1, a) = sample_interval(&track.times, t);
    if i0 == i1 {
        return track.values[i0];
    }
    track.values[i0].slerp(track.values[i1], a)
}

fn clone_mesh(mesh: &Mesh) -> Mesh {
    Mesh { vertices: mesh.vertices.clone(), indices: mesh.indices.clone(), texture: mesh.texture.clone() }
}
