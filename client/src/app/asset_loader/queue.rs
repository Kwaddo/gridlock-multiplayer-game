use client::screens::PlayingState;

use super::cache::asset_loaded;
use super::constants::PREFETCH_AHEAD;
use super::entry::load_asset_entry;
use super::prefetch::{Prefetcher, PREFETCHED};

/// The game's assets, loaded in priority order a few frames at a time, so a
/// player can be in the game before all of them are here.
///
/// Joining waits only for what the first seconds need ([`AssetQueue::ready_to_play`]):
/// the small files, the player's own weapons and hands, and the idle
/// animation everyone else falls back to. Everything else keeps loading in
/// the background while playing; whatever the player is about to need (a
/// weapon picked in the lobby, an animation someone is playing) moves to the
/// front. The rest of the game copes with a missing model by drawing a
/// fallback until it arrives.
#[derive(Default)]
pub struct AssetQueue {
    /// Not loaded yet, highest priority first.
    pending: Vec<(String, String)>,
    /// What the player needs before spawning; the rest may arrive later.
    essential: std::collections::HashSet<String>,
    prefetcher: Prefetcher,
    total: usize,
    essential_total: usize,
}

impl AssetQueue {
    /// Queue `entries` (manifest order) for a game. `essential_models` are the
    /// models the player needs before spawning; every sound and texture is
    /// essential too (they are small). Assets already loaded (a second game
    /// this session) are skipped.
    pub fn start<'a>(
        entries: &[(String, String)],
        essential_models: impl IntoIterator<Item = &'a str>,
        playing_state: &PlayingState,
    ) -> Self {
        let mut essential: std::collections::HashSet<String> =
            essential_models.into_iter().map(str::to_string).collect();
        let mut pending: Vec<(String, String)> = entries
            .iter()
            .filter(|(key, path)| !asset_loaded(playing_state, key, path))
            .cloned()
            .collect();
        essential.extend(pending.iter().filter(|(_, path)| !is_model(path)).map(|(key, _)| key.clone()));
        // Essentials first, small files before models among them; otherwise
        // manifest order (the sort is stable).
        pending.sort_by_key(|(key, path)| (!essential.contains(key), is_model(path)));
        let essential_total = pending.iter().filter(|(key, _)| essential.contains(key)).count();
        // Downloads left over from an earlier game are not wanted any more.
        PREFETCHED.with(|p| p.borrow_mut().clear());
        AssetQueue { total: pending.len(), essential_total, pending, essential, prefetcher: Prefetcher::default() }
    }

    /// Move `keys` (those still pending) to the front, in the order given.
    pub fn prioritize<'a>(&mut self, keys: impl IntoIterator<Item = &'a str>) {
        let mut front = Vec::new();
        for key in keys {
            if let Some(i) = self.pending.iter().position(|(k, _)| k == key) {
                front.push(self.pending.remove(i));
            }
        }
        if !front.is_empty() {
            front.append(&mut self.pending);
            self.pending = front;
        }
    }

    /// Mark `keys` (those still pending) as needed before spawning too, and
    /// move them to the front: for a queue started before the loadout was final.
    pub fn require<'a>(&mut self, keys: impl IntoIterator<Item = &'a str>) {
        let keys: Vec<&str> = keys.into_iter().collect();
        for key in &keys {
            if !self.essential.contains(*key) && self.pending.iter().any(|(k, _)| k == key) {
                self.essential.insert(key.to_string());
                self.essential_total += 1;
            }
        }
        self.prioritize(keys);
    }

    /// Load at most one asset whose bytes are already here (no frame waits on
    /// the network) and keep the next few downloading. Returns whether it
    /// loaded one. Takes the first of the next few that has arrived, so one
    /// slow download does not hold up the ones behind it.
    pub async fn pump(&mut self, playing_state: &mut PlayingState) -> bool {
        // Something else loaded these meanwhile (e.g. the player list asking
        // for a model): nothing left to do for them.
        self.pending.retain(|(key, path)| !asset_loaded(playing_state, key, path));
        for (_, path) in self.pending.iter().take(PREFETCH_AHEAD) {
            self.prefetcher.start(path);
        }
        let window = self.pending.len().min(PREFETCH_AHEAD);
        let Some(i) = (0..window).find(|&i| self.prefetcher.ready(&self.pending[i].1)) else {
            return false;
        };
        let (key, path) = self.pending.remove(i);
        // A failed load is dropped too: retrying would stall the queue.
        load_asset_entry(&key, &path, playing_state).await;
        true
    }

    /// Every essential asset is loaded (or failed for good).
    pub fn ready_to_play(&self) -> bool {
        !self.pending.iter().any(|(key, _)| self.essential.contains(key))
    }

    pub fn is_done(&self) -> bool {
        self.pending.is_empty()
    }

    /// (loaded, total) of the essential part, for the loading screen.
    pub fn essential_progress(&self) -> (usize, usize) {
        let left = self.pending.iter().filter(|(key, _)| self.essential.contains(key)).count();
        (self.essential_total.saturating_sub(left), self.essential_total)
    }

    /// (loaded, total) of everything queued for this game.
    pub fn progress(&self) -> (usize, usize) {
        (self.total - self.pending.len(), self.total)
    }

    /// The asset being worked on now, as (key, path).
    pub fn current(&self) -> Option<&(String, String)> {
        self.pending.first()
    }
}

fn is_model(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".glb") || lower.ends_with(".gltf") || lower.ends_with(".obj")
}
