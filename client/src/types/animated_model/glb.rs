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
    Ok((document, buffers, GlbImages { encoded }))
}

/// A GLB's embedded images (PNG or JPEG), decoded when a mesh uses one.
/// Decoded textures are shared by content: the player's animation files each
/// embed the same textures, and decoding and uploading those once instead of
/// once per file saves most of the load time and GPU memory.
pub struct GlbImages {
    encoded: Vec<Option<Vec<u8>>>,
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
        use std::hash::{Hash, Hasher};
        let bytes = self.encoded.get(index)?.as_ref()?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        let key = hasher.finish();
        TEXTURES.with(|textures| {
            textures.borrow_mut().entry(key).or_insert_with(|| decode_texture(bytes)).clone()
        })
    }
}

fn decode_texture(bytes: &[u8]) -> Option<Texture2D> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    if width > u16::MAX as u32 || height > u16::MAX as u32 {
        return None;
    }
    let texture = Texture2D::from_rgba8(width as u16, height as u16, &image);
    texture.set_filter(FilterMode::Nearest);
    Some(texture)
}
