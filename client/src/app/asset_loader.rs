use client::screens::PlayingState;
use client::types::animated_model::load_gltf_animated_model;
use macroquad::prelude::*;
use std::sync::Arc;

use super::obj_loader::{load_gltf_static_mesh, load_obj_mesh_with_materials};

pub async fn load_asset_entry(key: String, path: String, playing_state: &mut PlayingState) {
    let lower = path.to_lowercase();
    let is_obj = lower.ends_with(".obj");
    let is_gltf = lower.ends_with(".glb") || lower.ends_with(".gltf");
    let is_sound = lower.ends_with(".ogg") || lower.ends_with(".wav") || lower.ends_with(".mp3");

    if is_asset_cached(playing_state, &key, &path, is_obj, is_gltf, is_sound) {
        return;
    }

    if is_obj {
        match load_obj_mesh_with_materials(&path).await {
            Ok(mesh) => {
                let vertex_count = mesh.vertices.len();
                playing_state.models.insert(key.clone(), mesh);
                playing_state
                    .asset_source_paths
                    .insert(key.clone(), path.clone());
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
                playing_state
                    .asset_source_paths
                    .insert(key.clone(), path.clone());
                println!(
                    "Loaded animated GLTF model: {} ({} verts) from {}",
                    key, vertex_count, path
                );
            }
            Err(anim_err) => match load_gltf_static_mesh(&path).await {
                Ok(mesh) => {
                    let vertex_count = mesh.vertices.len();
                    playing_state.models.insert(key.clone(), mesh);
                    playing_state
                        .asset_source_paths
                        .insert(key.clone(), path.clone());
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
    } else if is_sound {
        match std::fs::read(&path) {
            Ok(bytes) => {
                playing_state
                    .sound_samples
                    .insert(key.clone(), Arc::from(bytes.into_boxed_slice()));
                playing_state
                    .asset_source_paths
                    .insert(key.clone(), path.clone());
                println!("Loaded sound bytes: {} from {}", key, path);
            }
            Err(e) => {
                println!("Failed to load sound bytes {} from {}: {:?}", key, path, e);
            }
        }
    } else {
        match load_texture(&path).await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                playing_state.assets.insert(key.clone(), tex);
                playing_state
                    .asset_source_paths
                    .insert(key.clone(), path.clone());
                println!("Loaded texture: {} from {}", key, path);
            }
            Err(e) => {
                println!("Failed to load texture {} from {}: {:?}", key, path, e);
            }
        }
    }
}

fn is_asset_cached(
    playing_state: &PlayingState,
    key: &str,
    path: &str,
    is_obj: bool,
    is_gltf: bool,
    is_sound: bool,
) -> bool {
    let Some(cached_path) = playing_state.asset_source_paths.get(key) else {
        return false;
    };
    if cached_path != path {
        return false;
    }

    if is_obj || is_gltf {
        if playing_state.models.contains_key(key) || playing_state.animated_models.contains_key(key)
        {
            println!("Asset cache hit: {} ({})", key, path);
            return true;
        }
    } else if is_sound {
        if playing_state.sound_samples.contains_key(key) {
            println!("Asset cache hit: {} ({})", key, path);
            return true;
        }
    } else if playing_state.assets.contains_key(key) {
        println!("Asset cache hit: {} ({})", key, path);
        return true;
    }

    false
}
