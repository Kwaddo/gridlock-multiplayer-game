use macroquad::prelude::*;
use std::cell::Cell;
use std::collections::HashMap;

use super::clip_select::select_clip;
use super::constants::SKIN_INTERVAL;
use super::glb::import_glb;
use super::nodes::load_nodes;
use super::parts::build_parts;
use super::{AnimatedModel, ClipSelectionMode, Pose, SharedGeometry};

pub fn load_gltf_animated_model(path: &str, bytes: &[u8]) -> Result<AnimatedModel, String> {
    load_gltf_animated_model_internal(path, bytes, ClipSelectionMode::LongestDuration, None, false)
}

pub fn load_gltf_animated_model_with_mode(
    path: &str,
    bytes: &[u8],
    mode: ClipSelectionMode,
) -> Result<AnimatedModel, String> {
    load_gltf_animated_model_internal(path, bytes, mode, None, false)
}

pub fn load_gltf_animated_model_with_clip_index(
    path: &str,
    bytes: &[u8],
    clip_index: usize,
) -> Result<AnimatedModel, String> {
    load_gltf_animated_model_internal(
        path,
        bytes,
        ClipSelectionMode::LongestDuration,
        Some(clip_index),
        false,
    )
}

pub fn load_gltf_animated_model_with_unskinned(
    path: &str,
    bytes: &[u8],
    include_unskinned_primitives: bool,
) -> Result<AnimatedModel, String> {
    load_gltf_animated_model_internal(
        path,
        bytes,
        ClipSelectionMode::LongestDuration,
        None,
        include_unskinned_primitives,
    )
}

fn load_gltf_animated_model_internal(
    path: &str,
    bytes: &[u8],
    mode: ClipSelectionMode,
    forced_clip_index: Option<usize>,
    include_unskinned_primitives: bool,
) -> Result<AnimatedModel, String> {
    let (doc, buffers, images) = import_glb(path, bytes)?;

    let mesh_count = doc.meshes().count();
    let skin_count = doc.skins().count();
    let anim_count = doc.animations().count();
    println!(
        "[anim] {} meshes={} skins={} animations={}",
        path, mesh_count, skin_count, anim_count
    );

    let skin = doc
        .skins()
        .next()
        .ok_or_else(|| format!("no skin found in '{}'", path))?;

    let mut prim_nodes: HashMap<(usize, usize), usize> = HashMap::new();
    if include_unskinned_primitives {
        // Map mesh primitive -> node index so rigid primitives can follow node transforms.
        for node in doc.nodes() {
            if let Some(node_mesh) = node.mesh() {
                for prim in node_mesh.primitives() {
                    prim_nodes
                        .entry((node_mesh.index(), prim.index()))
                        .or_insert(node.index());
                }
            }
        }
    }

    // Collect all skinned primitives plus rigid (unskinned) primitives.
    let mut skinned_prims: Vec<(gltf::Mesh, gltf::Primitive)> = Vec::new();
    let mut rigid_prims: Vec<(gltf::Mesh, gltf::Primitive, usize)> = Vec::new();
    for mesh in doc.meshes() {
        for prim in mesh.primitives() {
            let r = prim.reader(|buffer| Some(&buffers[buffer.index()]));
            if r.read_positions().is_some() && r.read_joints(0).is_some() && r.read_weights(0).is_some() {
                skinned_prims.push((mesh.clone(), prim));
            } else if include_unskinned_primitives && r.read_positions().is_some() {
                let node_idx = prim_nodes
                    .get(&(mesh.index(), prim.index()))
                    .copied()
                    .unwrap_or(0);
                rigid_prims.push((mesh.clone(), prim, node_idx));
            }
        }
    }
    if skinned_prims.is_empty() {
        return Err(format!("no skinned primitive (POSITION+JOINTS_0+WEIGHTS_0) found in '{}'", path));
    }
    let mesh = skinned_prims[0].0.clone();
    let first_prim_index = skinned_prims[0].1.index();
    let anim = select_clip(path, &doc, &buffers, mode, forced_clip_index)?;

    println!(
        "[anim] {} using mesh={} first_prim={} anim={}",
        path,
        mesh.index(),
        first_prim_index,
        anim.index()
    );

    let (parts, all_positions) = build_parts(path, skinned_prims, rigid_prims, &buffers, &images)?;
    let total_vertices: usize = parts.iter().map(|p| p.mesh.vertices.len()).sum();
    let total_indices: usize = parts.iter().map(|p| p.mesh.indices.len()).sum();
    println!(
        "[anim] '{}' total vertices={} indices={} parts={}",
        path,
        total_vertices,
        total_indices,
        parts.len()
    );

    let (_node_count, node_parents, node_rest, node_tracks, duration) =
        load_nodes(path, &doc, &buffers, &anim)?;

    let joint_nodes: Vec<usize> = skin.joints().map(|j| j.index()).collect();
    let node_names: Vec<String> = doc
        .nodes()
        .map(|n| n.name().unwrap_or("").to_ascii_lowercase())
        .collect();

    // here we are collecting joint names for each joint node so we can do token matching against them to find hitbox positions for the player
    let joint_names: Vec<String> = joint_nodes
        .iter()
        .map(|&i| node_names.get(i).cloned().unwrap_or_default())
        .collect();
    let joint_roles = super::joints::resolve_joint_roles(&joint_names);
    let spine_chain = super::aim_bend::resolve_spine_chain(&joint_names, &joint_nodes);
    let upper_mask = super::upper_body::upper_body_mask(&node_names, &node_parents);
    let inverse_bind: Vec<Mat4> = match skin
        .reader(|buffer| Some(&buffers[buffer.index()]))
        .read_inverse_bind_matrices()
    {
        Some(mats) => mats.map(|m| Mat4::from_cols_array_2d(&m)).collect(),
        None => vec![Mat4::IDENTITY; joint_nodes.len()],
    };

    if inverse_bind.len() != joint_nodes.len() {
        return Err(format!(
            "inverse bind matrix count {} != joint count {} in '{}'",
            inverse_bind.len(),
            joint_nodes.len(),
            path
        ));
    }

    // Compute bounding box from bind pose to determine scale/offset
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for p in &all_positions {
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }
    let raw_height = (max_y - min_y).max(0.0001);
    let target_height = 1.5_f32; // slightly shorter than eye height (1.6) so players look right
    let normalize_scale = target_height / raw_height;
    let y_offset = -min_y * normalize_scale; // shift feet to y=0

    let mut model = AnimatedModel {
        geometry: SharedGeometry::shared(parts),
        gpu_drawn: Cell::new(false),
        pose: Pose::new(&node_rest, skin.joints().len()),
        node_rest,
        node_parents,
        node_tracks,
        joint_nodes,
        joint_names,
        joint_roles,
        spine_chain,
        upper_mask,
        inverse_bind,
        duration,
        time: 0.0,
        skin_accum: SKIN_INTERVAL, // force first update
        playback_loop: true,
        playback_paused: false,
        playback_finished: false,
        playback_start: 0.0,
        playback_end: duration.max(0.0),
        normalize_scale,
        y_offset,
    };
    // Joint positions (hitboxes) are valid from the start; vertices are skinned
    // the first time the clip is drawn.
    model.pose();

    println!(
        "[anim] {} raw_height={:.1} scale={:.4} y_offset={:.2}",
        path, raw_height, normalize_scale, y_offset
    );

    Ok(model)
}
