use macroquad::prelude::*;

use super::paths::resolve_asset_path;

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
