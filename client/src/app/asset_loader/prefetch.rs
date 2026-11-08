use macroquad::experimental::coroutines::{start_coroutine, Coroutine};
use macroquad::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

use super::cache::has_ext;

thread_local! {
    /// What [`Prefetcher`] downloaded (or why it could not), by resolved path,
    /// until the loader takes it.
    pub(super) static PREFETCHED: RefCell<HashMap<String, Result<Vec<u8>, String>>> = RefCell::new(HashMap::new());
}

/// Downloads assets in the background (macroquad coroutines), so the loading
/// screen keeps drawing, and a game hosted in this tab keeps answering
/// players, while a large model downloads. Every file the loader reads goes
/// through it except OBJs, whose loader fetches their materials itself (the
/// browser build has none).
#[derive(Default)]
pub(super) struct Prefetcher {
    pending: HashMap<String, Coroutine<Result<Vec<u8>, String>>>,
}

impl Prefetcher {
    /// Start downloading `resolved` (a path already resolved) unless it is
    /// already on its way.
    pub fn start(&mut self, resolved: &str) {
        if !prefetchable(resolved) || self.pending.contains_key(resolved) {
            return;
        }
        let file = resolved.to_string();
        let download = start_coroutine(async move { load_file(&file).await.map_err(|e| format!("{e:?}")) });
        self.pending.insert(resolved.to_string(), download);
    }

    /// Whether `resolved` (a path already resolved) can be loaded now without
    /// waiting on the network. Starts its download if nothing has yet.
    pub fn ready(&mut self, resolved: &str) -> bool {
        if !prefetchable(resolved) {
            return true;
        }
        self.start(resolved);
        let download = &self.pending[resolved];
        if !download.is_done() {
            return false;
        }
        // A failure is handed over too: retrying inline would wait on the
        // network inside a frame (a black flash on the loading screen).
        if let Some(result) = download.retrieve() {
            PREFETCHED.with(|p| p.borrow_mut().insert(resolved.to_string(), result));
        }
        self.pending.remove(resolved);
        true
    }

}

fn prefetchable(resolved_path: &str) -> bool {
    !has_ext(resolved_path, ".obj")
}

/// The file's bytes: downloaded already by [`Prefetcher`], or read now.
pub(super) async fn read_bytes(resolved_path: &str) -> Result<Vec<u8>, String> {
    if let Some(result) = PREFETCHED.with(|p| p.borrow_mut().remove(resolved_path)) {
        return result;
    }
    load_file(resolved_path).await.map_err(|e| format!("{e:?}"))
}
