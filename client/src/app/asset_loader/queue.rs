use client::screens::PlayingState;
use platform::Instant;

use super::cache::{asset_loaded, has_ext};
use super::constants::PREFETCH_AHEAD;
use super::entry::begin_asset_entry;
use super::glb_job::GlbJob;
use super::pacer::LoadPacer;
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
    /// A model taken off `pending` and loading a step at a time.
    in_progress: Option<InProgress>,
}

/// A model loading a step at a time: its manifest entry and the job.
struct InProgress {
    entry: (String, String),
    job: GlbJob,
    /// Asked for by the last [`AssetQueue::prioritize`]: needed now.
    wanted: bool,
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
            in_progress: None,
        }
    }

    /// Move `keys` (those still pending) to the front, in the order given.
    pub fn prioritize<'a>(&mut self, keys: impl IntoIterator<Item = &'a str>) {
        // In place, as it runs every frame while playing: `pending[..front]`
        // are the keys moved so far, in order; the rest keep their order.
        // A key already where it belongs moves nothing.
        let mut front = 0;
        let mut job_wanted = false;
        for key in keys {
            job_wanted |= self.in_progress.as_ref().is_some_and(|p| p.job.key() == key);
            if let Some(i) = self.pending[front..].iter().position(|p| p.key() == key) {
                self.pending[front..=front + i].rotate_right(1);
                front += 1;
            }
        }
        self.wanted = front;
        if let Some(in_progress) = &mut self.in_progress {
            in_progress.wanted = job_wanted;
        }
    }

    /// Mark `keys` (those still pending) as needed before spawning too, and
    /// move them to the front: for a queue started before the loadout was final.
    pub fn require<'a>(&mut self, keys: impl IntoIterator<Item = &'a str>) {
        let keys: Vec<&str> = keys.into_iter().collect();
        for key in &keys {
            let queued = self.pending.iter().any(|p| p.key() == *key)
                || self.in_progress.as_ref().is_some_and(|p| p.job.key() == *key);
            if !self.essential.contains(*key) && queued {
                self.essential.insert(key.to_string());
                self.essential_total += 1;
            }
        }
        self.prioritize(keys);
    }

    /// Load at most one asset whose bytes are already here (no frame waits on
    /// the network) and keep the next few downloading. Returns whether it
    /// loaded one. Takes the first of the next few that has arrived, so one
    /// slow download does not hold up the ones behind it. A model
    /// [`pump_paced`](Self::pump_paced) left half loaded is finished first.
    /// Each asset is loaded whole, in one go.
    pub async fn pump(&mut self, playing_state: &mut PlayingState) -> bool {
        if self.in_progress.is_some() {
            while !self.step_job(playing_state) {}
            return true;
        }
        let Some(i) = self.next_arrived(playing_state) else {
            return false;
        };
        self.begin(i, playing_state).await;
        while !self.step_job(playing_state) {}
        true
    }

    /// [`pump`](Self::pump) for frames of a game being played: a load parses
    /// and uploads on this thread, so it is a hitch as long as itself, and
    /// back to back they stutter. [`LoadPacer`] decides from `prev_frame_s`
    /// (the previous frame's length, seconds) whether one may start; the
    /// downloads ahead keep going either way. What the player needs now
    /// (put at the front by `prioritize`: clips and models of the players in
    /// the game, whose hitboxes come from them) is not held back: one whole
    /// load a frame; its time still counts against the rest.
    ///
    /// Any other model (a GLB) loads a step a frame (see `GlbJob`): the first
    /// step reads and checks the file, the next upload its textures and build
    /// it a stage at a time, the last stores it. Each step is paced as a load.
    /// Returns whether it loaded an asset or did a step of one.
    pub async fn pump_paced(&mut self, playing_state: &mut PlayingState, prev_frame_s: f32) -> bool {
        // every frame, so the pacer counts all the time that passes
        let allowed = self.pacer.ready(prev_frame_s * 1000.0);
        let next = self.next_arrived(playing_state);
        let started = Instant::now();
        if let Some(in_progress) = &self.in_progress {
            // Something needed now waits behind the model: it goes unpaced too.
            let wanted_waiting = next.is_some_and(|i| i < self.wanted);
            if !may_step(allowed, in_progress.wanted, wanted_waiting) {
                return false;
            }
            // Needed now (a player's model, its hitboxes come from it): the
            // rest of it at once, as before steps, not over several frames.
            if in_progress.wanted {
                while !self.step_job(playing_state) {}
            } else {
                self.step_job(playing_state);
            }
        } else {
            let Some(i) = next.filter(|&i| allowed || i < self.wanted) else {
                return false;
            };
            let wanted = i < self.wanted;
            self.begin(i, playing_state).await;
            if wanted {
                while !self.step_job(playing_state) {}
            }
        }
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
        if self.in_progress.as_ref().is_some_and(|p| asset_loaded(playing_state, &p.entry.0, &p.entry.1)) {
            self.in_progress = None;
            self.release_if_done();
        }
        for p in self.pending.iter().take(PREFETCH_AHEAD) {
            self.prefetcher.start(&p.resolved);
        }
        let window = self.pending.len().min(PREFETCH_AHEAD);
        (0..window).find(|&i| self.prefetcher.ready(&self.pending[i].resolved))
    }

    /// Load `pending[i]`, or (a model) start loading it a step at a time.
    async fn begin(&mut self, i: usize, playing_state: &mut PlayingState) {
        let (key, path) = self.pending.remove(i).entry;
        let wanted = i < self.wanted;
        if wanted {
            self.wanted -= 1;
        }
        // A failed load is dropped too: retrying would stall the queue.
        match begin_asset_entry(&key, &path, playing_state).await {
            Some(job) => self.in_progress = Some(InProgress { entry: (key, path), job, wanted }),
            None => self.release_if_done(),
        }
    }

    /// The next step of the model loading, if one is. True once none is
    /// (it is done, or failed and dropped).
    fn step_job(&mut self, playing_state: &mut PlayingState) -> bool {
        let Some(in_progress) = &mut self.in_progress else {
            return true;
        };
        if !in_progress.job.step(playing_state) {
            return false;
        }
        self.in_progress = None;
        self.release_if_done();
        true
    }

    fn release_if_done(&mut self) {
        if self.is_done() {
            // What was decoded ahead and not taken, no mesh used.
            release_decoded();
        }
    }

    /// The model loading a step at a time, if it is essential: 1, else 0.
    fn essential_in_progress(&self) -> usize {
        usize::from(self.in_progress.as_ref().is_some_and(|p| self.essential.contains(&p.entry.0)))
    }

    /// Every essential asset is loaded (or failed for good).
    pub fn ready_to_play(&self) -> bool {
        self.essential_in_progress() == 0 && !self.pending.iter().any(|p| self.essential.contains(p.key()))
    }

    pub fn is_done(&self) -> bool {
        self.pending.is_empty() && self.in_progress.is_none()
    }

    /// (loaded, total) of the essential part, for the loading screen.
    pub fn essential_progress(&self) -> (usize, usize) {
        let left = self.pending.iter().filter(|p| self.essential.contains(p.key())).count()
            + self.essential_in_progress();
        (self.essential_total.saturating_sub(left), self.essential_total)
    }

    /// (loaded, total) of everything queued for this game.
    pub fn progress(&self) -> (usize, usize) {
        let left = self.pending.len() + usize::from(self.in_progress.is_some());
        (self.total - left, self.total)
    }

    /// The asset being worked on now, as (key, path).
    pub fn current(&self) -> Option<&(String, String)> {
        match &self.in_progress {
            Some(in_progress) => Some(&in_progress.entry),
            None => self.pending.first().map(|p| &p.entry),
        }
    }
}

