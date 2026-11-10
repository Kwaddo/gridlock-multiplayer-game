use macroquad::prelude::*;
use std::collections::HashMap;

/// Parse a self-contained `.glb` from memory. The bytes are read up front with
/// macroquad's `load_file`, which works on native builds and in the browser
/// alike; `path` is only for error messages. Images are left encoded until a
/// mesh asks for one (see [`GlbImages`]).
pub fn import_glb(
    path: &str,
    bytes: &[u8],
) -> Result<(gltf::Document, Vec<gltf::buffer::Data>, GlbImages), String> {
    let fail = |e: gltf::Error| format!("failed to import gltf '{}': {e}", path);
    let gltf::Gltf { document, blob } = gltf::Gltf::from_slice(bytes).map_err(fail)?;
    let buffers = gltf::import_buffers(&document, None, blob).map_err(fail)?;
    // Only images stored in the binary chunk are supported; the game's GLBs
    // embed all of theirs that way.
    let encoded = document
        .images()
        .map(|image| match image.source() {
            gltf::image::Source::View { view, .. } => {
                let data = &buffers[view.buffer().index()];
                data.get(view.offset()..view.offset() + view.length()).map(<[u8]>::to_vec)
            }
            gltf::image::Source::Uri { .. } => None,
        })
        .collect();
    #[cfg(target_arch = "wasm32")]
    let images = GlbImages { encoded, path: path.to_string() };
    #[cfg(not(target_arch = "wasm32"))]
    let images = GlbImages { encoded };
    Ok((document, buffers, images))
}

/// A GLB's embedded images (PNG or JPEG), decoded when a mesh uses one.
/// Decoded textures are shared by content: the player's animation files each
/// embed the same textures, and decoding and uploading those once instead of
/// once per file saves most of the load time and GPU memory.
pub struct GlbImages {
    encoded: Vec<Option<Vec<u8>>>,
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
    /// once per distinct image.
    pub fn texture(&self, index: usize) -> Option<Texture2D> {
        let bytes = self.encoded.get(index)?.as_ref()?;
        let key = content_key(bytes);
        TEXTURES.with(|textures| {
            textures.borrow_mut().entry(key).or_insert_with(|| self.decode_texture(key, index, bytes)).clone()
        })
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
    use std::sync::Mutex;

    enum Slot {
        /// A worker is decoding it.
        Decoding,
        /// Decoded, waiting for the loader (`None`: not a valid image).
        Ready(Option<image::RgbaImage>),
        /// The loader has it (or decoded it itself): never decode it again.
        Taken,
    }

    static SLOTS: Mutex<Option<HashMap<u64, Slot>>> = Mutex::new(None);

    fn with<R>(f: impl FnOnce(&mut HashMap<u64, Slot>) -> R) -> R {
        let mut guard = SLOTS.lock().unwrap_or_else(|e| e.into_inner());
        f(guard.get_or_insert_with(HashMap::new))
    }

    /// Decode `bytes` unless another worker or the loader already did.
    pub fn decode(key: u64, bytes: &[u8]) {
        let claimed = with(|slots| match slots.contains_key(&key) {
            true => false,
            false => {
                slots.insert(key, Slot::Decoding);
                true
            }
        });
        if !claimed {
            return;
        }
        let image = image::load_from_memory(bytes).ok().map(|i| i.to_rgba8());
        with(|slots| {
            if let Some(slot @ Slot::Decoding) = slots.get_mut(&key) {
                *slot = Slot::Ready(image);
            }
        });
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
}

/// Decode a GLB's embedded images on the calling (worker) thread, ahead of
/// the loader. Native builds only: the browser build has no threads.
#[cfg(not(target_arch = "wasm32"))]
pub fn predecode_glb_images(bytes: &[u8]) {
    let Ok(gltf::Gltf { document, blob }) = gltf::Gltf::from_slice(bytes) else {
        return;
    };
    let Some(blob) = blob else {
        return;
    };
    for image in document.images() {
        let gltf::image::Source::View { view, .. } = image.source() else {
            continue;
        };
        if !matches!(view.buffer().source(), gltf::buffer::Source::Bin) {
            continue;
        }
        if let Some(encoded) = blob.get(view.offset()..view.offset() + view.length()) {
            predecoded::decode(content_key(encoded), encoded);
        }
    }
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
        let (_, _, images) = import_glb(path, &bytes).unwrap();
        let encoded: Vec<&Vec<u8>> = images.encoded.iter().flatten().collect();
        assert!(!encoded.is_empty(), "fixture has embedded images");
        predecode_glb_images(&bytes);
        let key = content_key(encoded[0]);
        let image = predecoded::take(key).expect("decoded by the worker").expect("a valid image");
        assert!(image.width() > 0);
        assert!(predecoded::take(key).is_none(), "taken only once");
        predecode_glb_images(&bytes);
        assert!(predecoded::take(key).is_none(), "not decoded again after it was taken");
    }
}
