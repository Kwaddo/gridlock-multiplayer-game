use client::screens::PlayingState;

/// Whether the asset `key` is already loaded from `path`.
pub fn asset_loaded(playing_state: &PlayingState, key: &str, path: &str) -> bool {
    // Resolving only puts a directory in front of the path, so the raw path
    // has the same extension: no file probing or allocation per check.
    let is_obj = has_ext(path, ".obj");
    let is_gltf = has_ext(path, ".glb") || has_ext(path, ".gltf");
    let is_sound = has_ext(path, ".ogg") || has_ext(path, ".wav") || has_ext(path, ".mp3");
    is_asset_cached(playing_state, key, path, is_obj, is_gltf, is_sound)
}

/// Whether `path` ends in `ext` (given in lowercase), in any case.
pub(super) fn has_ext(path: &str, ext: &str) -> bool {
    path.len() >= ext.len()
        && path.as_bytes()[path.len() - ext.len()..].eq_ignore_ascii_case(ext.as_bytes())
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
        playing_state.models.contains_key(key)
            || playing_state.animated_models.contains_key(key)
            || playing_state.characters.contains_key(key) // a character on its own
            || playing_state.clips.contains_key(key) // a bones only animation, even before a character is in to wear it
    } else if is_sound {
        playing_state.audio.has(key)
    } else {
        playing_state.assets.contains_key(key)
    }
}
