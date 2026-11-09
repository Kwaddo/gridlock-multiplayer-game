use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use macroquad::prelude::*;

use super::clip_select::select_clip;
use super::constants::SKIN_INTERVAL;
use super::glb::import_glb;
use super::nodes::load_nodes;
use super::{AnimatedModel, ClipSelectionMode, KeyframesQuat, KeyframesVec3, LocalTransform, NodeAnimationTrack};

/*
    an animation on its own without mesh or textures,
    loaded from the bones only files the converter makes (the .glb files in models/animations)
    then put on a character with with_clip, which retargets it to that body 
    before this every animation file carried its own copy of the whole character (~9 MB each)
*/
pub struct Clip {
    // keyed by the bone's short name (after the ':'), so mixamorig1:hips matches mixamorig:hips
    tracks: HashMap<String, NodeAnimationTrack>,
    // the source skeleton's rest pose per bone, the retarget works from changes off this
    rest: HashMap<String, LocalTransform>,
    duration: f32,
}

// mixamorig1:LeftArm -> leftarm, so bones match whatever prefix the download got
pub(super) fn short_bone_name(name: &str) -> String {
    name.rsplit(':').next().unwrap_or(name).to_ascii_lowercase()
}

// load a bones only glb as a clip
pub fn load_clip(path: &str, bytes: &[u8]) -> Result<Clip, String> {
    let (doc, buffers, _images) = import_glb(path, bytes)?;
    let anim = select_clip(path, &doc, &buffers, ClipSelectionMode::LongestDuration, None)?;
    let (_count, _parents, node_rest, node_tracks, duration) = load_nodes(path, &doc, &buffers, &anim)?;
    let mut tracks = HashMap::new();
    let mut rest = HashMap::new();
    for (node, track) in doc.nodes().zip(node_tracks) {
        let name = short_bone_name(node.name().unwrap_or(""));
        if name.is_empty() {
            continue;
        }
        rest.insert(name.clone(), node_rest[node.index()]);
        tracks.insert(name, track);
    }
    Ok(Clip { tracks, rest, duration })
}

impl Clip {
    pub fn duration(&self) -> f32 {
        self.duration
    }
}

impl AnimatedModel {
    /*
        this character playing clip
        shares the character's mesh and skeleton, only the keyframes are new
        rotations: copied as a change from the source's rest pose onto this body's rest pose
        hips movement: scaled by hip height, so a taller or shorter body still bobs and crouches right
        every other bone keeps this character's own lengths, the clip's lengths (from whoever it was downloaded on) are ignored
        bones the clip doesnt have (extra bones on some skins) just stay at rest
    */
    pub fn with_clip(&self, clip: &Clip) -> AnimatedModel {
        let mut node_tracks = vec![NodeAnimationTrack::default(); self.node_rest.len()];
        for (j, &node) in self.joint_nodes.iter().enumerate() {
            let bone = short_bone_name(&self.joint_names[j]);
            let (Some(track), Some(src_rest)) = (clip.tracks.get(&bone), clip.rest.get(&bone)) else {
                continue;
            };
            let dst_rest = self.node_rest[node];
            node_tracks[node].rotation = track.rotation.as_ref().map(|k| retarget_rotations(k, src_rest.rotation, dst_rest.rotation));
            if bone == "hips" {
                node_tracks[node].translation = track
                    .translation
                    .as_ref()
                    .map(|k| retarget_hips(k, src_rest.translation, dst_rest.translation));
            }
        }
        let mut model = AnimatedModel {
            geometry: Rc::clone(&self.geometry),
            // the character's mesh is already on the gpu (prepare_gpu at load), so start as gpu drawn
            // before this every fresh clip did one pointless cpu bend first, right after a character switch
            gpu_drawn: Cell::new(matches!(*self.geometry.gpu.borrow(), super::gpu::GpuGeometry::Ready(_))),
            pose: self.new_pose(),
            node_rest: self.node_rest.clone(),
            node_parents: self.node_parents.clone(),
            node_tracks,
            joint_nodes: self.joint_nodes.clone(),
            joint_names: self.joint_names.clone(),
            joint_roles: self.joint_roles,
            spine_chain: self.spine_chain.clone(),
            upper_mask: self.upper_mask.clone(),
            inverse_bind: self.inverse_bind.clone(),
            duration: clip.duration,
            time: 0.0,
            skin_accum: SKIN_INTERVAL,
            playback_loop: true,
            playback_paused: false,
            playback_finished: false,
            playback_start: 0.0,
            playback_end: clip.duration.max(0.0),
            normalize_scale: self.normalize_scale,
            y_offset: self.y_offset,
        };
        model.pose();
        model
    }
}

