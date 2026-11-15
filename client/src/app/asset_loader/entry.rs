use client::screens::PlayingState;
use client::types::animated_model::{
    load_gltf_animated_model, load_gltf_animated_model_with_unskinned,
};
use macroquad::prelude::*;

use super::cache::is_asset_cached;
use super::paths::resolve_runtime_asset_path;
use super::prefetch::{read_bytes, take_decoded_image};
use crate::app::obj_loader::{
    load_gltf_static_mesh, load_obj_mesh_with_materials,
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
        println!("Asset cache hit: {} ({})", key, path);
        return;
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
        // try animated loader first, but if the returned mesh has no texture we
        // fall back to static loader. some static weapon GLBs are exported with
        // a skin/animation placeholder which confuses the animated loader.
        let keep_unskinned_primitives = key == "fp_hands_sawnoff"
            || key == "fp_hands_minigun";
        // Read once; works from disk natively and over HTTP in the browser.
        let bytes = match read_bytes(&resolved_path).await {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!("Failed to read {} from {}: {}", key_string, path_string, e);
                return;
            }
        };
        // a character (mesh + skeleton) or a bones only animation, see screens/playing/characters.rs
        // before this every anim_ file was a whole character with one animation in it
        if key.starts_with("character_") {
            match load_gltf_animated_model(&resolved_path, &bytes) {
                Ok(model) => {
                    playing_state.asset_source_paths.insert(key_string.clone(), path_string.clone());
                    println!("Loaded character: {} from {}", key_string, path_string);
                    playing_state.add_character(key_string, model);
                }
                Err(e) => warn!("Failed to load character {} from {}: {}", key_string, path_string, e),
            }
            return;
        }
        if key.starts_with("anim_") && path.contains("/animations/") {
            match client::types::animated_model::load_clip(&resolved_path, &bytes) {
                Ok(clip) => {
                    playing_state.asset_source_paths.insert(key_string.clone(), path_string.clone());
                    println!("Loaded animation: {} ({:.2}s) from {}", key_string, clip.duration(), path_string);
                    playing_state.add_clip(key_string, clip);
                }
                Err(e) => warn!("Failed to load animation {} from {}: {}", key_string, path_string, e),
            }
            return;
        }
        let load_animated = || if keep_unskinned_primitives {
            load_gltf_animated_model_with_unskinned(&resolved_path, &bytes, true)
        } else {
            load_gltf_animated_model(&resolved_path, &bytes)
        };
        // The weapon GLBs have no animation (some no skin either), so the
        // animated loader can only fail on them, after a whole import. Skip
        // it then; its error is only wanted if the static loader fails too,
        // so it runs then (`None` here).
        let animated_result = if animated_load_cannot_succeed(&bytes) {
            Err(None)
        } else {
            load_animated().map_err(Some)
        };
        match animated_result {
            Ok(animated) => {
                if !animated.has_any_texture() {
                    // animated model produced but no texture; try static loader
                    match load_gltf_static_mesh(&resolved_path, &bytes).await {
                        Ok(meshes) => {
                            let total_verts: usize = meshes.iter().map(|m| m.vertices.len()).sum();
                            // a key loaded again (another path for it, or a
                            // new session) replaces its meshes: appending
                            // stacked two guns into one and grew every session
                            let entry = playing_state.models.entry(key_string.clone()).or_default();
                            entry.clear();
                            entry.extend(meshes);
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
                            animated.prepare_gpu();
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
                    animated.prepare_gpu();
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
                    Ok(meshes) => {
                        // if no primitive had a texture, the loader bakes in
                        // lighting to give the user some built‑in shading
                        let total_verts: usize = meshes.iter().map(|m| m.vertices.len()).sum();
                        // a key loaded again (another path for it, or a new
                        // session) replaces its meshes: appending stacked two
                        // guns into one and grew every session
                        let entry = playing_state.models.entry(key_string.clone()).or_default();
                        entry.clear();
                        entry.extend(meshes);
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
                        // skipped above: the animated loader fails for sure, run it for its error
                        let anim_err = anim_err.unwrap_or_else(|| match load_animated() {
                            Err(e) => e,
                            Ok(_) => String::from("animated loader skipped"),
                        });
                        warn!(
                            "Failed GLTF load {} from {}. animated_err={:?} static_err={:?}",
                            key_string, path_string, anim_err, static_err
                        );
                    }
                }
            }
        }
    } else if is_sound {
        match read_bytes(&resolved_path).await {
            Ok(bytes) => {
                if let Err(e) = playing_state.audio.insert(key_string.clone(), bytes).await {
                    warn!("Failed to decode sound {} from {}: {}", key_string, path_string, e);
                    return;
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
}

/// Whether the animated loader is sure to fail on this glTF: it needs a skin
/// (`no skin found`) and an animation (`no usable animation clip`). Reads only
/// the JSON part, without importing the file. `false` when unsure (the JSON
/// does not parse), so the animated loader still gets its try.
fn animated_load_cannot_succeed(bytes: &[u8]) -> bool {
    use serde::de::IgnoredAny;

    #[derive(serde::Deserialize)]
    struct SkinsAndAnimations {
        #[serde(default)]
        skins: Vec<IgnoredAny>,
        #[serde(default)]
        animations: Vec<IgnoredAny>,
    }

    // as gltf::Gltf::from_slice tells a binary .glb from a JSON .gltf
    let json = if bytes.starts_with(b"glTF") {
        match gltf::Glb::from_slice(bytes) {
            Ok(glb) => glb.json,
            Err(_) => return false,
        }
    } else {
        std::borrow::Cow::Borrowed(bytes)
    };
    match serde_json::from_slice::<SkinsAndAnimations>(&json) {
        Ok(doc) => doc.skins.is_empty() || doc.animations.is_empty(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::animated_load_cannot_succeed;

    /// A .glb holding `json` and no binary chunk.
    fn glb(json: &str) -> Vec<u8> {
        let mut json = json.as_bytes().to_vec();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }
        let mut out = Vec::new();
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&(12 + 8 + json.len() as u32).to_le_bytes());
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&json);
        out
    }

    #[test]
    fn skips_animated_loader_only_without_skin_or_animation() {
        let asset = r#""asset":{"version":"2.0"}"#;
        let skin = r#""skins":[{"joints":[0]}]"#;
        let anim = r#""animations":[{"channels":[],"samplers":[]}]"#;
        assert!(animated_load_cannot_succeed(&glb(&format!("{{{asset}}}"))));
        assert!(animated_load_cannot_succeed(&glb(&format!("{{{asset},{skin}}}"))));
        assert!(animated_load_cannot_succeed(&glb(&format!("{{{asset},{anim}}}"))));
        assert!(animated_load_cannot_succeed(&glb(&format!(r#"{{{asset},"skins":[],{anim}}}"#))));
        assert!(!animated_load_cannot_succeed(&glb(&format!("{{{asset},{skin},{anim}}}"))));
        // a JSON .gltf
        assert!(animated_load_cannot_succeed(format!("{{{asset},{skin}}}").as_bytes()));
        assert!(!animated_load_cannot_succeed(format!("{{{asset},{skin},{anim}}}").as_bytes()));
        // unsure: let the animated loader try
        assert!(!animated_load_cannot_succeed(b"glTF garbage"));
        assert!(!animated_load_cannot_succeed(b"not json"));
        assert!(!animated_load_cannot_succeed(&glb(&format!(r#"{{{asset},"skins":7,{anim}}}"#))));
    }

    /// The game's own GLBs (`client/assets` is not in git; point
    /// `GRIDLOCK_ASSETS_DIR` at a copy, or it looks in `assets`): every file
    /// the animated loader is skipped for, it fails on.
    #[test]
    fn skipped_glbs_fail_the_animated_loader() {
        use client::types::animated_model::{
            load_gltf_animated_model, load_gltf_animated_model_with_unskinned,
        };
        let root = std::env::var("GRIDLOCK_ASSETS_DIR").unwrap_or_else(|_| "assets".to_string());
        let mut dirs = vec![std::path::PathBuf::from(root).join("models")];
        let mut checked = 0;
        while let Some(dir) = dirs.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for path in entries.flatten().map(|e| e.path()) {
                if path.is_dir() {
                    dirs.push(path);
                    continue;
                }
                if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("glb")) {
                    continue;
                }
                let bytes = std::fs::read(&path).unwrap();
                if !animated_load_cannot_succeed(&bytes) {
                    continue;
                }
                let name = path.to_string_lossy();
                assert!(load_gltf_animated_model(&name, &bytes).is_err(), "{name}");
                assert!(load_gltf_animated_model_with_unskinned(&name, &bytes, true).is_err(), "{name}");
                checked += 1;
            }
        }
        println!("{checked} GLBs skip the animated loader");
    }
}
