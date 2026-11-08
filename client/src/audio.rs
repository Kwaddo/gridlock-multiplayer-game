//! Sound effects: rodio on native builds, Web Audio in the browser (through the
//! `gridlock_web` JS plugin; rodio's browser backend would need wasm-bindgen).
//! Both play each sample fire-and-forget with its own speed and volume.

use std::collections::HashMap;

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
}

impl Audio {
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        return Audio {
            output: rodio::OutputStream::try_default()
                .map_err(|e| println!("Failed to initialize audio output stream: {:?}", e))
                .ok(),
            samples: HashMap::new(),
        };

        #[cfg(target_arch = "wasm32")]
        return Audio { sounds: HashMap::new() };
    }

    pub fn has(&self, key: &str) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self.samples.contains_key(key);

        #[cfg(target_arch = "wasm32")]
        return self.sounds.contains_key(key);
    }

    /// Register an encoded sample (e.g. Ogg) under `key`.
    pub async fn insert(&mut self, key: String, bytes: Vec<u8>) -> Result<(), String> {
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
        #[cfg(not(target_arch = "wasm32"))]
        {
            use rodio::Source;
            let (Some(sample), Some((_, handle))) = (self.samples.get(key), &self.output) else {
                return;
            };
            let source = sample.clone().speed(speed).amplify(volume).convert_samples::<f32>();
            let _ = handle.play_raw(source);
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(&id) = self.sounds.get(key) {
                unsafe { web::gridlock_audio_play(id, speed, volume) };
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    extern "C" {
        pub fn gridlock_audio_load(ptr: *const u8, len: u32) -> u32;
        pub fn gridlock_audio_play(id: u32, speed: f32, volume: f32);
    }
}
