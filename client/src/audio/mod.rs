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
    output: Output,
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
            output: Output::open(),
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
            let Some(handle) = self.output.handle() else {
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

/// The output device (native builds), opened on a thread of its own: opening
/// it takes tens of milliseconds (far longer the first time after boot), which
/// held up the first frame, and closing it about as long. The stream can not
/// move between threads, so that thread keeps it until the `Audio` is dropped.
#[cfg(not(target_arch = "wasm32"))]
struct Output {
    handle: std::cell::OnceCell<Option<rodio::OutputStreamHandle>>,
    opened: std::sync::mpsc::Receiver<Option<rodio::OutputStreamHandle>>,
    /// Dropped with the `Audio`, which has the thread close the stream.
    _close: std::sync::mpsc::Sender<()>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Output {
    fn open() -> Self {
        let (opened_tx, opened) = std::sync::mpsc::channel();
        let (close, closed) = std::sync::mpsc::channel::<()>();
        let spawned = std::thread::Builder::new().name("audio output".into()).spawn(move || {
            let stream = rodio::OutputStream::try_default()
                .map_err(|e| println!("Failed to initialize audio output stream: {:?}", e))
                .ok();
            let Some((stream, handle)) = stream else {
                let _ = opened_tx.send(None);
                return;
            };
            let _ = opened_tx.send(Some(handle));
            let _ = closed.recv(); // until the `Audio` is dropped
            drop(stream);
        });
        if let Err(e) = spawned {
            println!("Failed to initialize audio output stream: {:?}", e);
        }
        Output { handle: std::cell::OnceCell::new(), opened, _close: close }
    }

    /// The device, or `None` if it could not be opened. The first call waits
    /// for it to open, which it normally did long before the first sound.
    fn handle(&self) -> Option<&rodio::OutputStreamHandle> {
        self.handle.get_or_init(|| self.opened.recv().ok().flatten()).as_ref()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::Output;

    /// Opening the device does not block, asking for it gives the same
    /// answer every time (also without a device), and dropping it does not
    /// wait for the stream to close.
    #[test]
    fn output_opens_in_the_background() {
        let output = Output::open();
        let opened = output.handle().is_some();
        assert_eq!(output.handle().is_some(), opened);
        let start = std::time::Instant::now();
        drop(output);
        assert!(start.elapsed() < std::time::Duration::from_millis(20), "closed on its own thread");
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    extern "C" {
        pub fn gridlock_audio_load(ptr: *const u8, len: u32) -> u32;
        pub fn gridlock_audio_play(id: u32, speed: f32, volume: f32, pan: f32);
    }
}
