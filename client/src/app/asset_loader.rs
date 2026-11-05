use client::screens::PlayingState;
use client::types::animated_model::{
    load_gltf_animated_model, load_gltf_animated_model_with_unskinned,
};
use macroquad::experimental::coroutines::{start_coroutine, Coroutine};
use macroquad::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use super::obj_loader::{
    load_gltf_static_mesh, load_gltf_static_mesh_lit, load_obj_mesh_with_materials,
};

/// How many assets [`Prefetcher`] downloads ahead of the one being loaded.
pub const PREFETCH_AHEAD: usize = 4;

thread_local! {
    /// What [`Prefetcher`] downloaded (or why it could not), by resolved path,
    /// until the loader takes it.
    static PREFETCHED: RefCell<HashMap<String, Result<Vec<u8>, String>>> = RefCell::new(HashMap::new());
}

/// Downloads assets in the background (macroquad coroutines), so the loading
/// screen keeps drawing, and a game hosted in this tab keeps answering
/// players, while a large model downloads. Every file the loader reads goes
/// through it except OBJs, whose loader fetches their materials itself (the
/// browser build has none).
#[derive(Default)]
pub struct Prefetcher {
    pending: HashMap<String, Coroutine<Result<Vec<u8>, String>>>,
}

impl Prefetcher {
    /// Start downloading `path` unless it is already on its way.
    pub fn start(&mut self, path: &str) {
        let resolved = resolve_runtime_asset_path(path);
        if !prefetchable(&resolved) || self.pending.contains_key(&resolved) {
            return;
        }
        let file = resolved.clone();
        let download = start_coroutine(async move { load_file(&file).await.map_err(|e| format!("{e:?}")) });
        self.pending.insert(resolved, download);
    }

    /// Whether `path` can be loaded now without waiting on the network.
    /// Starts its download if nothing has yet.
    pub fn ready(&mut self, path: &str) -> bool {
        let resolved = resolve_runtime_asset_path(path);
        if !prefetchable(&resolved) {
            return true;
        }
        self.start(path);
        let download = &self.pending[&resolved];
        if !download.is_done() {
            return false;
        }
        // A failure is handed over too: retrying inline would wait on the
        // network inside a frame (a black flash on the loading screen).
        if let Some(result) = download.retrieve() {
            PREFETCHED.with(|p| p.borrow_mut().insert(resolved.clone(), result));
        }
        self.pending.remove(&resolved);
        true
    }

    /// Forget downloads in flight and anything not yet taken.
    pub fn clear(&mut self) {
        self.pending.clear();
        PREFETCHED.with(|p| p.borrow_mut().clear());
    }
}

fn prefetchable(resolved_path: &str) -> bool {
    !resolved_path.to_lowercase().ends_with(".obj")
}

/// How much of `path` has downloaded, as (received, total) bytes, while the
/// browser build fetches it from the models host. `None` when nothing is
/// known (native builds read from disk).
pub fn download_progress(path: &str) -> Option<(f64, f64)> {
    #[cfg(target_arch = "wasm32")]
    {
        let resolved = resolve_runtime_asset_path(path);
        let (received, total) = unsafe {
            (
                web::gridlock_download_received(resolved.as_ptr(), resolved.len() as u32),
                web::gridlock_download_total(resolved.as_ptr(), resolved.len() as u32),
            )
        };
        (received >= 0.0 && total > 0.0).then_some((received, total))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = path;
        None
    }
}

/// Every asset in the game's manifest, `assets/manifest.json`: one list for all
/// games, kept with the assets themselves (`{ world: { key: path } }`).
pub async fn load_manifest() -> Vec<(String, String)> {
    let path = resolve_asset_path("assets/manifest.json");
    let manifest: serde_json::Value = match load_file(&path).await {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            warn!("Bad asset manifest {}: {}", path, e);
            serde_json::Value::Null
        }),
        Err(e) => {
            warn!("No asset manifest at {}: {:?}", path, e);
            serde_json::Value::Null
        }
    };
    manifest
        .as_object()
        .into_iter()
        .flat_map(|worlds| worlds.values())
        .filter_map(|world| world.as_object())
        .flat_map(|world| world.iter())
        .filter_map(|(key, path)| Some((key.clone(), path.as_str()?.to_string())))
        .collect()
}

/// Start downloading `path` into the browser's cache in the background, for a
/// game the player has not joined yet (models only; a no-op natively).
pub fn preload(path: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let resolved = resolve_runtime_asset_path(path);
        unsafe { web::gridlock_preload(resolved.as_ptr(), resolved.len() as u32) };
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = path;
}

/// Models preloaded so far and in all, while there are any.
pub fn preload_status() -> Option<(u32, u32)> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut status = [0u32; 2];
        unsafe { web::gridlock_preload_status(status.as_mut_ptr()) };
        (status[1] > 0).then_some((status[0], status[1]))
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

#[cfg(target_arch = "wasm32")]
mod web {
    extern "C" {
        pub fn gridlock_download_received(ptr: *const u8, len: u32) -> f64;
        pub fn gridlock_download_total(ptr: *const u8, len: u32) -> f64;
        pub fn gridlock_preload(ptr: *const u8, len: u32);
        pub fn gridlock_preload_status(out: *mut u32);
    }
}

/// The file's bytes: downloaded already by [`Prefetcher`], or read now.
async fn read_bytes(resolved_path: &str) -> Result<Vec<u8>, String> {
    if let Some(result) = PREFETCHED.with(|p| p.borrow_mut().remove(resolved_path)) {
        return result;
    }
    load_file(resolved_path).await.map_err(|e| format!("{e:?}"))
}

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
        let texture = read_bytes(&resolved_path).await.and_then(|bytes| {
            Image::from_file_with_format(&bytes, None)
                .map(|image| Texture2D::from_image(&image))
                .map_err(|e| format!("{e:?}"))
        });
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

/// Whether the asset `key` is already loaded from `path`.
pub fn asset_loaded(playing_state: &PlayingState, key: &str, path: &str) -> bool {
    let lower = resolve_runtime_asset_path(path).to_lowercase();
    let is_obj = lower.ends_with(".obj");
    let is_gltf = lower.ends_with(".glb") || lower.ends_with(".gltf");
    let is_sound = lower.ends_with(".ogg") || lower.ends_with(".wav") || lower.ends_with(".mp3");
    is_asset_cached(playing_state, key, path, is_obj, is_gltf, is_sound)
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
        playing_state.models.contains_key(key) || playing_state.animated_models.contains_key(key)
    } else if is_sound {
        playing_state.audio.has(key)
    } else {
        playing_state.assets.contains_key(key)
    }
}