/// May the model loading a step at a time take its next step this frame?
/// When the pacer `allowed` one, or unpaced when it is `wanted` now, or when
/// something wanted now has arrived and waits behind it.
fn may_step(allowed: bool, wanted: bool, wanted_waiting: bool) -> bool {
    allowed || wanted || wanted_waiting
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

    /// A model loading in steps: its manifest entry, the job built from a
    /// file the static loader takes (no step of it runs here).
    fn in_progress(key: &str) -> InProgress {
        let bytes = super::super::glb_job::test_glb(r#"{"asset":{"version":"2.0"}}"#);
        let job = GlbJob::start(key, "p.glb", "p.glb", &bytes).unwrap();
        InProgress { entry: (key.to_string(), "p.glb".to_string()), job, wanted: false }
    }

    /// A model half loaded is still queued: not done, not ready to play if
    /// essential, not counted as loaded, and what is being worked on.
    #[test]
    fn a_model_loading_in_steps_is_still_queued() {
        let mut q = queue(&[("b", "2.glb")]);
        q.total = 2;
        q.essential.insert("a".to_string());
        q.essential_total = 1;
        q.in_progress = Some(in_progress("a"));
        assert!(!q.is_done());
        assert!(!q.ready_to_play());
        assert_eq!(q.progress(), (0, 2));
        assert_eq!(q.essential_progress(), (0, 1));
        assert_eq!(q.current().map(|e| e.0.as_str()), Some("a"));
        // done with it
        q.in_progress = None;
        assert!(!q.is_done());
        assert!(q.ready_to_play());
        assert_eq!(q.progress(), (1, 2));
        assert_eq!(q.essential_progress(), (1, 1));
        assert_eq!(q.current().map(|e| e.0.as_str()), Some("b"));
    }

    /// `prioritize` tells the model in progress whether it is wanted now,
    /// and `require` makes it essential like a pending one.
    #[test]
    fn a_model_in_progress_can_be_wanted_and_required() {
        let mut q = queue(&[("b", "2.glb"), ("c", "3.glb")]);
        q.in_progress = Some(in_progress("a"));
        let wanted = |q: &AssetQueue| q.in_progress.as_ref().unwrap().wanted;
        q.prioritize(["c", "a"]);
        assert!(wanted(&q));
        assert_eq!(q.wanted, 1, "only pending ones count at the front");
        q.prioritize(["c"]);
        assert!(!wanted(&q));
        assert!(q.ready_to_play());
        q.require(["a", "z"]);
        assert!(wanted(&q));
        assert!(!q.ready_to_play());
        assert_eq!(q.essential_progress(), (0, 1));
        q.require(["a"]);
        assert_eq!(q.essential_progress(), (0, 1), "counted once");
    }

    #[test]
    fn steps_wait_for_the_pacer_unless_needed_now() {
        assert!(!may_step(false, false, false));
        assert!(may_step(true, false, false));
        assert!(may_step(false, true, false), "wanted itself");
        assert!(may_step(false, false, true), "something wanted waits behind it");
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
