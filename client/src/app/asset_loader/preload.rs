#[cfg(target_arch = "wasm32")]
use super::paths::resolve_runtime_asset_path;

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
