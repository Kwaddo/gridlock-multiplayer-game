use gltf::animation::util::ReadOutputs;
use macroquad::prelude::*;

use super::{KeyframesQuat, KeyframesVec3, LocalTransform, NodeAnimationTrack};

/// The node hierarchy, rest pose and `anim`'s per-node tracks: returns the
/// node count, parents, rest transforms, tracks and clip duration.
#[allow(clippy::type_complexity)]
pub(super) fn load_nodes(
    path: &str,
    doc: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    anim: &gltf::Animation,
) -> Result<(usize, Vec<Option<usize>>, Vec<LocalTransform>, Vec<NodeAnimationTrack>, f32), String> {
    let node_count = doc.nodes().len();
    let mut node_parents = vec![None; node_count];
    for node in doc.nodes() {
        for child in node.children() {
            node_parents[child.index()] = Some(node.index());
        }
    }

    let mut node_rest = vec![
        LocalTransform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        node_count
    ];
    for node in doc.nodes() {
        let idx = node.index();
        let (t, r, s) = node.transform().decomposed();
        node_rest[idx] = LocalTransform {
            translation: vec3(t[0], t[1], t[2]),
            rotation: Quat::from_xyzw(r[0], r[1], r[2], r[3]).normalize(),
            scale: vec3(s[0], s[1], s[2]),
        };
    }

    let mut node_tracks = vec![NodeAnimationTrack::default(); node_count];
    let mut duration = 0.0f32;
    let mut track_count = 0usize;
    for channel in anim.channels() {
        let node_idx = channel.target().node().index();
        let r = channel.reader(|buffer| Some(&buffers[buffer.index()]));
        let times: Vec<f32> = match r.read_inputs() {
            Some(t) => t.collect(),
            None => continue,
        };
        if times.is_empty() {
            continue;
        }
        if let Some(last) = times.last() {
            duration = duration.max(*last);
        }
        let outputs = match r.read_outputs() {
            Some(o) => o,
            None => continue,
        };
        match outputs {
            ReadOutputs::Translations(v) => {
                let vals: Vec<Vec3> = v.map(|x| vec3(x[0], x[1], x[2])).collect();
                node_tracks[node_idx].translation = Some(KeyframesVec3 { times, values: vals });
                track_count += 1;
            }
            ReadOutputs::Rotations(v) => {
                let vals: Vec<Quat> = v
                    .into_f32()
                    .map(|x| Quat::from_xyzw(x[0], x[1], x[2], x[3]).normalize())
                    .collect();
                node_tracks[node_idx].rotation = Some(KeyframesQuat { times, values: vals });
                track_count += 1;
            }
            ReadOutputs::Scales(v) => {
                let vals: Vec<Vec3> = v.map(|x| vec3(x[0], x[1], x[2])).collect();
                node_tracks[node_idx].scale = Some(KeyframesVec3 { times, values: vals });
                track_count += 1;
            }
            ReadOutputs::MorphTargetWeights(_) => {}
        }
    }
    println!(
        "[anim] {} clip_duration={} track_count={}",
        path, duration, track_count
    );
    if duration <= 0.0 || track_count == 0 {
        return Err(format!(
            "animation clip has no effective tracks in '{}'",
            path
        ));
    }
    Ok((node_count, node_parents, node_rest, node_tracks, duration))
}
