use client::screens::PlayingState;
use client::types::animated_model::{
    load_gltf_animated_model, load_gltf_animated_model_with_unskinned,
};
use macroquad::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

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
        let keep_unskinned_primitives = key == "fp_hands_sawnoff"
            || key == "fp_hands_minigun";
        // Read once; works from disk natively and over HTTP in the browser.
        let bytes = match load_file(&resolved_path).await {
            Ok(bytes) => bytes,
            Err(e) => {
                println!("Failed to read {} from {}: {:?}", key_string, path_string, e);
                return;
            }
        };
        let animated_result = if keep_unskinned_primitives {
            load_gltf_animated_model_with_unskinned(&resolved_path, &bytes, true)
        } else {
            load_gltf_animated_model(&resolved_path, &bytes)
        };
        match animated_result {
            Ok(animated) => {
                if !animated.has_any_texture() {
                    // animated model produced but no texture; try static loader
                    match load_gltf_static_mesh(&resolved_path, &bytes).await {
                        Ok(mut meshes) => {
                            let all_untextured = meshes.iter().all(|m| m.texture.is_none());
                            if all_untextured {
                                if let Ok(lit) = load_gltf_static_mesh_lit(&resolved_path, &bytes).await {
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
                            let vertex_count = animated.total_vertex_count();
                            playing_state
                                .animated_models
                                .insert(key_string.clone(), animated);
                            playing_state
                                .asset_source_paths
                                .insert(key_string.clone(), path_string.clone());
                            println!(
                                "Loaded animated GLTF model (no texture, static fallback failed): {} ({} verts) from {}. Error is {:?}",
                                key_string, vertex_count, path_string, static_err
                            );
                        }
                    }
                } else {
                    let vertex_count = animated.total_vertex_count();
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
                match load_gltf_static_mesh(&resolved_path, &bytes).await {
                    Ok(mut meshes) => {
                        // if no primitive had a texture, try the lit variant to give
                        // the user some built‑in shading
                        let all_untextured = meshes.iter().all(|m| m.texture.is_none());
                        if all_untextured {
                            if let Ok(lit) = load_gltf_static_mesh_lit(&resolved_path, &bytes).await {
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
        match load_file(&resolved_path).await.map_err(|e| format!("{e:?}")) {
            Ok(bytes) => {
                if let Err(e) = playing_state.audio.insert(key_string.clone(), bytes).await {
                    println!("Failed to decode sound {} from {}: {}", key_string, path_string, e);
                    return;
                }
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
        match load_texture(&resolved_path).await {
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

/// In the browser, paths are URLs relative to the page; nothing to probe.
#[cfg(target_arch = "wasm32")]
pub fn resolve_asset_path(path: &str) -> String {
    path.to_string()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn resolve_asset_path(path: &str) -> String {
    // 1. As-is (works when CWD is the right place, e.g. `cargo run`)
    if Path::new(path).exists() {
        return path.to_string();
    }

    // 2. Relative to the executable's directory — handles double-clicking the binary
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let candidate = exe_dir.join(path);
            if candidate.exists() {
                return candidate.to_string_lossy().into_owned();
            }
            // Also try exe_dir/assets/... when path is bare (e.g. "fonts/Pixel Game.otf")
            if !path.starts_with("assets/") {
                let candidate = exe_dir.join("assets").join(path);
                if candidate.exists() {
                    return candidate.to_string_lossy().into_owned();
                }
            }
        }
    }

    // 3. client/ or server/ prefix — works when CWD is the workspace root
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

fn resolve_runtime_asset_path(path: &str) -> String {
    resolve_asset_path(path)
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
        if playing_state.audio.has(key) {
            println!("Asset cache hit: {} ({})", key, path);
            return true;
        }
    } else if playing_state.assets.contains_key(key) {
        println!("Asset cache hit: {} ({})", key, path);
        return true;
    }

    false
}
