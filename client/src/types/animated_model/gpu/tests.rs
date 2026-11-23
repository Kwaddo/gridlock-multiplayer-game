use super::super::{
    KeyframesQuat, KeyframesVec3, LocalTransform, NodeAnimationTrack, RigidVertex, SharedGeometry, SkinVertex,
};
use super::*;
use macroquad::models::{Mesh, Vertex};
use std::cell::Cell;
use std::rc::Rc;

/// Hips (joint 0) slide, the arm (joint 1) turns, and a gun node (not a
/// joint) rides on the arm: one skinned part over both joints, one rigid
/// part on the gun node, like the sawn-off first-person rig.
fn rig_with_rigid_gun() -> AnimatedModel {
    let rest = |translation: Vec3, rotation: Quat| LocalTransform { translation, rotation, scale: Vec3::ONE };
    let node_rest = vec![
        rest(Vec3::ZERO, Quat::IDENTITY),
        rest(vec3(0.0, 1.0, 0.0), Quat::IDENTITY),
        rest(vec3(0.4, 0.3, -0.2), Quat::from_rotation_y(0.7)),
    ];
    let tracks = vec![
        NodeAnimationTrack {
            translation: Some(KeyframesVec3 { times: vec![0.0, 1.0], values: vec![Vec3::ZERO, vec3(2.0, 0.5, 0.0)] }),
            ..Default::default()
        },
        NodeAnimationTrack {
            rotation: Some(KeyframesQuat {
                times: vec![0.0, 1.0],
                values: vec![Quat::IDENTITY, Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)],
            }),
            ..Default::default()
        },
        NodeAnimationTrack::default(),
    ];
    let vertex = |i: usize| Vertex::new(i as f32 * 0.3, 1.0 + i as f32 * 0.1, 0.2, 0.1, 0.9, WHITE);
    let skinned = AnimatedPart {
        mesh: Mesh { vertices: (0..4).map(vertex).collect(), indices: vec![0, 1, 2, 2, 1, 3], texture: None },
        skin_vertices: (0..4)
            .map(|i| SkinVertex {
                bind_position: vertex(i).position,
                bind_normal: Vec3::Y,
                joints: [0, 1, 0, 0],
                weights: [1.0 - i as f32 * 0.25, i as f32 * 0.25, 0.0, 0.0],
            })
            .collect(),
        rigid_vertices: Vec::new(),
        skinned_vertex_count: 4,
    };
    let rigid = AnimatedPart {
        mesh: Mesh { vertices: (0..3).map(vertex).collect(), indices: vec![0, 1, 2], texture: None },
        skin_vertices: Vec::new(),
        rigid_vertices: (0..3)
            .map(|i| RigidVertex { bind_position: vertex(i).position, bind_normal: Vec3::Y, node_idx: 2 })
            .collect(),
        skinned_vertex_count: 0,
    };
    AnimatedModel {
        geometry: Rc::new(SharedGeometry::new(vec![skinned, rigid])),
        gpu_drawn: Cell::new(false),
        pose: Pose::new(&node_rest, 2),
        node_rest,
        node_parents: vec![None, Some(0), Some(1)],
        node_tracks: tracks,
        joint_nodes: vec![0, 1],
        joint_names: vec!["hips".into(), "arm".into()],
        joint_roles: [None; super::super::joints::JOINT_ROLE_COUNT],
        spine_chain: Vec::new(),
        upper_mask: Vec::new(),
        inverse_bind: vec![Mat4::IDENTITY, Mat4::from_translation(vec3(0.0, -1.0, 0.0))],
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

// what the vertex shader computes for every vertex, in buffer order
fn shader_positions(batches: &[BatchData], joints: &[Mat4; MAX_GPU_JOINTS]) -> Vec<Vec3> {
    let mut out = Vec::new();
    for v in batches.iter().flat_map(|b| &b.vertices) {
        let mut skin = Mat4::ZERO;
        for k in 0..4 {
            skin += joints[v.joints[k] as usize] * v.weights[k];
        }
        out.push(skin.transform_point3(Vec3::from_array(v.position)));
    }
    out
}

// a rigid part now skins on the gpu, landing where the cpu fallback puts it
#[test]
fn rigid_parts_on_the_gpu_match_the_cpu_fallback() {
    let model = rig_with_rigid_gun();
    let (batches, rigid_base, rigid_nodes) = gpu_layout(&model.geometry.parts).expect("fits the joint table");
    assert_eq!((rigid_base, rigid_nodes.as_slice()), (2, &[2][..]));
    for time in [0.0, 0.35, 0.8] {
        let mut pose = model.new_pose();
        model.sample_pose(time, &mut pose);
        model.skin_pose_on_cpu(&mut pose);
        let cpu: Vec<Vec3> = model.pose_meshes(&pose).flat_map(|m| m.vertices.iter().map(|v| v.position)).collect();
        let gpu = shader_positions(&batches, &joint_table(&pose, rigid_base, &rigid_nodes));
        assert_eq!(cpu.len(), gpu.len());
        for (c, g) in cpu.iter().zip(&gpu) {
            assert!((*c - *g).length() < 1e-5, "t {time}: cpu {c:?} gpu {g:?}");
        }
    }
}

// more joints plus rigid nodes than the shader holds stays on the cpu
#[test]
fn too_many_slots_stay_on_the_cpu() {
    let model = rig_with_rigid_gun();
    let mut parts: Vec<AnimatedPart> = Vec::new();
    for part in &model.geometry.parts {
        let mut skin_vertices = part.skin_vertices.clone();
        skin_vertices.iter_mut().for_each(|v| v.joints[0] = (MAX_GPU_JOINTS - 1) as u16);
        parts.push(AnimatedPart {
            mesh: Mesh { vertices: part.mesh.vertices.clone(), indices: part.mesh.indices.clone(), texture: None },
            skin_vertices,
            rigid_vertices: part.rigid_vertices.clone(),
            skinned_vertex_count: part.skinned_vertex_count,
        });
    }
    assert!(gpu_layout(&parts).is_none());
    parts.pop(); // without the rigid part every joint still fits
    assert!(gpu_layout(&parts).is_some());
}
