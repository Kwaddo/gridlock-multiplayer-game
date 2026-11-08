//! Placeholder sound effects, synthesized at startup from noise and tones and
//! handed to the audio backend as 16-bit mono WAV files. They keep the game
//! from being silent until real recordings replace them through the manifest
//! (see `catalog`). Deterministic: the same key always gives the same sound.

const RATE: u32 = 22050;
const PEAK: f32 = 0.9;

#[derive(Clone, Copy)]
enum Wave {
    Sine,
    Tri,
    Square,
}

/// Amplitude envelope over a layer's duration.
#[derive(Clone, Copy)]
enum Env {
    /// Fast attack, then exponential decay at this rate (1/s).
    Decay(f32),
    /// Rises and falls (half a sine): whooshes and sweeps.
    Swell,
}

impl Env {
    fn at(self, t: f32, dur: f32) -> f32 {
        let attack = (t / 0.002).min(1.0);
        let release = ((dur - t) / 0.004).clamp(0.0, 1.0);
        let shape = match self {
            Env::Decay(k) => (-t * k).exp(),
            Env::Swell => (std::f32::consts::PI * t / dur).sin(),
        };
        attack * release * shape
    }
}

struct Clip {
    s: Vec<f32>,
    seed: u32,
}

impl Clip {
    fn new(secs: f32) -> Self {
        Clip { s: vec![0.0; (secs * RATE as f32) as usize], seed: 0x9E37_79B9 }
    }

