use client::screens::PlayingState;
use client::types::animated_model::{import_glb, AnimatedBuild, AnimatedModel, ClipSelectionMode, GlbImages};
use macroquad::models::Mesh;
use macroquad::prelude::warn;

use crate::app::obj_loader::load_gltf_static_mesh;

/// A GLB model or character loaded a step at a time, so a big one is not one
/// long frame: [`start`](GlbJob::start) reads and checks the file, then each
/// [`step`](GlbJob::step) uploads one of its textures or does one stage of
/// the build, and the last one stores the result in the playing state. Nothing
/// of it is stored before then, so nothing draws or hits a half-built model.
/// Steps run back to back give what loading it in one go gave.
pub(super) struct GlbJob {
    key: String,
    /// The manifest's path, stored with the asset.
    path: String,
    /// Where the file was read from.
    resolved: String,
    doc: gltf::Document,
    buffers: Vec<gltf::buffer::Data>,
    images: GlbImages,
    /// Images to upload ahead of the build, a step each, the next one last.
    textures: Vec<usize>,
    state: State,
}

enum State {
    /// A character being built; added to the characters when done.
    Character(AnimatedBuild),
    /// A model being built by the animated loader.
    Animated(AnimatedBuild),
    /// Built with textures: uploaded and stored next.
    Ready(AnimatedModel),
    /// A character built: added next.
    ReadyCharacter(AnimatedModel),
    /// Built without a texture: the static loader is tried next, and this
    /// kept if it fails.
    Textureless(AnimatedModel),
    /// The animated loader failed (`Some`: why) or cannot work on the file
    /// (`None`): the static loader next.
    Static(Option<String>),
    Done,
}

