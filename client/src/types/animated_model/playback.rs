use macroquad::models::Mesh;
use macroquad::prelude::*;
use std::rc::Rc;

use super::constants::SKIN_INTERVAL;
use super::{AnimatedModel, KeyframesQuat, KeyframesVec3};

impl AnimatedModel {
    pub fn mesh(&self) -> &Mesh {
        self.meshes().next().expect("an animated model has at least one part")
    }

    pub fn meshes(&self) -> impl Iterator<Item = &Mesh> {
        let skinned = self.skinned.as_deref();
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
    pub(super) fn pose(&mut self) {
        for (i, pose) in self.node_pose.iter_mut().enumerate() {
            let mut p = self.node_rest[i];
            if let Some(track) = &self.node_tracks[i].translation {
                p.translation = sample_vec3(track, self.time);
            }
            if let Some(track) = &self.node_tracks[i].rotation {
                p.rotation = sample_quat(track, self.time);
            }
            if let Some(track) = &self.node_tracks[i].scale {
                p.scale = sample_vec3(track, self.time);
            }
            *pose = p;
            self.node_global_valid[i] = false;
        }

        for i in 0..self.joint_nodes.len() {
            let node_idx = self.joint_nodes[i];
            let global = self.global_for_node(node_idx);
            // here we are caching each individual joint position for every joint in the skeleton so we can do hit detection against them for the player
            self.joint_positions[i] = global.transform_point3(Vec3::ZERO);
            self.skin_mats[i] = global * self.inverse_bind[i];
        }
    }

    /// Skin the mesh on the CPU with the current pose.
    fn skin_on_cpu(&mut self) {
        let geometry = Rc::clone(&self.geometry);
        let skinned = self
            .skinned
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
                    if joint_idx >= self.skin_mats.len() {
                        continue;
                    }
                    let m = self.skin_mats[joint_idx];
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
                let global = self.global_for_node(vtx.node_idx);
                let out_p = global.transform_point3(vtx.bind_position);
                let out_n = (global * vtx.bind_normal.extend(0.0))
                    .truncate()
                    .normalize_or_zero();
                let Some(out) = self.skinned.as_mut().and_then(|m| m[part_idx].vertices.get_mut(out_idx)) else {
                    continue;
                };
                out.position = out_p;
                out.normal = out_n.extend(0.0);
            }
        }
    }

    fn global_for_node(&mut self, node_idx: usize) -> Mat4 {
        if self.node_global_valid[node_idx] {
            return self.node_global[node_idx];
        }
        let local = self.node_pose[node_idx].to_mat4();
        let global = if let Some(parent) = self.node_parents[node_idx] {
            self.global_for_node(parent) * local
        } else {
            local
        };
        self.node_global[node_idx] = global;
        self.node_global_valid[node_idx] = true;
        global
    }
    // lets us find a joint position by matching tokens within the joint name (head, hips, etc) so we can create hitboxes for the player
    pub fn joint_position_with_tokens(&self, tokens: &[&str]) -> Option<Vec3> {
        super::joints::find_joint(&self.joint_names, tokens)
            .and_then(|i| self.joint_positions.get(i).copied())
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

fn sample_vec3(track: &KeyframesVec3, t: f32) -> Vec3 {
    let (i0, i1, a) = sample_interval(&track.times, t);
    if i0 == i1 {
        return track.values[i0];
    }
    track.values[i0].lerp(track.values[i1], a)
}

fn sample_quat(track: &KeyframesQuat, t: f32) -> Quat {
    let (i0, i1, a) = sample_interval(&track.times, t);
    if i0 == i1 {
        return track.values[i0];
    }
    track.values[i0].slerp(track.values[i1], a)
}

fn clone_mesh(mesh: &Mesh) -> Mesh {
    Mesh { vertices: mesh.vertices.clone(), indices: mesh.indices.clone(), texture: mesh.texture.clone() }
}
