use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedAssetEntry {
    key: String,
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedAssetManifest {
    server_addr: String,
    updated_at_unix_secs: u64,
    entries: Vec<CachedAssetEntry>,
}

pub fn load_cached_manifest(server_addr: &str) -> Vec<(String, String)> {
    let file_path = manifest_file_path(server_addr);
    let Ok(raw) = fs::read_to_string(&file_path) else {
        return Vec::new();
    };

    let Ok(manifest) = serde_json::from_str::<CachedAssetManifest>(&raw) else {
        return Vec::new();
    };

    if manifest.server_addr != server_addr {
        return Vec::new();
    }

    manifest
        .entries
        .into_iter()
        .map(|entry| (entry.key, entry.path))
        .collect()
}

pub fn save_cached_manifest(server_addr: &str, entries: &[(String, String)]) {
    let file_path = manifest_file_path(server_addr);
    if let Some(parent) = file_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            println!(
                "Failed to create asset cache directory {:?}: {:?}",
                parent, e
            );
            return;
        }
    }

    let manifest = CachedAssetManifest {
        server_addr: server_addr.to_string(),
        updated_at_unix_secs: platform::unix_time_ms() / 1000,
        entries: entries
            .iter()
            .map(|(key, path)| CachedAssetEntry {
                key: key.clone(),
                path: path.clone(),
            })
            .collect(),
    };

    match serde_json::to_string_pretty(&manifest) {
        Ok(json) => {
            if let Err(e) = fs::write(&file_path, json) {
                println!(
                    "Failed to write asset cache manifest {:?}: {:?}",
                    file_path, e
                );
            }
        }
        Err(e) => {
            println!(
                "Failed to serialize asset cache manifest for {}: {:?}",
                server_addr, e
            );
        }
    }
}

fn manifest_file_path(server_addr: &str) -> PathBuf {
    let mut path = PathBuf::from("cache/assets/manifests");
    path.push(format!("{}.json", sanitize_server_addr(server_addr)));
    path
}

fn sanitize_server_addr(server_addr: &str) -> String {
    let mut out = String::with_capacity(server_addr.len());
    for ch in server_addr.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unknown_server".to_string()
    } else {
        out
    }
}