    /// White noise in -1..1 (xorshift32).
    fn rand(&mut self) -> f32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// A tone gliding exponentially from `f0` to `f1` Hz.
    fn tone(&mut self, at: f32, dur: f32, f0: f32, f1: f32, amp: f32, env: Env, wave: Wave) -> &mut Self {
        let start = (at * RATE as f32) as usize;
        let n = (dur * RATE as f32) as usize;
        let mut phase = 0.0f32;
        for i in 0..n {
            let Some(out) = self.s.get_mut(start + i) else { break };
            let t = i as f32 / RATE as f32;
            let f = f0 * (f1 / f0).powf(t / dur);
            phase = (phase + f / RATE as f32).fract();
            let v = match wave {
                Wave::Sine => (phase * std::f32::consts::TAU).sin(),
                Wave::Tri => 1.0 - 4.0 * (phase - 0.5).abs(),
                Wave::Square => {
                    if phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
            };
            *out += v * amp * env.at(t, dur);
        }
        self
    }

    /// Filtered noise: one-pole low-pass at `lp` Hz, then a one-pole
    /// high-pass at `hp` Hz (0 skips it).
    fn noise(&mut self, at: f32, dur: f32, amp: f32, env: Env, lp: f32, hp: f32) -> &mut Self {
        let start = (at * RATE as f32) as usize;
        let n = (dur * RATE as f32) as usize;
        let a_lp = 1.0 - (-std::f32::consts::TAU * lp / RATE as f32).exp();
        let a_hp = 1.0 - (-std::f32::consts::TAU * hp / RATE as f32).exp();
        let (mut low, mut hp_low) = (0.0f32, 0.0f32);
        for i in 0..n {
            if start + i >= self.s.len() {
                break;
            }
            let t = i as f32 / RATE as f32;
            low += a_lp * (self.rand() - low);
            let mut v = low;
            if hp > 0.0 {
                hp_low += a_hp * (v - hp_low);
                v -= hp_low;
            }
            self.s[start + i] += v * amp * env.at(t, dur);
        }
        self
    }

    /// A gun report: a bright crack, a body of filtered noise and a low thump.
    fn shot(&mut self, lp: f32, dur: f32, decay: f32, thump_hz: f32, thump: f32, crack: f32) -> &mut Self {
        self.noise(0.0, 0.015, crack, Env::Decay(200.0), 9000.0, 2000.0)
            .noise(0.0, dur, 1.0, Env::Decay(decay), lp, 80.0)
            .tone(0.0, dur * 0.6, thump_hz, thump_hz * 0.5, thump, Env::Decay(decay * 1.2), Wave::Sine)
    }

    /// Short notes one after another.
    fn notes(&mut self, notes: &[(f32, f32)], dur: f32, amp: f32, decay: f32) -> &mut Self {
        for &(at, hz) in notes {
            self.tone(at, dur, hz, hz, amp, Env::Decay(decay), Wave::Tri);
        }
        self
    }

    /// Normalise to a common peak and encode as WAV.
    fn finish(&self) -> Vec<u8> {
        let peak = self.s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        let gain = if peak > 0.0 { PEAK / peak } else { 0.0 };
        wav(self.s.iter().map(|v| (v * gain).clamp(-1.0, 1.0)), self.s.len())
    }
}

/// 16-bit mono PCM WAV.
fn wav(samples: impl Iterator<Item = f32>, count: usize) -> Vec<u8> {
    let data_len = (count * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&RATE.to_le_bytes());
    b.extend_from_slice(&(RATE * 2).to_le_bytes()); // byte rate
    b.extend_from_slice(&2u16.to_le_bytes()); // block align
    b.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for v in samples {
        b.extend_from_slice(&((v * i16::MAX as f32) as i16).to_le_bytes());
    }
    b
}

/// The placeholder for `key`, or `None` for a key without a recipe.
pub(super) fn placeholder(key: &str) -> Option<Vec<u8>> {
    use Env::*;
    use Wave::*;
    let clip = match key {
        // ── Weapons ──
        "pistol_shot" => Clip::new(0.2).shot(4200.0, 0.18, 28.0, 140.0, 0.7, 0.6).finish(),
        "revolver_shot" => Clip::new(0.32).shot(3200.0, 0.30, 16.0, 110.0, 0.9, 0.7).finish(),
        "deagle_shot" => Clip::new(0.36).shot(3000.0, 0.34, 14.0, 95.0, 1.0, 0.8).finish(),
        "smg_shot" => Clip::new(0.13).shot(5000.0, 0.12, 38.0, 170.0, 0.5, 0.5).finish(),
        "ar_shot" => Clip::new(0.2).shot(3800.0, 0.18, 26.0, 120.0, 0.7, 0.6).finish(),
        "shotgun_shot" => Clip::new(0.44).shot(2200.0, 0.42, 11.0, 80.0, 1.0, 0.5).finish(),
        "sniper_shot" => Clip::new(0.9)
            .shot(2600.0, 0.6, 8.0, 70.0, 1.0, 1.0)
            .noise(0.1, 0.8, 0.3, Decay(4.0), 900.0, 60.0) // the echo down the corridors
            .finish(),
        "knife_slash" => Clip::new(0.18).noise(0.0, 0.18, 1.0, Swell, 3500.0, 600.0).finish(),
        "dry_fire" => Clip::new(0.06)
            .noise(0.0, 0.012, 1.0, Decay(300.0), 9000.0, 2500.0)
            .tone(0.0, 0.02, 2400.0, 2400.0, 0.3, Decay(200.0), Square)
            .noise(0.035, 0.012, 0.7, Decay(300.0), 9000.0, 2500.0)
            .finish(),
        "reload_start" => Clip::new(0.2)
            .noise(0.0, 0.03, 1.0, Decay(120.0), 6000.0, 1500.0)
            .tone(0.0, 0.05, 900.0, 700.0, 0.4, Decay(80.0), Tri)
            .noise(0.12, 0.05, 0.6, Decay(90.0), 3000.0, 400.0)
            .finish(),
        "reload_end" => Clip::new(0.26)
            .noise(0.0, 0.03, 0.9, Decay(120.0), 5000.0, 800.0)
            .tone(0.0, 0.06, 1800.0, 1800.0, 0.4, Decay(60.0), Tri)
            .noise(0.14, 0.03, 1.0, Decay(150.0), 7000.0, 2000.0)
            .tone(0.14, 0.1, 1300.0, 1250.0, 0.35, Decay(50.0), Tri)
            .finish(),
        "weapon_switch" => Clip::new(0.14)
            .noise(0.0, 0.03, 0.6, Decay(150.0), 5000.0, 800.0)
            .noise(0.05, 0.03, 0.4, Decay(150.0), 5000.0, 800.0)
            .noise(0.09, 0.04, 0.8, Decay(130.0), 4000.0, 600.0)
            .finish(),
        "wall_impact" => Clip::new(0.07)
            .noise(0.0, 0.06, 1.0, Decay(60.0), 3000.0, 400.0)
            .tone(0.0, 0.04, 220.0, 160.0, 0.4, Decay(80.0), Sine)
            .finish(),
        "wall_break" => {
            let mut c = Clip::new(0.75);
            c.tone(0.0, 0.4, 70.0, 40.0, 1.0, Decay(10.0), Sine)
                .noise(0.0, 0.7, 0.8, Decay(7.0), 1500.0, 60.0);
            // falling debris: grains that thin out
            for i in 0..12 {
                let jitter = (c.rand() + 1.0) * 0.015;
                let at = 0.04 * i as f32 + jitter;
                c.noise(at, 0.03, 0.5 * (1.0 - i as f32 / 12.0), Decay(100.0), 4000.0, 700.0);
            }
            c.finish()
        }

        // ── Movement ──
        "footstep" => Clip::new(0.08)
            .noise(0.0, 0.07, 1.0, Decay(55.0), 1400.0, 60.0)
            .tone(0.0, 0.06, 90.0, 60.0, 0.8, Decay(60.0), Sine)
            .finish(),
        "slide" => Clip::new(0.45).noise(0.0, 0.45, 0.7, Swell, 2000.0, 300.0).finish(),
        "jump" => Clip::new(0.14)
            .noise(0.0, 0.12, 0.6, Swell, 900.0, 100.0)
            .noise(0.0, 0.04, 0.5, Decay(70.0), 1400.0, 60.0)
            .finish(),
        "land" => Clip::new(0.14)
            .tone(0.0, 0.12, 110.0, 55.0, 1.0, Decay(30.0), Sine)
            .noise(0.0, 0.1, 0.8, Decay(40.0), 1200.0, 50.0)
            .finish(),

        // ── Combat feedback ──
        "hit_marker" => Clip::new(0.06)
            .tone(0.0, 0.05, 2100.0, 2000.0, 1.0, Decay(70.0), Tri)
            .noise(0.0, 0.006, 0.4, Decay(400.0), 9000.0, 3000.0)
            .finish(),
        "headshot" => Clip::new(0.12)
            .tone(0.0, 0.1, 3000.0, 2900.0, 1.0, Decay(40.0), Sine)
            .tone(0.0, 0.1, 4500.0, 4400.0, 0.4, Decay(45.0), Sine)
            .noise(0.0, 0.006, 0.4, Decay(400.0), 9000.0, 3000.0)
            .finish(),
        "kill_confirm" => Clip::new(0.26).notes(&[(0.0, 880.0), (0.08, 1320.0)], 0.16, 0.9, 18.0).finish(),
        "hurt" => Clip::new(0.16)
            .tone(0.0, 0.15, 160.0, 90.0, 1.0, Decay(22.0), Sine)
            .noise(0.0, 0.12, 0.6, Decay(30.0), 900.0, 60.0)
            .finish(),
        "death" => Clip::new(0.75)
            .tone(0.0, 0.7, 420.0, 70.0, 0.9, Decay(4.0), Tri)
            .noise(0.0, 0.4, 0.4, Decay(8.0), 700.0, 40.0)
            .finish(),
        "respawn" => Clip::new(0.36)
            .tone(0.0, 0.35, 330.0, 880.0, 0.6, Swell, Sine)
            .tone(0.0, 0.35, 495.0, 1320.0, 0.3, Swell, Sine)
            .finish(),

        // ── Match flow ──
        "teleport" => Clip::new(0.56)
            .tone(0.0, 0.55, 200.0, 1600.0, 0.6, Swell, Sine)
            .tone(0.0, 0.55, 203.0, 1620.0, 0.4, Swell, Tri)
            .noise(0.0, 0.55, 0.15, Swell, 6000.0, 2000.0)
            .finish(),
        "countdown_tick" => Clip::new(0.13).tone(0.0, 0.12, 660.0, 660.0, 0.8, Decay(18.0), Sine).finish(),
        "match_start" => Clip::new(0.42)
            .tone(0.0, 0.4, 990.0, 990.0, 0.9, Decay(6.0), Sine)
            .tone(0.0, 0.4, 1485.0, 1485.0, 0.3, Decay(6.0), Sine)
            .finish(),
        "match_end" => Clip::new(0.7)
            .notes(&[(0.0, 523.0), (0.12, 659.0), (0.24, 784.0), (0.36, 1047.0)], 0.3, 0.8, 8.0)
            .finish(),
        "level_up" => Clip::new(0.3).notes(&[(0.0, 660.0), (0.07, 880.0), (0.14, 1320.0)], 0.14, 0.8, 20.0).finish(),
        "chat" => Clip::new(0.08).tone(0.0, 0.07, 1200.0, 1500.0, 0.7, Decay(40.0), Sine).finish(),
        _ => return None,
    };
    Some(clip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalogue_key_has_a_placeholder() {
        for &key in super::super::catalog::ALL_KEYS {
            let wav = placeholder(key).unwrap_or_else(|| panic!("no placeholder for {key}"));
            assert!(wav.len() > 44 + 200, "{key} is nearly empty");
            assert_eq!(&wav[0..4], b"RIFF");
            let samples: Vec<i16> =
                wav[44..].chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
            assert!(samples.iter().any(|s| s.unsigned_abs() > 10_000), "{key} is silent");
        }
    }

    #[test]
    fn every_weapon_shot_key_is_in_the_catalogue() {
        for id in crate::weapons::stats::WeaponId::ALL {
            let key = id.stats().fire_sound_asset_key;
            assert!(
                super::super::catalog::ALL_KEYS.contains(&key),
                "{key} (fired by {id:?}) has no placeholder"
            );
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn placeholders_decode() {
        for &key in super::super::catalog::ALL_KEYS {
            let wav = placeholder(key).unwrap();
            rodio::Decoder::new(std::io::Cursor::new(wav)).unwrap_or_else(|e| panic!("{key}: {e}"));
        }
    }
}
