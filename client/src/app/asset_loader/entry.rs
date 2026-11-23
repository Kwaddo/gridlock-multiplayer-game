use client::screens::PlayingState;
use macroquad::prelude::*;

use super::cache::is_asset_cached;
use super::glb_job::GlbJob;
use super::paths::resolve_runtime_asset_path;
use super::prefetch::{read_bytes, take_decoded_image};
use crate::app::obj_loader::load_obj_mesh_with_materials;

/// Load the asset `key` from `path`, or (a GLB model or character) start
/// loading it: the job returned does the rest a step at a time.
pub async fn begin_asset_entry(key: &str, path: &str, playing_state: &mut PlayingState) -> Option<GlbJob> {
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
        println!("Asset cache hit: {} ({})", key, path);
        return None;
    }

    // clone the strings only when we actually store them
    let key_string = key.to_string();
    let path_string = path.to_string();

    if is_obj {
        match load_obj_mesh_with_materials(&resolved_path).await {
            Ok(mesh) => {
                let vertex_count = mesh.vertices.len();
                // ensure vector storage, big meshes split so macroquad draws every triangle
                playing_state
                    .models
                    .entry(key_string.clone())
                    .or_default()
                    .extend(client::types::mesh_split::split_mesh(mesh));
                playing_state
                    .asset_source_paths
                    .insert(key_string.clone(), path_string.clone());
                println!(
                    "Loaded model: {} ({} verts) from {}",
                    key_string, vertex_count, path_string
                );
            }
            Err(e) => {
                warn!(
                    "Failed to load model {} from {}: {:?}",
                    key_string, path_string, e
                );
            }
        }
    } else if is_gltf {
        // Read once; works from disk natively and over HTTP in the browser.
        let bytes = match read_bytes(&resolved_path).await {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!("Failed to read {} from {}: {}", key_string, path_string, e);
                return None;
            }
        };
        // a character (mesh + skeleton) or a bones only animation, see screens/playing/characters.rs
        // before this every anim_ file was a whole character with one animation in it
        if key.starts_with("anim_") && path.contains("/animations/") {
            match client::types::animated_model::load_clip(&resolved_path, &bytes) {
                Ok(clip) => {
                    playing_state.asset_source_paths.insert(key_string.clone(), path_string.clone());
                    println!("Loaded animation: {} ({:.2}s) from {}", key_string, clip.duration(), path_string);
                    playing_state.add_clip(key_string, clip);
                }
                Err(e) => warn!("Failed to load animation {} from {}: {}", key_string, path_string, e),
            }
            return None;
        }
        // a character or another model: a step at a time
        return GlbJob::start(key, path, &resolved_path, &bytes);
    } else if is_sound {
        match read_bytes(&resolved_path).await {
            Ok(bytes) => {
                if let Err(e) = playing_state.audio.insert(key_string.clone(), bytes).await {
                    warn!("Failed to decode sound {} from {}: {}", key_string, path_string, e);
                    return None;
                }
                playing_state
                    .asset_source_paths
                    .insert(key_string.clone(), path_string.clone());
                println!("Loaded sound bytes: {} from {}", key_string, path_string);
            }
            Err(e) => {
                warn!(
                    "Failed to load sound bytes {} from {}: {:?}",
                    key_string, path_string, e
                );
            }
        }
    } else {
        // Decoded from bytes so a prefetched file is used without waiting.
        // A worker thread may have decoded it already (native builds).
        let bytes = read_bytes(&resolved_path).await;
        let texture = match take_decoded_image(&resolved_path) {
            Some(image) => Ok(Texture2D::from_image(&image)),
            None => bytes.and_then(|bytes| {
                Image::from_file_with_format(&bytes, None)
                    .map(|image| Texture2D::from_image(&image))
                    .map_err(|e| format!("{e:?}"))
            }),
        };
        match texture {
            Ok(tex) => {
                tex.set_filter(FilterMode::Nearest);
                playing_state.assets.insert(key_string.clone(), tex);
                playing_state
                    .asset_source_paths
                    .insert(key_string.clone(), path_string.clone());
                println!("Loaded texture: {} from {}", key_string, path_string);
            }
            Err(e) => {
                warn!(
                    "Failed to load texture {} from {}: {:?}",
                    key_string, path_string, e
                );
            }
        }
    }
    None
}
