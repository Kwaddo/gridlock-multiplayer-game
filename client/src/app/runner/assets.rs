use client::screens::PlayingState;
use client::weapons::loadout::Loadout;

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
    // the default character too, the idle animation is just keyframes now and needs a body to play on
    let mut keys = vec!["fp_hands", "anim_rifle_idle", PlayingState::DEFAULT_CHARACTER];
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
        .map(|p| p.anim.skin_type)
        .chain(playing_state.practice_bots.iter().map(|b| b.player.anim.skin_type));
    keys.extend(skins.filter_map(|skin| skin.model_key()));
    // skin test: with dev mode unlocked load every character, otherwise only the default one is ever needed
    if playing_state.dev_mode.is_unlocked() {
        keys.extend(PlayingState::CHARACTER_ORDER);
    }
    keys
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
