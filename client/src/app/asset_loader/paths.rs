#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

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

pub(super) fn resolve_runtime_asset_path(path: &str) -> String {
    resolve_asset_path(path)
}
