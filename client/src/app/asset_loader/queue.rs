use client::screens::PlayingState;
use platform::Instant;

use super::cache::{asset_loaded, has_ext};
use super::constants::PREFETCH_AHEAD;
use super::entry::load_asset_entry;
use super::paths::resolve_runtime_asset_path;
use super::prefetch::{release_decoded, Prefetcher, PREFETCHED};

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
    pending: Vec<Pending>,
    /// What the player needs before spawning; the rest may arrive later.
    essential: std::collections::HashSet<String>,
    prefetcher: Prefetcher,
    total: usize,
    essential_total: usize,
    /// Spaces out loads while playing ([`AssetQueue::pump_paced`]).
    pacer: LoadPacer,
    /// How many of `pending` the last [`prioritize`](Self::prioritize) put
    /// at the front: needed now, so not paced.
    wanted: usize,
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
        let mut pending: Vec<Pending> = entries
            .iter()
            .filter(|(key, path)| !asset_loaded(playing_state, key, path))
            .map(|entry| Pending { resolved: resolve_runtime_asset_path(&entry.1), entry: entry.clone() })
            .collect();
        essential.extend(pending.iter().filter(|p| !is_model(p.path())).map(|p| p.key().to_string()));
        // Essentials first, small files before models among them; otherwise
        // manifest order (the sort is stable).
        pending.sort_by_key(|p| (!essential.contains(p.key()), is_model(p.path())));
        let essential_total = pending.iter().filter(|p| essential.contains(p.key())).count();
        // Downloads left over from an earlier game are not wanted any more.
        PREFETCHED.with(|p| p.borrow_mut().clear());
        release_decoded();
        AssetQueue {
            total: pending.len(),
            essential_total,
            pending,
            essential,
            prefetcher: Prefetcher::default(),
            pacer: LoadPacer::default(),
            wanted: 0,
        }
    }

    /// Move `keys` (those still pending) to the front, in the order given.
    pub fn prioritize<'a>(&mut self, keys: impl IntoIterator<Item = &'a str>) {
        // In place, as it runs every frame while playing: `pending[..front]`
        // are the keys moved so far, in order; the rest keep their order.
        // A key already where it belongs moves nothing.
        let mut front = 0;
        for key in keys {
            if let Some(i) = self.pending[front..].iter().position(|p| p.key() == key) {
                self.pending[front..=front + i].rotate_right(1);
                front += 1;
            }
        }
        self.wanted = front;
    }

    /// Mark `keys` (those still pending) as needed before spawning too, and
    /// move them to the front: for a queue started before the loadout was final.
    pub fn require<'a>(&mut self, keys: impl IntoIterator<Item = &'a str>) {
        let keys: Vec<&str> = keys.into_iter().collect();
        for key in &keys {
            if !self.essential.contains(*key) && self.pending.iter().any(|p| p.key() == *key) {
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
        let Some(i) = self.next_arrived(playing_state) else {
            return false;
        };
        self.load(i, playing_state).await;
        true
    }

    /// [`pump`](Self::pump) for frames of a game being played: a load parses
    /// and uploads on this thread, so it is a hitch as long as itself, and
    /// back to back they stutter. [`LoadPacer`] decides from `prev_frame_s`
    /// (the previous frame's length, seconds) whether one may start; the
    /// downloads ahead keep going either way. What the player needs now
    /// (put at the front by `prioritize`: clips and models of the players in
    /// the game, whose hitboxes come from them) is not held back: one a frame,
    /// as before pacing; its time still counts against the rest.
    pub async fn pump_paced(&mut self, playing_state: &mut PlayingState, prev_frame_s: f32) -> bool {
        // every frame, so the pacer counts all the time that passes
        let allowed = self.pacer.ready(prev_frame_s * 1000.0);
        let next = self.next_arrived(playing_state);
        let Some(i) = next.filter(|&i| allowed || i < self.wanted) else {
            return false;
        };
        let started = Instant::now();
        self.load(i, playing_state).await;
        self.pacer.loaded(started.elapsed().as_secs_f32() * 1000.0);
        true
    }

    /// Drop what is loaded already, keep the next few downloading, and return
    /// the index of the first of them whose bytes are here.
    fn next_arrived(&mut self, playing_state: &PlayingState) -> Option<usize> {
        // Something else loaded these meanwhile (e.g. the player list asking
        // for a model): nothing left to do for them.
        let (mut at, mut gone_wanted) = (0, 0);
        let wanted = self.wanted;
        self.pending.retain(|p| {
            let keep = !asset_loaded(playing_state, p.key(), p.path());
            gone_wanted += usize::from(!keep && at < wanted);
            at += 1;
            keep
        });
        self.wanted -= gone_wanted;
        for p in self.pending.iter().take(PREFETCH_AHEAD) {
            self.prefetcher.start(&p.resolved);
        }
        let window = self.pending.len().min(PREFETCH_AHEAD);
        (0..window).find(|&i| self.prefetcher.ready(&self.pending[i].resolved))
    }

    async fn load(&mut self, i: usize, playing_state: &mut PlayingState) {
        let (key, path) = self.pending.remove(i).entry;
        if i < self.wanted {
            self.wanted -= 1;
        }
        // A failed load is dropped too: retrying would stall the queue.
        load_asset_entry(&key, &path, playing_state).await;
        if self.pending.is_empty() {
            // What was decoded ahead and not taken, no mesh used.
            release_decoded();
        }
    }

    /// Every essential asset is loaded (or failed for good).
    pub fn ready_to_play(&self) -> bool {
        !self.pending.iter().any(|p| self.essential.contains(p.key()))
    }

    pub fn is_done(&self) -> bool {
        self.pending.is_empty()
    }

    /// (loaded, total) of the essential part, for the loading screen.
    pub fn essential_progress(&self) -> (usize, usize) {
        let left = self.pending.iter().filter(|p| self.essential.contains(p.key())).count();
        (self.essential_total.saturating_sub(left), self.essential_total)
    }

    /// (loaded, total) of everything queued for this game.
    pub fn progress(&self) -> (usize, usize) {
        (self.total - self.pending.len(), self.total)
    }

    /// The asset being worked on now, as (key, path).
    pub fn current(&self) -> Option<&(String, String)> {
        self.pending.first().map(|p| &p.entry)
    }
}

/// A queued asset: its manifest entry (key, path), and the path resolved
/// once here rather than on every frame's download checks.
struct Pending {
    entry: (String, String),
    resolved: String,
}

impl Pending {
    fn key(&self) -> &str {
        &self.entry.0
    }

    fn path(&self) -> &str {
        &self.entry.1
    }
}

/// Share of the time while playing that background loads may take.
const LOAD_SHARE: f32 = 0.25;
/// A frame this long (ms) is already over budget: no load starts after one.
const SLOW_FRAME_MS: f32 = 20.0;
/// However slow the frames, a load starts at least this often (ms), so the
/// queue never stalls.
const MAX_GAP_MS: f32 = 500.0;

/// When the next background load may start while playing. A load of `x` ms
/// leaves a debt of `x` that every frame pays back by [`LOAD_SHARE`] of its
/// length, so loads take about that share of the time, spread out rather
/// than back to back. None starts after a frame already over
/// [`SLOW_FRAME_MS`] (which includes the frame a big load just made slow),
/// and one always may once [`MAX_GAP_MS`] passed since the last.
#[derive(Default)]
struct LoadPacer {
    debt_ms: f32,
    since_load_ms: f32,
    load_in_next_frame_ms: f32,
}

impl LoadPacer {
    /// Called once per frame with the previous frame's length: may a load
    /// start in this one?
    fn ready(&mut self, prev_frame_ms: f32) -> bool {
        let frame = prev_frame_ms.max(0.0);
        self.since_load_ms += (frame - std::mem::take(&mut self.load_in_next_frame_ms)).max(0.0);
        self.debt_ms = (self.debt_ms - frame * LOAD_SHARE).max(0.0);
        self.since_load_ms >= MAX_GAP_MS || (self.debt_ms <= 0.0 && frame <= SLOW_FRAME_MS)
    }

    /// A load just took `took_ms`.
    fn loaded(&mut self, took_ms: f32) {
        self.debt_ms = took_ms.max(0.0);
        // The gap counts from the load's end. The next frame's length
        // includes a load done in one go, so that much of it is left out (or
        // a long one would start the next right after); a load that awaited
        // a download over several frames is over by then, and leaves out
        // no more than that one frame.
        self.since_load_ms = 0.0;
        self.load_in_next_frame_ms = self.debt_ms;
    }
}

fn is_model(path: &str) -> bool {
    has_ext(path, ".glb") || has_ext(path, ".gltf") || has_ext(path, ".obj")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue(entries: &[(&str, &str)]) -> AssetQueue {
        let pending = entries
            .iter()
            .map(|(k, p)| Pending { entry: (k.to_string(), p.to_string()), resolved: p.to_string() })
            .collect();
        AssetQueue { pending, ..AssetQueue::default() }
    }

    fn order(queue: &AssetQueue) -> Vec<(String, String)> {
        queue.pending.iter().map(|p| p.entry.clone()).collect()
    }

    /// `prioritize` as it was, moving out to a new Vec: the in-place one must
    /// give the same order.
    fn prioritize_reference(pending: &mut Vec<(String, String)>, keys: &[&str]) {
        let mut front = Vec::new();
        for key in keys {
            if let Some(i) = pending.iter().position(|p| p.0 == *key) {
                front.push(pending.remove(i));
            }
        }
        if !front.is_empty() {
            front.append(pending);
            *pending = front;
        }
    }

    /// Frames of `frame_ms` (plus the load's own time on the frame after a
    /// load, as a real frame would measure it) where every load takes
    /// `load_ms`: the frames on which a load started.
    fn paced(frame_ms: f32, load_ms: f32, frames: usize) -> Vec<usize> {
        let mut pacer = LoadPacer::default();
        let mut prev = frame_ms;
        let mut starts = Vec::new();
        for f in 0..frames {
            if pacer.ready(prev) {
                pacer.loaded(load_ms);
                starts.push(f);
                prev = frame_ms + load_ms;
            } else {
                prev = frame_ms;
            }
        }
        starts
    }

    #[test]
    fn small_loads_go_every_frame() {
        // a 1 ms sound: nothing to pay back by the next frame
        assert_eq!(paced(16.7, 1.0, 10), (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn big_loads_are_spread_to_their_share() {
        for &(frame, load) in &[(16.7, 40.0), (16.7, 120.0), (8.3, 60.0)] {
            let starts = paced(frame, load, 2000);
            assert!(starts.windows(2).all(|w| w[1] - w[0] > 1), "back to back: {frame} {load}");
            let wall = 2000.0 * frame + starts.len() as f32 * load;
            let share = starts.len() as f32 * load / wall;
            assert!(share <= LOAD_SHARE + 0.02 && share >= LOAD_SHARE - 0.08, "share {share}: {frame} {load}");
        }
    }

    #[test]
    fn slow_frames_wait_but_never_forever() {
        // already over budget every frame (30 fps): only the guarantee loads
        let starts = paced(33.3, 5.0, 300);
        assert!(!starts.is_empty());
        let max_gap_frames = (MAX_GAP_MS / 33.3).ceil() as usize;
        assert!(starts.windows(2).all(|w| w[1] - w[0] <= max_gap_frames), "{starts:?}");
        assert!(starts[0] <= max_gap_frames);
        // a load so long its debt would outlast the guarantee
        let starts = paced(16.7, 5000.0, 400);
        assert!(starts.len() >= 2);
        assert!(starts.windows(2).all(|w| w[1] - w[0] <= (MAX_GAP_MS / 16.7).ceil() as usize + 1));
    }

    // a load that waited on a download over several short frames: the next
    // still comes within the guarantee of its end, not that plus the wait
    #[test]
    fn a_long_wait_on_the_network_does_not_push_the_next_load_back() {
        let mut pacer = LoadPacer::default();
        assert!(pacer.ready(16.7));
        pacer.loaded(3000.0);
        let frames = (1..100).position(|_| pacer.ready(16.7)).unwrap() + 1;
        assert!(frames as f32 * 16.7 <= MAX_GAP_MS + 2.0 * 16.7, "{frames} frames");
    }

    #[test]
    fn one_slow_frame_skips_one_load() {
        let mut pacer = LoadPacer::default();
        assert!(pacer.ready(16.7));
        pacer.loaded(0.5);
        assert!(!pacer.ready(25.0));
        assert!(pacer.ready(16.7));
    }

    #[test]
    fn prioritize_matches_reference() {
        // duplicate keys too (the manifest and the viewmodel list both name some)
        let entries = [
            ("a", "1.glb"), ("b", "2.png"), ("c", "3.glb"), ("a", "1.glb"),
            ("d", "4.ogg"), ("e", "5.glb"), ("c", "6.glb"), ("f", "7.glb"),
        ];
        let key_lists: &[&[&str]] = &[
            &[], &["a"], &["f"], &["a", "a"], &["c", "a", "c", "a", "c"], &["z", "e", "z"],
            &["f", "e", "d", "c", "b", "a"], &["a", "b", "c"], &["b", "a", "f", "a", "x"],
        ];
        for keys in key_lists {
            // twice in a row too: the second time is the usual case, nothing moves
            let mut q = queue(&entries);
            let mut expected = order(&q);
            for _ in 0..2 {
                q.prioritize(keys.iter().copied());
                prioritize_reference(&mut expected, keys);
                assert_eq!(order(&q), expected, "keys {keys:?}");
            }
        }
        // every key list of length 3 over the keys (and one not queued)
        let alphabet = ["a", "b", "c", "d", "e", "f", "x"];
        for &k1 in &alphabet {
            for &k2 in &alphabet {
                for &k3 in &alphabet {
                    let keys = [k1, k2, k3];
                    let mut q = queue(&entries);
                    let mut expected = order(&q);
                    q.prioritize(keys);
                    prioritize_reference(&mut expected, &keys);
                    assert_eq!(order(&q), expected, "keys {keys:?}");
                }
            }
        }
    }
}
