use client::screens::PlayingState;
use client::types::animated_model::load_gltf_animated_model;
use macroquad::prelude::*;
use std::path::Path;
use std::sync::Arc;

use super::obj_loader::{load_gltf_static_mesh, load_gltf_static_mesh_lit, load_obj_mesh_with_materials};

pub async fn load_asset_entry(key: String, path: String, playing_state: &mut PlayingState) {
    let resolved_path = resolve_runtime_asset_path(&path);
    let lower = resolved_path.to_lowercase();
    let is_obj = lower.ends_with(".obj");
    let is_gltf = lower.ends_with(".glb") || lower.ends_with(".gltf");
    let is_sound = lower.ends_with(".ogg") || lower.ends_with(".wav") || lower.ends_with(".mp3");

    if is_asset_cached(playing_state, &key, &path, is_obj, is_gltf, is_sound) {
        return;
    }

    if is_obj {
        match load_obj_mesh_with_materials(&resolved_path).await {
            Ok(mesh) => {
                let vertex_count = mesh.vertices.len();
                playing_state.models.insert(key.clone(), mesh);
                playing_state
                    .asset_source_paths
                    .insert(key.clone(), path.clone());
                println!(
                    "Loaded model: {} ({} verts) from {}",
                    key, vertex_count, resolved_path
                );
            }
            Err(e) => {
                println!(
                    "Failed to load model {} from {} (requested {}): {:?}",
                    key, resolved_path, path, e
                );
            }
        }
    } else if is_gltf {
        match load_gltf_animated_model(&resolved_path) {
            Ok(animated) => {
                let vertex_count = animated.mesh().vertices.len();
                playing_state.animated_models.insert(key.clone(), animated);
                playing_state
                    .asset_source_paths
                    .insert(key.clone(), path.clone());
                println!(
                    "Loaded animated GLTF model: {} ({} verts) from {}",
                    key, vertex_count, resolved_path
                );
            }
            Err(anim_err) => {
                // Weapon/viewmodel models (keys ending in "_model") get lighting baked
                // into vertex colors at load time so they have depth without a custom shader.
                let is_viewmodel = key.ends_with("_model");
                let static_result = if is_viewmodel {
                    load_gltf_static_mesh_lit(&resolved_path).await
                } else {
                    load_gltf_static_mesh(&resolved_path).await
                };
                match static_result {
                    Ok(mesh) => {
                        let vertex_count = mesh.vertices.len();
                        playing_state.models.insert(key.clone(), mesh);
                        playing_state
                            .asset_source_paths
                            .insert(key.clone(), path.clone());
                        println!(
                            "Loaded static GLTF model: {} ({} verts) from {}",
                            key, vertex_count, resolved_path
                        );
                    }
                    Err(static_err) => {
                        println!(
                            "Failed GLTF load {} from {} (requested {}). animated_err={:?} static_err={:?}",
                            key, resolved_path, path, anim_err, static_err
                        );
                    }
                }
            }
        }
    } else if is_sound {
        match std::fs::read(&resolved_path) {
            Ok(bytes) => {
                playing_state
                    .sound_samples
                    .insert(key.clone(), Arc::from(bytes.into_boxed_slice()));
                playing_state
                    .asset_source_paths
                    .insert(key.clone(), path.clone());
                println!("Loaded sound bytes: {} from {}", key, resolved_path);
            }
            Err(e) => {
                println!(
                    "Failed to load sound bytes {} from {} (requested {}): {:?}",
                    key, resolved_path, path, e
                );
            }
        }
    } else {
        match load_texture(&resolved_path).await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                playing_state.assets.insert(key.clone(), tex);
                playing_state
                    .asset_source_paths
                    .insert(key.clone(), path.clone());
                println!("Loaded texture: {} from {}", key, resolved_path);
            }
            Err(e) => {
                println!(
                    "Failed to load texture {} from {} (requested {}): {:?}",
                    key, resolved_path, path, e
                );
            }
        }
    }
}

fn resolve_runtime_asset_path(path: &str) -> String {
    if Path::new(path).exists() {
        return path.to_string();
    }
    if path.starts_with("assets/") {
        let client_path = format!("client/{}", path);
        if Path::new(&client_path).exists() {
            return client_path;
        }
        let server_path = format!("server/{}", path);
        if Path::new(&server_path).exists() {
            return server_path;
        }
    }
    path.to_string()
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