// each key: this body's rest × (how far the source bone turned from its own rest)
fn retarget_rotations(src: &KeyframesQuat, src_rest: Quat, dst_rest: Quat) -> KeyframesQuat {
    let undo_src = src_rest.inverse();
    KeyframesQuat {
        times: src.times.clone(),
        values: src.values.iter().map(|q| (dst_rest * (undo_src * *q)).normalize()).collect(),
    }
}

// hips: this body's rest + the source's movement off its rest, scaled by how tall the two hips are
fn retarget_hips(src: &KeyframesVec3, src_rest: Vec3, dst_rest: Vec3) -> KeyframesVec3 {
    let ratio = dst_rest.length() / src_rest.length().max(0.0001);
    KeyframesVec3 {
        times: src.times.clone(),
        values: src.values.iter().map(|v| dst_rest + (*v - src_rest) * ratio).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quats(values: Vec<Quat>) -> KeyframesQuat {
        KeyframesQuat { times: (0..values.len()).map(|i| i as f32).collect(), values }
    }

    #[test]
    fn bone_names_match_across_prefixes() {
        assert_eq!(short_bone_name("mixamorig1:LeftArm"), "leftarm");
        assert_eq!(short_bone_name("mixamorig:leftarm"), "leftarm");
        assert_eq!(short_bone_name("Hips"), "hips");
    }

    #[test]
    fn same_rest_pose_copies_rotations_as_is() {
        let rest = Quat::from_rotation_z(0.3);
        let src = quats(vec![Quat::from_rotation_x(0.5), Quat::from_rotation_y(-0.2)]);
        let out = retarget_rotations(&src, rest, rest);
        for (a, b) in out.values.iter().zip(&src.values) {
            assert!(a.abs_diff_eq(*b, 1e-5) || a.abs_diff_eq(-*b, 1e-5), "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn different_rest_pose_carries_the_change_not_the_raw_rotation() {
        let src_rest = Quat::from_rotation_z(0.4);
        let dst_rest = Quat::from_rotation_z(-0.1);
        // the source bends 0.5 about x away from its rest
        let bend = Quat::from_rotation_x(0.5);
        let out = retarget_rotations(&quats(vec![src_rest, src_rest * bend]), src_rest, dst_rest);
        // at rest it lands on this body's rest, bent it lands on this body's rest + the same bend
        assert!(out.values[0].abs_diff_eq(dst_rest, 1e-5));
        assert!(out.values[1].abs_diff_eq(dst_rest * bend, 1e-5));
    }

    // a character's skeleton and size without its mesh or textures (textures need a game window, tests dont have one)
    fn skeleton_only(path: &str, bytes: &[u8]) -> Result<AnimatedModel, String> {
        let (doc, buffers, _images) = import_glb(path, bytes)?;
        let skin = doc.skins().next().expect("character has a skin");
        let anim = select_clip(path, &doc, &buffers, ClipSelectionMode::LongestDuration, None).unwrap();
        let (_n, node_parents, node_rest, node_tracks, duration) = load_nodes(path, &doc, &buffers, &anim).unwrap();
        let joint_nodes: Vec<usize> = skin.joints().map(|j| j.index()).collect();
        let joint_names: Vec<String> =
            joint_nodes.iter().map(|&i| doc.nodes().nth(i).unwrap().name().unwrap_or("").to_ascii_lowercase()).collect();
        let inverse_bind: Vec<Mat4> = skin
            .reader(|b| Some(&buffers[b.index()]))
            .read_inverse_bind_matrices()
            .map(|m| m.map(|m| Mat4::from_cols_array_2d(&m)).collect())
            .unwrap_or_else(|| vec![Mat4::IDENTITY; joint_nodes.len()]);
        // height from the mesh's bind positions, same as the real loader
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for mesh in doc.meshes() {
            for prim in mesh.primitives() {
                if let Some(ps) = prim.reader(|b| Some(&buffers[b.index()])).read_positions() {
                    for p in ps {
                        lo = lo.min(p[1]);
                        hi = hi.max(p[1]);
                    }
                }
            }
        }
        let normalize_scale = 1.5 / (hi - lo).max(0.0001);
        Ok(AnimatedModel {
            geometry: Rc::new(super::super::SharedGeometry {
                parts: Vec::new(),
                gpu: std::cell::RefCell::new(super::super::gpu::GpuGeometry::Untried),
            }),
            gpu_drawn: Cell::new(false),
            pose: super::super::Pose::new(&node_rest, joint_nodes.len()),
            joint_roles: super::super::joints::resolve_joint_roles(&joint_names),
            spine_chain: super::super::aim_bend::resolve_spine_chain(&joint_names, &joint_nodes),
            upper_mask: Vec::new(),
            node_rest,
            node_parents,
            node_tracks,
            joint_nodes,
            joint_names,
            inverse_bind,
            duration,
            time: 0.0,
            skin_accum: 0.0,
            playback_loop: true,
            playback_paused: false,
            playback_finished: false,
            playback_start: 0.0,
            playback_end: duration,
            normalize_scale,
            y_offset: -lo * normalize_scale,
        })
    }

    // these must work, the rest are proportion tests that are allowed to fail (Ahmed's priority)
    const MUST_WORK: [&str; 3] = ["Ch15", "Skin_Pete", "Skin_Vecna"];

    // the real converted files, skipped when theyre not there (client/assets is git ignored)
    // every animation on every character: loads, retargets, and the pose comes out sane
    #[test]
    fn real_animations_retarget_onto_every_character() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models");
        let (chars, anims) = (root.join("characters"), root.join("animations"));
        if !chars.is_dir() || !anims.is_dir() {
            eprintln!("no converted assets, skipping");
            return;
        }
        let glbs = |dir: &std::path::Path| -> Vec<std::path::PathBuf> {
            let mut v: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "glb")).collect();
            v.sort();
            v
        };
        let clips: Vec<(String, Clip)> = glbs(&anims)
            .iter()
            .map(|p| {
                let name = p.file_stem().unwrap().to_string_lossy().to_string();
                let clip = load_clip(&name, &std::fs::read(p).unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
                (name, clip)
            })
            .collect();
        assert!(!clips.is_empty());
        for path in glbs(&chars) {
            let who = path.file_stem().unwrap().to_string_lossy().to_string();
            let character = match skeleton_only(&who, &std::fs::read(&path).unwrap()) {
                Ok(c) => c,
                Err(e) => {
                    assert!(!MUST_WORK.contains(&who.as_str()), "{who} must load: {e}");
                    eprintln!("{who}: did not load ({e})");
                    continue;
                }
            };
            let mut matched_bones = 0;
            for (name, clip) in &clips {
                let model = character.with_clip(clip);
                let mut pose = model.new_pose();
                model.sample_pose(clip.duration() * 0.5, &mut pose);
                assert!(pose.joint_positions.iter().all(|p| p.is_finite()), "{who} + {name}: nan in pose");
                matched_bones = model.node_tracks.iter().filter(|t| t.rotation.is_some()).count();
            }
            // the standing idle's hips, in the game's metres (after the model's own scale)
            let idle = clips.iter().find(|(n, _)| n == "Rifle_Idle").map(|(_, c)| character.with_clip(c));
            let hips_y = idle.map(|m| {
                let mut pose = m.new_pose();
                m.sample_pose(0.0, &mut pose);
                m.joint_role_position_in(&pose, super::super::JointRole::Hips).unwrap_or_default().y * m.normalize_scale + m.y_offset
            });
            eprintln!("{who}: {} joints, {matched_bones} retargeted, standing hips at {:?} m", character.joint_nodes.len(), hips_y);
        }
    }

    #[test]
    fn hips_movement_scales_with_hip_height() {
        let src_rest = vec3(0.0, 0.0, 100.0);
        let dst_rest = vec3(0.0, 0.0, 50.0); // half as tall
        let src = KeyframesVec3 { times: vec![0.0, 1.0], values: vec![src_rest, src_rest + vec3(0.0, 0.0, -20.0)] };
        let out = retarget_hips(&src, src_rest, dst_rest);
        assert!(out.values[0].abs_diff_eq(dst_rest, 1e-4)); // standing = this body's own hip height
        assert!(out.values[1].abs_diff_eq(dst_rest + vec3(0.0, 0.0, -10.0), 1e-4)); // crouch drop halves too
    }
}
