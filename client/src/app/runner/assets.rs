use std::path::Path;

use client::screens::PlayingState;
use client::weapons::loadout::Loadout;

// asset quality selector used when resolving which version of a model to
// request from the server.  the value is passed down to
// `resolve_asset_path_for_quality`, which currently just uses it when
// choosing between `Compressed`/`Full` folders; defaults to `High`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(super) enum ModelQuality {
    Low,
    Medium,
    High,
}

impl Default for ModelQuality {
    fn default() -> Self {
        ModelQuality::High
    }
}

pub(super) fn set_cwd_to_assets_root() {
    // The browser build fetches assets relative to the page instead.
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        while let Some(d) = dir {
            if d.join("assets").is_dir() {
                let _ = std::env::set_current_dir(&d);
                return;
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }
}

/// The models a player needs to spawn with `loadout`: its weapons and their
/// hands, the knife's hands, and the idle animation every other player is
/// drawn with until their own animations arrive.
pub(super) fn spawn_assets(loadout: &Loadout) -> Vec<&'static str> {
    let mut keys = vec!["fp_hands", "anim_rifle_idle"];
    for id in loadout.slots {
        keys.extend(id.stats().viewmodel_model);
        keys.push(PlayingState::preferred_hands_key_for_weapon(id));
    }
    keys
}

/// What to load next while playing: the weapons the player holds or picked in
/// the lobby, then the animations other players are showing right now.
pub(super) fn wanted_assets(playing_state: &PlayingState, loadout: &Loadout) -> Vec<&'static str> {
    let held = playing_state.weapons.weapons.iter().map(|w| w.id);
    let mut keys = Vec::new();
    for id in held.chain(loadout.slots) {
        keys.extend(id.stats().viewmodel_model);
        keys.push(PlayingState::preferred_hands_key_for_weapon(id));
    }
    let skins = playing_state.other_players
        .iter()
        .map(|p| p.skin_type)
        .chain(playing_state.practice_bots.iter().map(|b| b.player.skin_type));
    keys.extend(skins.filter_map(|skin| skin.model_key()));
    keys
}

pub(super) fn resolve_asset_entries_for_quality(
    entries: &[(String, String)],
    model_quality: ModelQuality
) -> Vec<(String, String)> {
    // the `model_quality` parameter is currently unused; kept so we can
    // support different heuristics later (low/medium/high).
    let _ = model_quality;
    entries
        .iter()
        .map(|(key, path)| { (key.clone(), resolve_asset_path_for_quality(path, model_quality)) })
        .collect()
}

fn resolve_asset_path_for_quality(path: &str, _model_quality: ModelQuality) -> String {
    // avoid allocating a normalized copy unless we know it's a player GLB file
    if !path.ends_with(".glb") || !path.starts_with("assets/models/player/") {
        return path.to_string();
    }
    let normalized = if path.contains('\\') { path.replace('\\', "/") } else { path.to_string() };

    // The browser cannot probe which variants exist; the web build ships the
    // compressed set.
    #[cfg(target_arch = "wasm32")]
    if let Some(file_name) = normalized.rsplit('/').next() {
        return format!("assets/models/player/Compressed/{}", file_name);
    }

    if Path::new(&normalized).exists() {
        return normalized;
    }

    let Some(file_name) = Path::new(&normalized)
        .file_name()
        .and_then(|s| s.to_str()) else {
        return path.to_string();
    };

    let compressed_path = format!("assets/models/player/Compressed/{}", file_name);
    if Path::new(&compressed_path).exists() {
        return compressed_path;
    }

    let full_path = format!("assets/models/player/Full/{}", file_name);
    if Path::new(&full_path).exists() {
        return full_path;
    }

    let flat_path = format!("assets/models/player/{}", file_name);
    if Path::new(&flat_path).exists() {
        return flat_path;
    }

    let compressed_idle = "assets/models/player/Compressed/Rifle_Idle.glb";
    if Path::new(compressed_idle).exists() {
        return compressed_idle.to_string();
    }

    let full_idle = "assets/models/player/Full/Rifle_Idle.glb";
    if Path::new(full_idle).exists() {
        return full_idle.to_string();
    }

    let idle = "assets/models/player/Rifle_Idle.glb";
    if Path::new(idle).exists() {
        return idle.to_string();
    }

    path.to_string()
}

/// Viewmodel assets (first-person hands and weapons) the game needs that are
/// not loaded yet.
pub(super) fn missing_viewmodel_assets(playing_state: &PlayingState) -> Vec<(String, String)> {
    let required = [
        ("fp_hands", "assets/models/hand/fps_animated_carbine.glb"),
        ("fp_hands_pistol", "assets/models/hand/fps_pistol_animated.glb"),

        ("fp_hands_smg", "assets/models/hand/fps_animated_smg.glb"),
        ("fp_hands_sniper", "assets/models/hand/sniper_animated.glb"),
        ("fp_hands_minigun", "assets/models/hand/minigun_animated.glb"),
        ("fp_hands_sawnoff", "assets/models/hand/sawnoff_animated.glb"),
        ("m1911_model", "assets/models/weapons/Pistols/m1911.glb"),
        ("deagle_model", "assets/models/weapons/Pistols/deagle.glb"),
        ("mp5_model", "assets/models/weapons/SMGs/mp5.glb"),
        ("spas_model", "assets/models/weapons/Shotguns/spas.glb"),
        ("m16_model", "assets/models/weapons/Assault Rifles/m16.glb"),
        ("bolt_action_model", "assets/models/weapons/Snipers/bolt_action.glb"),
        ("minigun_model", "assets/models/weapons/Other/minigun.glb"),
    ];

    required
        .into_iter()
        .filter(|(key, _)| {
            !playing_state.models.contains_key(*key) &&
                !playing_state.animated_models.contains_key(*key)
        })
        .map(|(key, path)| (key.to_string(), path.to_string()))
        .collect()
}
