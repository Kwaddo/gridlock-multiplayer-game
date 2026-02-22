use client::screens::PlayingState;
use client::types::animated_model::{
    load_gltf_animated_model, load_gltf_animated_model_with_unskinned,
};
use macroquad::prelude::*;
use std::path::Path;
use std::sync::Arc;

use super::obj_loader::{
    load_gltf_static_mesh, load_gltf_static_mesh_lit, load_obj_mesh_with_materials,
};

pub async fn load_asset_entry(key: &str, path: &str, playing_state: &mut PlayingState) {
    // try to resolve the path relative to client/ or server/ if needed; the
    // original manifest entry is kept for caching purposes so we compare on
    // the unmodified string (matches server-side path).
    let resolved_path = resolve_runtime_asset_path(path);

    // lowercasing the path is slightly expensive; do it once on the *resolved* path
    let lower = resolved_path.to_lowercase();
    let is_obj = lower.ends_with(".obj");
    let is_gltf = lower.ends_with(".glb") || lower.ends_with(".gltf");
    let is_sound = lower.ends_with(".ogg") || lower.ends_with(".wav") || lower.ends_with(".mp3");

    if is_asset_cached(playing_state, key, path, is_obj, is_gltf, is_sound) {
        return;
    }

    // clone the strings only when we actually store them
    let key_string = key.to_string();
    let path_string = path.to_string();

    if is_obj {
        match load_obj_mesh_with_materials(&resolved_path).await {
            Ok(mesh) => {
                let vertex_count = mesh.vertices.len();
                // ensure vector storage
                playing_state
                    .models
                    .entry(key_string.clone())
                    .or_default()
                    .push(mesh);
                playing_state
                    .asset_source_paths
                    .insert(key_string.clone(), path_string.clone());
                println!(
                    "Loaded model: {} ({} verts) from {}",
                    key_string, vertex_count, path_string
                );
            }
            Err(e) => {
                println!(
                    "Failed to load model {} from {}: {:?}",
                    key_string, path_string, e
                );
            }
        }
    } else if is_gltf {
        // try animated loader first, but if the returned mesh has no texture we
        // fall back to static loader. some static weapon GLBs are exported with
        // a skin/animation placeholder which confuses the animated loader.
        let keep_unskinned_primitives = key == "fp_hands_knife";
        let animated_result = if keep_unskinned_primitives {
            load_gltf_animated_model_with_unskinned(&resolved_path, true)
        } else {
            load_gltf_animated_model(&resolved_path)
        };
        match animated_result {
            Ok(animated) => {
                if animated.mesh().texture.is_none() {
                    // animated model produced but no texture; try static loader
                    match load_gltf_static_mesh(&resolved_path).await {
                        Ok(mut meshes) => {
                            let all_untextured = meshes.iter().all(|m| m.texture.is_none());
                            if all_untextured {
                                if let Ok(lit) = load_gltf_static_mesh_lit(&resolved_path).await {
                                    meshes = lit;
                                }
                            }

                            let total_verts: usize = meshes.iter().map(|m| m.vertices.len()).sum();
                            let entry = playing_state.models.entry(key_string.clone()).or_default();
                            for mesh in meshes {
                                entry.push(mesh);
                            }
                            playing_state
                                .asset_source_paths
                                .insert(key_string.clone(), path_string.clone());
                            println!(
                                "Loaded static GLTF model (anim fallback): {} ({} verts across {} sub-meshes) from {}",
                                key_string,
                                total_verts,
                                entry.len(),
                                path_string
                            );
                        }
                        Err(static_err) => {
                            // static path failed; keep the textureless animated mesh
                            let vertex_count = animated.mesh().vertices.len();
                            playing_state
                                .animated_models
                                .insert(key_string.clone(), animated);
                            playing_state
                                .asset_source_paths
                                .insert(key_string.clone(), path_string.clone());
                            println!(
                                "Loaded animated GLTF model (no texture, static fallback failed): {} ({} verts) from {}",
                                key_string, vertex_count, path_string
                            );
                        }
                    }
                } else {
                    let vertex_count = animated.mesh().vertices.len();
                    playing_state
                        .animated_models
                        .insert(key_string.clone(), animated);
                    playing_state
                        .asset_source_paths
                        .insert(key_string.clone(), path_string.clone());
                    println!(
                        "Loaded animated GLTF model: {} ({} verts) from {}",
                        key_string, vertex_count, path_string
                    );
                }
            }
            Err(anim_err) => {
                match load_gltf_static_mesh(&resolved_path).await {
                    Ok(mut meshes) => {
                        // if no primitive had a texture, try the lit variant to give
                        // the user some built‑in shading
                        let all_untextured = meshes.iter().all(|m| m.texture.is_none());
                        if all_untextured {
                            if let Ok(lit) = load_gltf_static_mesh_lit(&resolved_path).await {
                                meshes = lit;
                            }
                        }

                        let total_verts: usize = meshes.iter().map(|m| m.vertices.len()).sum();
                        let entry = playing_state.models.entry(key_string.clone()).or_default();
                        for mesh in meshes {
                            entry.push(mesh);
                        }
                        playing_state
                            .asset_source_paths
                            .insert(key_string.clone(), path_string.clone());
                        println!(
                            "Loaded static GLTF model: {} ({} verts across {} sub-meshes) from {}",
                            key_string,
                            total_verts,
                            entry.len(),
                            path_string
                        );
                    }
                    Err(static_err) => {
                        println!(
                            "Failed GLTF load {} from {}. animated_err={:?} static_err={:?}",
                            key_string, path_string, anim_err, static_err
                        );
                    }
                }
            }
        }
    } else if is_sound {
        match std::fs::read(path) {
            Ok(bytes) => {
                playing_state
                    .sound_samples
                    .insert(key_string.clone(), Arc::from(bytes.into_boxed_slice()));
                playing_state
                    .asset_source_paths
                    .insert(key_string.clone(), path_string.clone());
                println!("Loaded sound bytes: {} from {}", key_string, path_string);
            }
            Err(e) => {
                println!(
                    "Failed to load sound bytes {} from {}: {:?}",
                    key_string, path_string, e
                );
            }
        }
    } else {
        match load_texture(path).await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                playing_state.assets.insert(key_string.clone(), tex);
                playing_state
                    .asset_source_paths
                    .insert(key_string.clone(), path_string.clone());
                println!("Loaded texture: {} from {}", key_string, path_string);
            }
            Err(e) => {
                println!(
                    "Failed to load texture {} from {}: {:?}",
                    key_string, path_string, e
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
