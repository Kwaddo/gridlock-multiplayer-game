use gltf::mesh::util::ReadIndices;
use macroquad::models::{Mesh, Vertex};
use macroquad::prelude::*;

use super::glb::GlbImages;
use super::{AnimatedPart, RigidVertex, SkinVertex};

/// Build one animated part per skinned and rigid primitive, plus every
/// bind-pose vertex position (for the model's bounding box).
pub(super) fn build_parts(
    path: &str,
    skinned_prims: Vec<(gltf::Mesh, gltf::Primitive)>,
    rigid_prims: Vec<(gltf::Mesh, gltf::Primitive, usize)>,
    buffers: &[gltf::buffer::Data],
    images: &GlbImages,
) -> Result<(Vec<AnimatedPart>, Vec<Vec3>), String> {
    // Build one animated part per primitive so each part keeps its own texture/material.
    let mut parts: Vec<AnimatedPart> = Vec::new();
    let mut all_positions: Vec<Vec3> = Vec::new();

    println!("[anim] '{}' merging {} skinned primitives", path, skinned_prims.len());

    for (_mesh_ref, sp) in &skinned_prims {
        let reader = sp.reader(|buffer| Some(&buffers[buffer.index()]));
        let positions: Vec<Vec3> = reader
            .read_positions()
            .unwrap()
            .map(|p| vec3(p[0], p[1], p[2]))
            .collect();
        let normals: Vec<Vec3> = reader
            .read_normals()
            .map(|it| it.map(|n| vec3(n[0], n[1], n[2])).collect())
            .unwrap_or_else(|| vec![vec3(0.0, 1.0, 0.0); positions.len()]);
        let uvs: Vec<Vec2> = reader
            .read_tex_coords(0)
            .map(|uv| uv.into_f32().map(|u| vec2(u[0], u[1])).collect())
            .unwrap_or_else(|| vec![Vec2::ZERO; positions.len()]);
        let joints: Vec<[u16; 4]> = reader
            .read_joints(0)
            .unwrap()
            .into_u16()
            .collect();
        let mut weights: Vec<[f32; 4]> = reader
            .read_weights(0)
            .unwrap()
            .into_f32()
            .collect();

        if normals.len() != positions.len()
            || uvs.len() != positions.len()
            || joints.len() != positions.len()
            || weights.len() != positions.len()
        {
            return Err(format!("vertex attribute length mismatch in prim {} of '{}'", sp.index(), path));
        }

        for w in &mut weights {
            let sum = w[0] + w[1] + w[2] + w[3];
            if sum > 0.0001 {
                w[0] /= sum;
                w[1] /= sum;
                w[2] /= sum;
                w[3] /= sum;
            } else {
                *w = [1.0, 0.0, 0.0, 0.0];
            }
        }

        let mesh_texture = sp
            .material()
            .pbr_metallic_roughness()
            .base_color_texture()
            .and_then(|base_color| images.texture(base_color.texture().source().index()));

        let f = sp.material().pbr_metallic_roughness().base_color_factor();
        let mat_color = [
            (f[0] * 255.0) as u8,
            (f[1] * 255.0) as u8,
            (f[2] * 255.0) as u8,
            (f[3] * 255.0) as u8,
        ];

        // Primitive-local indices.
        let prim_indices: Vec<u16> = match reader.read_indices() {
            Some(ReadIndices::U8(it)) => it.map(|i| i as u16).collect(),
            Some(ReadIndices::U16(it)) => it.collect(),
            Some(ReadIndices::U32(it)) => {
                let mut out = Vec::new();
                for i in it {
                    if i > u16::MAX as u32 {
                        return Err(format!("index {} exceeds u16 in '{}'", i, path));
                    }
                    out.push(i as u16);
                }
                out
            }
            None => {
                let mut out = Vec::with_capacity(positions.len());
                for i in 0..positions.len() {
                    if i > u16::MAX as usize {
                        return Err(format!("vertex index {} exceeds u16 in '{}'", i, path));
                    }
                    out.push(i as u16);
                }
                out
            }
        };

        let runtime_vertices: Vec<Vertex> = positions
            .iter()
            .zip(normals.iter())
            .zip(uvs.iter())
            .map(|((p, n), uv)| Vertex {
                position: *p,
                uv: *uv,
                color: mat_color,
                normal: n.extend(0.0),
            })
            .collect();
        let skin_vertices: Vec<SkinVertex> = positions
            .iter()
            .zip(normals.iter())
            .zip(joints.iter())
            .zip(weights.iter())
            .map(|(((p, n), j), w)| SkinVertex {
                bind_position: *p,
                bind_normal: *n,
                joints: *j,
                weights: *w,
            })
            .collect();

        all_positions.extend_from_slice(&positions);
        parts.push(AnimatedPart {
            mesh: Mesh {
                vertices: runtime_vertices,
                indices: prim_indices,
                texture: mesh_texture,
            },
            skin_vertices,
            rigid_vertices: Vec::new(),
            skinned_vertex_count: positions.len(),
        });
    }

    println!("[anim] '{}' merging {} rigid primitives", path, rigid_prims.len());
    for (_mesh_ref, rp, node_idx) in &rigid_prims {
        let reader = rp.reader(|buffer| Some(&buffers[buffer.index()]));
        let positions: Vec<Vec3> = reader
            .read_positions()
            .unwrap()
            .map(|p| vec3(p[0], p[1], p[2]))
            .collect();
        let normals: Vec<Vec3> = reader
            .read_normals()
            .map(|it| it.map(|n| vec3(n[0], n[1], n[2])).collect())
            .unwrap_or_else(|| vec![vec3(0.0, 1.0, 0.0); positions.len()]);
        let uvs: Vec<Vec2> = reader
            .read_tex_coords(0)
            .map(|uv| uv.into_f32().map(|u| vec2(u[0], u[1])).collect())
            .unwrap_or_else(|| vec![Vec2::ZERO; positions.len()]);

        if normals.len() != positions.len() || uvs.len() != positions.len() {
            return Err(format!(
                "rigid vertex attribute length mismatch in prim {} of '{}'",
                rp.index(),
                path
            ));
        }

        let mesh_texture = rp
            .material()
            .pbr_metallic_roughness()
            .base_color_texture()
            .and_then(|base_color| images.texture(base_color.texture().source().index()));

        let f = rp.material().pbr_metallic_roughness().base_color_factor();
        let mat_color = [
            (f[0] * 255.0) as u8,
            (f[1] * 255.0) as u8,
            (f[2] * 255.0) as u8,
            (f[3] * 255.0) as u8,
        ];

        let prim_indices: Vec<u16> = match reader.read_indices() {
            Some(ReadIndices::U8(it)) => it.map(|i| i as u16).collect(),
            Some(ReadIndices::U16(it)) => it.collect(),
            Some(ReadIndices::U32(it)) => {
                let mut out = Vec::new();
                for i in it {
                    if i > u16::MAX as u32 {
                        return Err(format!("rigid index {} exceeds u16 in '{}'", i, path));
                    }
                    out.push(i as u16);
                }
                out
            }
            None => {
                let mut out = Vec::with_capacity(positions.len());
                for i in 0..positions.len() {
                    if i > u16::MAX as usize {
                        return Err(format!(
                            "rigid vertex index {} exceeds u16 in '{}'",
                            i, path
                        ));
                    }
                    out.push(i as u16);
                }
                out
            }
        };

        let mut rigid_runtime_vertices: Vec<Vertex> = Vec::with_capacity(positions.len());
        let mut rigid_vertices: Vec<RigidVertex> = Vec::with_capacity(positions.len());
        for i in 0..positions.len() {
            rigid_runtime_vertices.push(Vertex {
                position: positions[i],
                uv: uvs[i],
                color: mat_color,
                normal: normals[i].extend(0.0),
            });
            rigid_vertices.push(RigidVertex {
                bind_position: positions[i],
                bind_normal: normals[i],
                node_idx: *node_idx,
            });
        }
        all_positions.extend_from_slice(&positions);
        parts.push(AnimatedPart {
            mesh: Mesh {
                vertices: rigid_runtime_vertices,
                indices: prim_indices,
                texture: mesh_texture,
            },
            skin_vertices: Vec::new(),
            rigid_vertices,
            skinned_vertex_count: 0,
        });
    }
    if parts.is_empty() {
        return Err(format!("no renderable primitives found in '{}'", path));
    }
    Ok((parts, all_positions))
}
