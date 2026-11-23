use macroquad::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;

/// Parse a self-contained `.glb` from memory. The bytes are read up front with
/// macroquad's `load_file`, which works on native builds and in the browser
/// alike; `path` is only for error messages. Images are left encoded until a
/// mesh asks for one (see [`GlbImages`]), in place in the buffers. Nothing
/// returned borrows `bytes`, so a parsed file can be kept across frames.
pub fn import_glb(
    path: &str,
    bytes: &[u8],
) -> Result<(gltf::Document, Vec<gltf::buffer::Data>, GlbImages), String> {
    let fail = |e: gltf::Error| format!("failed to import gltf '{}': {e}", path);
    let (document, bin) = parse(bytes).map_err(fail)?;
    let buffers = gltf::import_buffers(&document, None, bin.map(Cow::into_owned)).map_err(fail)?;
    // Only images stored in a buffer are supported; the game's GLBs embed all
    // of theirs in the binary chunk. Those are not copied: they stay where
    // they are in the buffers.
    let encoded = document
        .images()
        .map(|image| match image.source() {
            gltf::image::Source::View { view, .. } => {
                let range = view.offset()..view.offset() + view.length();
                let buffer = view.buffer().index();
                buffers.get(buffer).and_then(|data| data.get(range.clone())).map(|_| (buffer, range))
            }
            gltf::image::Source::Uri { .. } => None,
        })
        .collect::<Vec<_>>();
    let keys = encoded.iter().map(|_| std::cell::OnceCell::new()).collect();
    #[cfg(target_arch = "wasm32")]
    let images = GlbImages { encoded, keys, path: path.to_string() };
    #[cfg(not(target_arch = "wasm32"))]
    let images = GlbImages { encoded, keys };
    Ok((document, buffers, images))
}

/// Parse glTF as `gltf::Gltf::from_slice` does (the document validated), but
/// with a .glb's binary chunk borrowed from `bytes` instead of copied.
fn parse(bytes: &[u8]) -> gltf::Result<(gltf::Document, Option<Cow<'_, [u8]>>)> {
    if !bytes.starts_with(b"glTF") {
        return gltf::Gltf::from_slice(bytes).map(|gltf| (gltf.document, gltf.blob.map(Cow::Owned)));
    }
    let glb = gltf::Glb::from_slice(bytes)?;
    let document = gltf::Document::from_json(gltf::json::deserialize::from_slice(&glb.json)?)?;
    Ok((document, glb.bin))
}

/// A GLB's embedded images (PNG or JPEG), decoded when a mesh uses one.
/// Decoded textures are shared by content: the player's animation files each
/// embed the same textures, and decoding and uploading those once instead of
/// once per file saves most of the load time and GPU memory.
pub struct GlbImages {
    /// Where each image's encoded bytes are in the buffers imported with it:
    /// (buffer, byte range).
    encoded: Vec<Option<(usize, std::ops::Range<usize>)>>,
    /// Each image's content key, hashed the first time a mesh asks for it:
    /// a rig's primitives share a few big images, and hashing all of one's
    /// bytes again for every primitive cost milliseconds per rig.
    keys: Vec<std::cell::OnceCell<u64>>,
    /// The file they came from, by which the browser's worker hands over
    /// the images it decoded.
    #[cfg(target_arch = "wasm32")]
    path: String,
}

thread_local! {
    /// Textures by a hash of their encoded bytes; `None` if one failed to decode.
    static TEXTURES: std::cell::RefCell<HashMap<u64, Option<Texture2D>>> =
        std::cell::RefCell::new(HashMap::new());
}

impl GlbImages {
    /// The image at `index` as a texture (nearest filtering), decoded at most
    /// once per distinct image. `buffers` are the ones imported with it.
    pub fn texture(&self, buffers: &[gltf::buffer::Data], index: usize) -> Option<Texture2D> {
        let bytes = self.encoded(buffers, index)?;
        let key = *self.keys[index].get_or_init(|| content_key(bytes));
        TEXTURES.with(|textures| {
            textures.borrow_mut().entry(key).or_insert_with(|| self.decode_texture(key, index, bytes)).clone()
        })
    }