impl GlbJob {
    /// Import `bytes` (the file at `resolved`, manifest entry `key`, `path`)
    /// and check what it can be loaded as. `None` if that failed (logged).
    pub fn start(key: &str, path: &str, resolved: &str, bytes: &[u8]) -> Option<GlbJob> {
        let character = key.starts_with("character_");
        let (doc, buffers, images) = match import_glb(resolved, bytes) {
            Ok(glb) => glb,
            Err(e) if character => {
                warn!("Failed to load character {} from {}: {}", key, path, e);
                return None;
            }
            // both loaders fail on it, the same way
            Err(e) => {
                warn!("Failed GLTF load {} from {}. animated_err={:?} static_err={:?}", key, path, e, e);
                return None;
            }
        };
        // The weapon GLBs have no animation (some no skin either), so the
        // animated loader can only fail on them. Skip it then; its error is
        // only wanted if the static loader fails too, so it runs then.
        let keep_unskinned = keep_unskinned_primitives(key);
        let build = |doc: &gltf::Document, buffers: &[gltf::buffer::Data]| {
            AnimatedBuild::start(resolved, doc, buffers, ClipSelectionMode::LongestDuration, None, keep_unskinned)
        };
        let state = if character {
            match build(&doc, &buffers) {
                Ok(build) => State::Character(build),
                Err(e) => {
                    warn!("Failed to load character {} from {}: {}", key, path, e);
                    return None;
                }
            }
        } else if animated_load_cannot_succeed(bytes) {
            State::Static(None)
        } else {
            match build(&doc, &buffers) {
                Ok(build) => State::Animated(build),
                Err(e) => State::Static(Some(e)),
            }
        };
        let mut textures = match &state {
            State::Character(build) | State::Animated(build) => build.texture_images(&doc, &buffers),
            _ => Vec::new(),
        };
        textures.reverse();
        let (key, path, resolved) = (key.to_string(), path.to_string(), resolved.to_string());
        Some(GlbJob { key, path, resolved, doc, buffers, images, textures, state })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    /// Do the next step. True once the job is over: the asset stored, or
    /// failed (logged).
    pub fn step(&mut self, playing_state: &mut PlayingState) -> bool {
        let (doc, buffers, images) = (&self.doc, &self.buffers[..], &self.images);
        if let Some(index) = self.textures.pop() {
            images.texture(buffers, index);
            return false;
        }
        let (key, path) = (self.key.clone(), self.path.clone());
        let next = match std::mem::replace(&mut self.state, State::Done) {
            State::Character(mut build) => match build.step(doc, buffers, images) {
                Ok(None) => State::Character(build),
                Ok(Some(model)) => State::ReadyCharacter(model),
                Err(e) => {
                    warn!("Failed to load character {} from {}: {}", key, path, e);
                    State::Done
                }
            },
            State::Animated(mut build) => match build.step(doc, buffers, images) {
                Ok(None) => State::Animated(build),
                // animated model produced but no texture; try static loader
                Ok(Some(model)) if !model.has_any_texture() => State::Textureless(model),
                Ok(Some(model)) => State::Ready(model),
                Err(e) => State::Static(Some(e)),
            },
            State::ReadyCharacter(model) => {
                playing_state.asset_source_paths.insert(key.clone(), path.clone());
                println!("Loaded character: {} from {}", key, path);
                playing_state.add_character(key, model);
                State::Done
            }
            State::Ready(animated) => {
                let vertex_count = animated.total_vertex_count();
                store_animated(playing_state, &key, &path, animated);
                println!("Loaded animated GLTF model: {} ({} verts) from {}", key, vertex_count, path);
                State::Done
            }
            State::Textureless(animated) => {
                match load_gltf_static_mesh(doc, buffers, images) {
                    Ok(meshes) => store_static(playing_state, &key, &path, meshes, " (anim fallback)"),
                    Err(static_err) => {
                        // static path failed; keep the textureless animated mesh
                        let vertex_count = animated.total_vertex_count();
                        store_animated(playing_state, &key, &path, animated);
                        println!(
                            "Loaded animated GLTF model (no texture, static fallback failed): {} ({} verts) from {}. Error is {:?}",
                            key, vertex_count, path, static_err
                        );
                    }
                }
                State::Done
            }
            State::Static(anim_err) => {
                match load_gltf_static_mesh(doc, buffers, images) {
                    // if no primitive had a texture, the loader bakes in
                    // lighting to give the user some built‑in shading
                    Ok(meshes) => store_static(playing_state, &key, &path, meshes, ""),
                    Err(static_err) => {
                        // skipped: the animated loader fails for sure, run it for its error
                        let anim_err = anim_err.unwrap_or_else(|| match self.build_now() {
                            Err(e) => e,
                            Ok(_) => String::from("animated loader skipped"),
                        });
                        warn!(
                            "Failed GLTF load {} from {}. animated_err={:?} static_err={:?}",
                            key, path, anim_err, static_err
                        );
                    }
                }
                State::Done
            }
            State::Done => State::Done,
        };
        self.state = next;
        matches!(self.state, State::Done)
    }

    /// The animated loader run on the file in one go.
    fn build_now(&self) -> Result<AnimatedModel, String> {
        let (doc, buffers, images) = (&self.doc, &self.buffers[..], &self.images);
        let mode = ClipSelectionMode::LongestDuration;
        let keep_unskinned = keep_unskinned_primitives(&self.key);
        let mut build = AnimatedBuild::start(&self.resolved, doc, buffers, mode, None, keep_unskinned)?;
        loop {
            if let Some(model) = build.step(doc, buffers, images)? {
                return Ok(model);
            }
        }
    }
}

/// Whether the animated loader keeps `key`'s unskinned primitives too. Some
/// static weapon GLBs are exported with a skin/animation placeholder.
fn keep_unskinned_primitives(key: &str) -> bool {
    key == "fp_hands_minigun"
}

fn store_animated(playing_state: &mut PlayingState, key: &str, path: &str, animated: AnimatedModel) {
    animated.prepare_gpu();
    playing_state.animated_models.insert(key.to_string(), animated);
    playing_state.asset_source_paths.insert(key.to_string(), path.to_string());
}

/// `meshes` as model `key`; `how` goes in the log line.
fn store_static(playing_state: &mut PlayingState, key: &str, path: &str, meshes: Vec<Mesh>, how: &str) {
    let total_verts: usize = meshes.iter().map(|m| m.vertices.len()).sum();
    // a key loaded again (another path for it, or a new session) replaces
    // its meshes: appending stacked two guns into one and grew every session
    let entry = playing_state.models.entry(key.to_string()).or_default();
    entry.clear();
    entry.extend(meshes);
    let sub_meshes = entry.len();
    client::types::static_gpu::forget(key);
    playing_state.asset_source_paths.insert(key.to_string(), path.to_string());
    println!(
        "Loaded static GLTF model{}: {} ({} verts across {} sub-meshes) from {}",
        how, key, total_verts, sub_meshes, path
    );
}

/// Whether the animated loader is sure to fail on this glTF: it needs a skin
/// (`no skin found`) and an animation (`no usable animation clip`). Reads only
/// the JSON part, without importing the file. `false` when unsure (the JSON
/// does not parse), so the animated loader still gets its try.
fn animated_load_cannot_succeed(bytes: &[u8]) -> bool {
    use serde::de::IgnoredAny;

    #[derive(serde::Deserialize)]
    struct SkinsAndAnimations {
        #[serde(default)]
        skins: Vec<IgnoredAny>,
        #[serde(default)]
        animations: Vec<IgnoredAny>,
    }

    // as gltf::Gltf::from_slice tells a binary .glb from a JSON .gltf
    let json = if bytes.starts_with(b"glTF") {
        match gltf::Glb::from_slice(bytes) {
            Ok(glb) => glb.json,
            Err(_) => return false,
        }
    } else {
        std::borrow::Cow::Borrowed(bytes)
    };
    match serde_json::from_slice::<SkinsAndAnimations>(&json) {
        Ok(doc) => doc.skins.is_empty() || doc.animations.is_empty(),
        Err(_) => false,
    }
}

/// A .glb holding `json` and no binary chunk.
#[cfg(test)]
pub(super) fn test_glb(json: &str) -> Vec<u8> {
    let mut json = json.as_bytes().to_vec();
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(12 + 8 + json.len() as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json);
    out
}

#[cfg(test)]
mod tests {
    use super::{animated_load_cannot_succeed, test_glb as glb, GlbJob, State};

