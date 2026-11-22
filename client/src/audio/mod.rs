//! Sound effects: rodio on native builds, Web Audio in the browser (through the
//! `gridlock_web` JS plugin; rodio's browser backend would need wasm-bindgen).
//! Both play each sample fire-and-forget with its own speed, volume and pan.
//!
//! Every sound the game plays has a key (see [`catalog`]). A sound exists only
//! when its file is listed in the manifest (e.g. `"coin": "assets/sounds/coin.ogg"`),
//! nothing is generated, so the game is silent unless we add a sound on purpose.

pub mod catalog;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

pub use catalog::Cue;

/// A native sample: decoded once, on the mixer thread the first time it plays,
/// and shared by every later play. Cloning it is a refcount bump, so a shot
/// costs no copy and no decoder setup on the main thread.
#[cfg(not(target_arch = "wasm32"))]
type Sample = rodio::source::Buffered<rodio::Decoder<std::io::Cursor<Vec<u8>>>>;

pub struct Audio {
    #[cfg(not(target_arch = "wasm32"))]
    output: Option<(rodio::OutputStream, rodio::OutputStreamHandle)>,
    #[cfg(not(target_arch = "wasm32"))]
    samples: HashMap<String, Sample>,

    #[cfg(target_arch = "wasm32")]
    sounds: HashMap<String, u32>,

    /// Master volume (0..1) times every play; 0 while muted. Starts silent
    /// until the settings are applied with [`Audio::set_levels`].
    master: f32,

    /// Keys played with no file loaded, so each one is warned about once.
    warned: RefCell<HashSet<String>>,
}

impl Audio {
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let audio = Audio {
            output: rodio::OutputStream::try_default()
                .map_err(|e| println!("Failed to initialize audio output stream: {:?}", e))
                .ok(),
            samples: HashMap::new(),
            master: 0.0,
            warned: RefCell::default(),
        };

        #[cfg(target_arch = "wasm32")]
        let audio = Audio { sounds: HashMap::new(), master: 0.0, warned: RefCell::default() };

        audio
    }

    /// Apply the player's volume settings. Cheap; called every frame.
    pub fn set_levels(&mut self, volume: f32, muted: bool) {
        self.master = if muted { 0.0 } else { volume.clamp(0.0, 1.0) };
    }

    /// Whether `key` can play (its file has loaded).
    pub fn has(&self, key: &str) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self.samples.contains_key(key);

        #[cfg(target_arch = "wasm32")]
        return self.sounds.contains_key(key);
    }

    /// Whether `key`'s file has loaded (the asset loader skips it then).
    pub fn has_file(&self, key: &str) -> bool {
        self.has(key)
    }

    /// Register an encoded sample (e.g. Ogg) under `key`.
    pub async fn insert(&mut self, key: String, bytes: Vec<u8>) -> Result<(), String> {
        self.insert_bytes(key, bytes)
    }

    fn insert_bytes(&mut self, key: String, bytes: Vec<u8>) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use rodio::Source;
            let decoder = rodio::Decoder::new(std::io::Cursor::new(bytes))
                .map_err(|e| format!("{key}: {e}"))?;
            self.samples.insert(key, decoder.buffered());
            Ok(())
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Decoding happens in the background; plays before it finishes are skipped.
            let id = unsafe { web::gridlock_audio_load(bytes.as_ptr(), bytes.len() as u32) };
            self.sounds.insert(key, id);
            Ok(())
        }
    }

    /// Fire and forget. `speed` scales pitch and tempo.
    pub fn play(&self, key: &str, speed: f32, volume: f32) {
        self.play_panned(key, speed, volume, 0.0);
    }

    /// Play a catalogue cue at its own volume, with its pitch jitter.
    pub fn play_cue(&self, cue: Cue) {
        self.play_cue_scaled(cue, 1.0);
    }

    /// Play a cue with its volume scaled by `scale`.
    pub fn play_cue_scaled(&self, cue: Cue, scale: f32) {
        let pitch = if cue.pitch_jitter > 0.0 {
            macroquad::rand::gen_range(1.0 - cue.pitch_jitter, 1.0 + cue.pitch_jitter)
        } else {
            1.0
        };
        self.play(cue.key, pitch, cue.volume * scale);
    }

    /// Fire and forget, placed in the stereo field: `pan` -1 is fully left,
    /// 0 centred, 1 fully right.
    pub fn play_panned(&self, key: &str, speed: f32, volume: f32, pan: f32) {
        let volume = volume * self.master;
        if volume <= 0.001 {
            return;
        }
        let pan = pan.clamp(-1.0, 1.0);

        #[cfg(not(target_arch = "wasm32"))]
        {
            use rodio::Source;
            let Some((_, handle)) = &self.output else {
                return;
            };
            let Some(sample) = self.samples.get(key) else {
                self.warn_missing(key);
                return;
            };
            let source = sample.clone().speed(speed).amplify(volume);
            if pan.abs() < 0.01 {
                let _ = handle.play_raw(source.convert_samples::<f32>());
            } else {
                // Balance law: the near ear stays at full level, the far one drops.
                let left = (1.0 - pan).min(1.0);
                let right = (1.0 + pan).min(1.0);
                let panned = rodio::source::ChannelVolume::new(source, vec![left, right]);
                let _ = handle.play_raw(panned.convert_samples::<f32>());
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            match self.sounds.get(key) {
                Some(&id) => unsafe { web::gridlock_audio_play(id, speed, volume, pan) },
                None => self.warn_missing(key),
            }
        }
    }

    /// A key with no file behind it (not in the manifest, or it failed to load).
    /// Only reached when a play misses, and it says so once per key.
    #[cold]
    fn warn_missing(&self, key: &str) {
        let mut warned = self.warned.borrow_mut();
        if !warned.contains(key) {
            macroquad::logging::warn!("sound '{}' isn't loaded, is it in manifest.json?", key);
            warned.insert(key.to_string());
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    extern "C" {
        pub fn gridlock_audio_load(ptr: *const u8, len: u32) -> u32;
        pub fn gridlock_audio_play(id: u32, speed: f32, volume: f32, pan: f32);
    }
}