    /// Image `index`'s encoded bytes, in `buffers`.
    fn encoded<'b>(&self, buffers: &'b [gltf::buffer::Data], index: usize) -> Option<&'b [u8]> {
        let (buffer, range) = self.encoded.get(index)?.as_ref()?;
        buffers.get(*buffer)?.get(range.clone())
    }
}

fn content_key(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    // `DefaultHasher::new` has fixed keys: the same bytes give the same key
    // on every thread.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Images a worker thread decoded ahead of the loader (native builds), so the
/// main thread only uploads them. Keyed like `TEXTURES`.
#[cfg(not(target_arch = "wasm32"))]
mod predecoded {
    use std::collections::HashMap;
    use std::sync::{Condvar, Mutex};

    enum Slot {
        /// A worker is decoding it.
        Decoding,
        /// Decoded, waiting for the loader (`None`: not a valid image).
        Ready(Option<image::RgbaImage>),
        /// The loader has it (or decoded it itself): never decode it again.
        Taken,
    }

    static SLOTS: Mutex<Option<HashMap<u64, Slot>>> = Mutex::new(None);
    /// Signalled whenever a worker stops decoding an image.
    static DECODED: Condvar = Condvar::new();

    fn with<R>(f: impl FnOnce(&mut HashMap<u64, Slot>) -> R) -> R {
        let mut guard = SLOTS.lock().unwrap_or_else(|e| e.into_inner());
        f(guard.get_or_insert_with(HashMap::new))
    }

    /// A worker's claim on decoding `key`. Dropping it (also when the decode
    /// panicked) wakes whoever waits on that image; a claim that never got
    /// its result in leaves the image for the loader to decode.
    struct Claim(u64);

    impl Drop for Claim {
        fn drop(&mut self) {
            with(|slots| {
                if matches!(slots.get(&self.0), Some(Slot::Decoding)) {
                    slots.remove(&self.0);
                }
            });
            DECODED.notify_all();
        }
    }

    /// Decode `bytes` unless another worker or the loader already did.
    /// False if another worker is decoding it right now.
    pub fn decode(key: u64, bytes: &[u8]) -> bool {
        let claimed = with(|slots| match slots.get(&key) {
            Some(Slot::Decoding) => None,
            Some(_) => Some(false),
            None => {
                slots.insert(key, Slot::Decoding);
                Some(true)
            }
        });
        match claimed {
            None => return false,
            Some(false) => return true,
            Some(true) => {}
        }
        let _claim = Claim(key);
        let image = image::load_from_memory(bytes).ok().map(|i| i.to_rgba8());
        with(|slots| {
            if let Some(slot @ Slot::Decoding) = slots.get_mut(&key) {
                *slot = Slot::Ready(image);
            }
        });
        true
    }

    /// Wait until no worker is decoding any of `keys` any more. A worker that
    /// left an image to another one waits for it before its file counts as
    /// downloaded: the loader would otherwise find it still decoding and
    /// decode it again on the main thread. Call it only once done with the
    /// images claimed, so two workers never wait on each other.
    pub fn wait_for(keys: &[u64]) {
        let mut guard = SLOTS.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            let slots = guard.get_or_insert_with(HashMap::new);
            if !keys.iter().any(|key| matches!(slots.get(key), Some(Slot::Decoding))) {
                return;
            }
            guard = DECODED.wait(guard).unwrap_or_else(|e| e.into_inner());
        }
    }

    /// The worker's result if it is done (`Some(None)`: it failed to decode);
    /// `None` means decode it now. Either way no worker decodes it after this.
    pub fn take(key: u64) -> Option<Option<image::RgbaImage>> {
        with(|slots| match slots.insert(key, Slot::Taken) {
            Some(Slot::Ready(image)) => Some(image),
            _ => None,
        })
    }

    /// Drop decoded images nothing took; asked for later, they decode then.
    pub fn release() {
        with(|slots| {
            for slot in slots.values_mut() {
                if matches!(slot, Slot::Ready(_)) {
                    *slot = Slot::Taken;
                }
            }
        });
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::Duration;

        /// A worker whose file shares an image another worker is decoding
        /// waits for it, and the loader then takes that worker's result.
        #[test]
        fn a_worker_waits_for_an_image_another_is_decoding() {
            const KEY: u64 = 0xF076_F076_F076_0001;
            with(|slots| slots.insert(KEY, Slot::Decoding));
            let other = Claim(KEY);
            assert!(!decode(KEY, &[]), "left to the worker decoding it");
            let waiter = std::thread::spawn(|| wait_for(&[KEY]));
            std::thread::sleep(Duration::from_millis(50));
            assert!(!waiter.is_finished(), "waits while it is being decoded");
            with(|slots| slots.insert(KEY, Slot::Ready(None)));
            drop(other);
            waiter.join().unwrap();
            assert!(matches!(take(KEY), Some(None)), "the other worker's result");
        }

        /// A decode that never finished (it panicked) wakes the waiters and
        /// leaves the image for the loader to decode.
        #[test]
        fn an_unfinished_decode_leaves_the_image_to_the_loader() {
            const KEY: u64 = 0xF076_F076_F076_0002;
            with(|slots| slots.insert(KEY, Slot::Decoding));
            let other = Claim(KEY);
            let waiter = std::thread::spawn(|| wait_for(&[KEY]));
            drop(other);
            waiter.join().unwrap();
            assert!(take(KEY).is_none(), "decoded by the loader");
        }
    }
}

