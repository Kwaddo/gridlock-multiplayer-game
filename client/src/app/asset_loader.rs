use client::screens::PlayingState;
use client::types::animated_model::load_gltf_animated_model;
use macroquad::prelude::*;

use super::obj_loader::{load_gltf_static_mesh, load_obj_mesh_with_materials};

pub async fn load_asset_entry(key: String, path: String, playing_state: &mut PlayingState) {
    let lower = path.to_lowercase();
    let is_obj = lower.ends_with(".obj");
    let is_gltf = lower.ends_with(".glb") || lower.ends_with(".gltf");

    if is_obj {
        match load_obj_mesh_with_materials(&path).await {
            Ok(mesh) => {
                let vertex_count = mesh.vertices.len();
                playing_state.models.insert(key.clone(), mesh);
                println!(
                    "Loaded model: {} ({} verts) from {}",
                    key, vertex_count, path
                );
            }
            Err(e) => {
                println!("Failed to load model {} from {}: {:?}", key, path, e);
            }
        }
    } else if is_gltf {
        match load_gltf_animated_model(&path) {
            Ok(animated) => {
                let vertex_count = animated.mesh().vertices.len();
                playing_state.animated_models.insert(key.clone(), animated);
                println!(
                    "Loaded animated GLTF model: {} ({} verts) from {}",
                    key, vertex_count, path
                );
            }
            Err(anim_err) => match load_gltf_static_mesh(&path).await {
                Ok(mesh) => {
                    let vertex_count = mesh.vertices.len();
                    playing_state.models.insert(key.clone(), mesh);
                    println!(
                        "Loaded static GLTF model: {} ({} verts) from {}",
                        key, vertex_count, path
                    );
                }
                Err(static_err) => {
                    println!(
                        "Failed GLTF load {} from {}. animated_err={:?} static_err={:?}",
                        key, path, anim_err, static_err
                    );
                }
            },
        }
    } else {
        match load_texture(&path).await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                playing_state.assets.insert(key.clone(), tex);
                println!("Loaded texture: {} from {}", key, path);
            }
            Err(e) => {
                println!("Failed to load texture {} from {}: {:?}", key, path, e);
            }
        }
    }
}