    /// A character is built as one or not at all; another model goes to the
    /// static loader when the animated one fails (with its error) or cannot
    /// work; a file that does not import is dropped.
    #[test]
    fn start_picks_the_loader() {
        let empty = glb(r#"{"asset":{"version":"2.0"}}"#);
        assert!(GlbJob::start("character_x", "x.glb", "x.glb", &empty).is_none());
        let job = GlbJob::start("x_model", "x.glb", "r/x.glb", &empty).unwrap();
        assert!(matches!(job.state, State::Static(None)));
        assert!(job.textures.is_empty());
        assert_eq!((job.key(), job.path.as_str(), job.resolved.as_str()), ("x_model", "x.glb", "r/x.glb"));
        let no_mesh = glb(r#"{"asset":{"version":"2.0"},"nodes":[{}],"skins":[{"joints":[0]}],
            "animations":[{"channels":[],"samplers":[]}]}"#);
        let job = GlbJob::start("x_model", "x.glb", "x.glb", &no_mesh).unwrap();
        assert!(matches!(&job.state, State::Static(Some(e)) if e.starts_with("no skinned primitive")));
        assert!(GlbJob::start("x_model", "x.glb", "x.glb", b"glTF garbage").is_none());
        assert!(GlbJob::start("character_x", "x.glb", "x.glb", b"glTF garbage").is_none());
    }

    #[test]
    fn skips_animated_loader_only_without_skin_or_animation() {
        let asset = r#""asset":{"version":"2.0"}"#;
        let skin = r#""skins":[{"joints":[0]}]"#;
        let anim = r#""animations":[{"channels":[],"samplers":[]}]"#;
        assert!(animated_load_cannot_succeed(&glb(&format!("{{{asset}}}"))));
        assert!(animated_load_cannot_succeed(&glb(&format!("{{{asset},{skin}}}"))));
        assert!(animated_load_cannot_succeed(&glb(&format!("{{{asset},{anim}}}"))));
        assert!(animated_load_cannot_succeed(&glb(&format!(r#"{{{asset},"skins":[],{anim}}}"#))));
        assert!(!animated_load_cannot_succeed(&glb(&format!("{{{asset},{skin},{anim}}}"))));
        // a JSON .gltf
        assert!(animated_load_cannot_succeed(format!("{{{asset},{skin}}}").as_bytes()));
        assert!(!animated_load_cannot_succeed(format!("{{{asset},{skin},{anim}}}").as_bytes()));
        // unsure: let the animated loader try
        assert!(!animated_load_cannot_succeed(b"glTF garbage"));
        assert!(!animated_load_cannot_succeed(b"not json"));
        assert!(!animated_load_cannot_succeed(&glb(&format!(r#"{{{asset},"skins":7,{anim}}}"#))));
    }

    /// The game's own GLBs (`client/assets` is not in git; point
    /// `GRIDLOCK_ASSETS_DIR` at a copy, or it looks in `assets`): every file
    /// the animated loader is skipped for, it fails on.
    #[test]
    fn skipped_glbs_fail_the_animated_loader() {
        use client::types::animated_model::{
            load_gltf_animated_model, load_gltf_animated_model_with_unskinned,
        };
        let root = std::env::var("GRIDLOCK_ASSETS_DIR").unwrap_or_else(|_| "assets".to_string());
        let mut dirs = vec![std::path::PathBuf::from(root).join("models")];
        let mut checked = 0;
        while let Some(dir) = dirs.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for path in entries.flatten().map(|e| e.path()) {
                if path.is_dir() {
                    dirs.push(path);
                    continue;
                }
                if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("glb")) {
                    continue;
                }
                let bytes = std::fs::read(&path).unwrap();
                if !animated_load_cannot_succeed(&bytes) {
                    continue;
                }
                let name = path.to_string_lossy();
                assert!(load_gltf_animated_model(&name, &bytes).is_err(), "{name}");
                assert!(load_gltf_animated_model_with_unskinned(&name, &bytes, true).is_err(), "{name}");
                checked += 1;
            }
        }
        println!("{checked} GLBs skip the animated loader");
        // asked to check a folder explicitly: an empty result is a failure
        if std::env::var_os("GRIDLOCK_ASSETS_DIR").is_some() {
            assert!(checked > 0, "GRIDLOCK_ASSETS_DIR is set but no GLB there was checked");
        }
    }
}