/// The images a material takes its colour from: the game samples no others
/// (`parts.rs`, `app/obj_loader.rs`), and a model often embeds big normal and
/// roughness maps too. Any other a mesh asks for is decoded then.
#[cfg(not(target_arch = "wasm32"))]
fn base_colour_images(document: &gltf::Document) -> std::collections::HashSet<usize> {
    document
        .materials()
        .flat_map(|material| {
            let base = material.pbr_metallic_roughness().base_color_texture();
            let diffuse = material.pbr_specular_glossiness().and_then(|sg| sg.diffuse_texture());
            let base = base.map(|info| info.texture().source().index());
            base.into_iter().chain(diffuse.map(|info| info.texture().source().index()))
        })
        .collect()
}

/// Decode a GLB's embedded images on the calling (worker) thread, ahead of
/// the loader: only those a mesh samples (see [`base_colour_images`]).
/// Native builds only: the browser build has no threads.
#[cfg(not(target_arch = "wasm32"))]
pub fn predecode_glb_images(bytes: &[u8]) {
    let Ok((document, Some(blob))) = parse(bytes) else {
        return;
    };
    let used = base_colour_images(&document);
    // Images another worker was decoding already (files often share them).
    let mut elsewhere = Vec::new();
    for image in document.images().filter(|image| used.contains(&image.index())) {
        let gltf::image::Source::View { view, .. } = image.source() else {
            continue;
        };
        if !matches!(view.buffer().source(), gltf::buffer::Source::Bin) {
            continue;
        }
        if let Some(encoded) = blob.get(view.offset()..view.offset() + view.length()) {
            let key = content_key(encoded);
            if !predecoded::decode(key, encoded) {
                elsewhere.push(key);
            }
        }
    }
    predecoded::wait_for(&elsewhere);
}

/// Forget images decoded ahead that nothing took (ones no mesh uses), once
/// the loader is done with the files they came from.
pub fn release_predecoded() {
    #[cfg(not(target_arch = "wasm32"))]
    predecoded::release();
    #[cfg(target_arch = "wasm32")]
    super::web_decoded::clear();
}

impl GlbImages {
    /// Decode image `index` (encoded as `bytes`), or take it from whoever
    /// decoded it ahead, and upload it.
    fn decode_texture(&self, _key: u64, _index: usize, bytes: &[u8]) -> Option<Texture2D> {
        #[cfg(not(target_arch = "wasm32"))]
        let image = match predecoded::take(_key) {
            Some(done) => done?,
            None => image::load_from_memory(bytes).ok()?.to_rgba8(),
        };
        #[cfg(target_arch = "wasm32")]
        let image = match super::web_decoded::take(&self.path, _index as u32)
            .and_then(|(w, h, rgba)| image::RgbaImage::from_raw(w, h, rgba))
        {
            Some(image) => image,
            None => image::load_from_memory(bytes).ok()?.to_rgba8(),
        };
        upload(image)
    }
}

