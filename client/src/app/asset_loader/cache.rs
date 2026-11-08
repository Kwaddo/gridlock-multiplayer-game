use client::screens::PlayingState;

use super::paths::resolve_runtime_asset_path;

/// Whether the asset `key` is already loaded from `path`.
pub fn asset_loaded(playing_state: &PlayingState, key: &str, path: &str) -> bool {
    let lower = resolve_runtime_asset_path(path).to_lowercase();
    let is_obj = lower.ends_with(".obj");
    let is_gltf = lower.ends_with(".glb") || lower.ends_with(".gltf");
    let is_sound = lower.ends_with(".ogg") || lower.ends_with(".wav") || lower.ends_with(".mp3");
    is_asset_cached(playing_state, key, path, is_obj, is_gltf, is_sound)
}

pub(super) fn is_asset_cached(
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