fn upload(image: image::RgbaImage) -> Option<Texture2D> {
    let (width, height) = image.dimensions();
    if width > u16::MAX as u32 || height > u16::MAX as u32 {
        return None;
    }
    let texture = Texture2D::from_rgba8(width as u16, height as u16, &image);
    // smooth sampling for model textures, before this it was Nearest so close up models looked pixelated
    texture.set_filter(FilterMode::Linear);
    Some(texture)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    /// A worker's decode is what the loader then takes, and nothing decodes
    /// an image twice.
    #[test]
    fn predecoded_images_are_taken_once() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/hand/fps_pistol_animated.glb");
        let Ok(bytes) = std::fs::read(path) else { return };
        let (document, buffers, images) = import_glb(path, &bytes).unwrap();
        let used = base_colour_images(&document);
        let first = used.iter().copied().min().expect("fixture has a textured material");
        let encoded = images.encoded(&buffers, first).expect("embedded in the binary chunk");
        predecode_glb_images(&bytes);
        let key = content_key(encoded);
        let image = predecoded::take(key).expect("decoded by the worker").expect("a valid image");
        assert!(image.width() > 0);
        assert!(predecoded::take(key).is_none(), "taken only once");
        predecode_glb_images(&bytes);
        assert!(predecoded::take(key).is_none(), "not decoded again after it was taken");
    }

    /// Importing without copying the binary chunk or the images gives what
    /// gltf's own parser gives: the same document, buffers and image bytes.
    #[test]
    fn import_matches_gltf_from_slice() {
        let models = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models");
        let mut paths: Vec<std::path::PathBuf> = ["hand", "weapons/Pistols"]
            .iter()
            .filter_map(|dir| std::fs::read_dir(format!("{models}/{dir}")).ok())
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "glb"))
            .collect();
        paths.push(format!("{models}/animations/Crouch.glb").into());
        for path in paths {
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let name = path.display().to_string();
            let reference = gltf::Gltf::from_slice(&bytes).unwrap();
            let ref_buffers = gltf::import_buffers(&reference.document, None, reference.blob).unwrap();
            let (document, buffers, images) = import_glb(&name, &bytes).unwrap();
            let json = |d: &gltf::Document| gltf::json::serialize::to_string(d.as_json()).unwrap();
            assert_eq!(json(&document), json(&reference.document), "{name}");
            assert_eq!(buffers.len(), ref_buffers.len(), "{name}");
            for (a, b) in buffers.iter().zip(&ref_buffers) {
                assert!(a.0 == b.0, "{name}: buffer differs");
            }
            assert_eq!(images.encoded.len(), reference.document.images().count(), "{name}");
            for image in reference.document.images() {
                let gltf::image::Source::View { view, .. } = image.source() else { continue };
                let expected = ref_buffers[view.buffer().index()].get(view.offset()..view.offset() + view.length());
                let got = images.encoded(&buffers, image.index());
                assert!(got == expected, "{name}: image {} differs", image.index());
            }
        }
    }

    /// Only the images a material takes its colour from are decoded ahead:
    /// the carbine rig also embeds normal and roughness maps no mesh samples.
    #[test]
    fn only_base_colour_images_are_predecoded() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/hand/fps_animated_carbine.glb");
        let Ok(bytes) = std::fs::read(path) else { return };
        let (document, buffers, images) = import_glb(path, &bytes).unwrap();
        let used = base_colour_images(&document);
        assert!(!used.is_empty() && used.len() < images.encoded.len(), "fixture has unused maps");
        for material in document.materials() {
            if let Some(info) = material.pbr_metallic_roughness().base_color_texture() {
                assert!(used.contains(&info.texture().source().index()));
            }
        }
        predecode_glb_images(&bytes);
        for index in 0..images.encoded.len() {
            let Some(encoded) = images.encoded(&buffers, index) else { continue };
            let key = content_key(encoded);
            if !used.contains(&index) && !used.iter().any(|&u| images.encoded(&buffers, u) == Some(encoded)) {
                assert!(predecoded::take(key).is_none(), "image {index} is no material's colour");
            }
        }
    }
}
