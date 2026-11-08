#[cfg(target_arch = "wasm32")]
use macroquad::experimental::coroutines::{start_coroutine, Coroutine};
use macroquad::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

use super::cache::has_ext;

thread_local! {
    /// What [`Prefetcher`] downloaded (or why it could not), by resolved path,
    /// until the loader takes it.
    pub(super) static PREFETCHED: RefCell<HashMap<String, Result<Vec<u8>, String>>> = RefCell::new(HashMap::new());
    /// Image files a worker thread already decoded, by resolved path, until
    /// the loader uploads them (native builds).
    pub(super) static DECODED_IMAGES: RefCell<HashMap<String, Image>> = RefCell::new(HashMap::new());
}

/// A file on its way: a browser download polled at `next_frame`, or (native)
/// a worker thread that reads the file and decodes what can be decoded off
/// the main thread, so neither the disk read nor PNG/JPEG decoding stalls
/// a frame. Only the GPU upload is left for the main thread. In the browser
/// a web worker decodes the images before the download counts as done
/// (`client::types::animated_model::web_decoded`).
enum Download {
    #[cfg(target_arch = "wasm32")]
    Coroutine(Coroutine<Result<Vec<u8>, String>>),
    #[cfg(not(target_arch = "wasm32"))]
    Thread(std::thread::JoinHandle<Fetched>),
}

#[cfg(not(target_arch = "wasm32"))]
struct Fetched {
    bytes: Result<Vec<u8>, String>,
    /// The file decoded as an image, for texture files.
    image: Option<Image>,
}

impl Download {
    fn start(file: String) -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            use client::types::animated_model::web_decoded;
            if has_ext(&file, ".glb") {
                web_decoded::decode_ahead(&file, web_decoded::KIND_GLB);
            } else if is_image(&file) {
                web_decoded::decode_ahead(&file, web_decoded::KIND_IMAGE);
            }
        }
        #[cfg(target_arch = "wasm32")]
        return Download::Coroutine(start_coroutine(async move {
            load_file(&file).await.map_err(|e| format!("{e:?}"))
        }));

        #[cfg(not(target_arch = "wasm32"))]
        return Download::Thread(std::thread::spawn(move || {
            let bytes = std::fs::read(&file).map_err(|e| format!("{file}: {e}"));
            let mut image = None;
            if let Ok(bytes) = &bytes {
                if has_ext(&file, ".glb") {
                    client::types::animated_model::predecode_glb_images(bytes);
                } else if is_image(&file) {
                    image = Image::from_file_with_format(bytes, None).ok();
                }
            }
            Fetched { bytes, image }
        }));
    }

    fn is_done(&self) -> bool {
        match self {
            #[cfg(target_arch = "wasm32")]
            Download::Coroutine(c) => c.is_done(),
            #[cfg(not(target_arch = "wasm32"))]
            Download::Thread(t) => t.is_finished(),
        }
    }

    /// Hand a finished download to the loader.
    fn deliver(self, resolved: &str) {
        let bytes = match self {
            #[cfg(target_arch = "wasm32")]
            Download::Coroutine(c) => c.retrieve(),
            #[cfg(not(target_arch = "wasm32"))]
            Download::Thread(t) => Some(match t.join() {
                Ok(fetched) => {
                    if let Some(image) = fetched.image {
                        DECODED_IMAGES.with(|d| d.borrow_mut().insert(resolved.to_string(), image));
                    }
                    fetched.bytes
                }
                Err(_) => Err(format!("{resolved}: loader thread panicked")),
            }),
        };
        if let Some(result) = bytes {
            PREFETCHED.with(|p| p.borrow_mut().insert(resolved.to_string(), result));
        }
    }
}

/// Downloads assets in the background, so the loading screen keeps drawing,
/// and a game hosted in this tab keeps answering players, while a large
/// model downloads. Every file the loader reads goes through it except OBJs,
/// whose loader fetches their materials itself (the browser build has none).
#[derive(Default)]
pub(super) struct Prefetcher {
    pending: HashMap<String, Download>,
}

impl Prefetcher {
    /// Start downloading `resolved` (a path already resolved) unless it is
    /// already on its way.
    pub fn start(&mut self, resolved: &str) {
        if !prefetchable(resolved) || self.pending.contains_key(resolved) {
            return;
        }
        self.pending.insert(resolved.to_string(), Download::start(resolved.to_string()));
    }

    /// Whether `resolved` (a path already resolved) can be loaded now without
    /// waiting on the network. Starts its download if nothing has yet.
    pub fn ready(&mut self, resolved: &str) -> bool {
        if !prefetchable(resolved) {
            return true;
        }
        self.start(resolved);
        if !self.pending[resolved].is_done() {
            return false;
        }
        // A failure is handed over too: retrying inline would wait on the
        // network inside a frame (a black flash on the loading screen).
        if let Some(download) = self.pending.remove(resolved) {
            download.deliver(resolved);
        }
        true
    }
}

fn prefetchable(resolved_path: &str) -> bool {
    !has_ext(resolved_path, ".obj")
}

fn is_image(path: &str) -> bool {
    [".png", ".jpg", ".jpeg"].iter().any(|ext| has_ext(path, ext))
}

/// The file's bytes: downloaded already by [`Prefetcher`], or read now.
pub(super) async fn read_bytes(resolved_path: &str) -> Result<Vec<u8>, String> {
    if let Some(result) = PREFETCHED.with(|p| p.borrow_mut().remove(resolved_path)) {
        return result;
    }
    load_file(resolved_path).await.map_err(|e| format!("{e:?}"))
}

/// The image file at `resolved_path`, already decoded by a worker thread, if
/// one did.
pub(super) fn take_decoded_image(resolved_path: &str) -> Option<Image> {
    #[cfg(target_arch = "wasm32")]
    {
        use client::types::animated_model::web_decoded;
        let (width, height, bytes) = web_decoded::take(resolved_path, web_decoded::WHOLE_FILE)?;
        if width > u16::MAX as u32 || height > u16::MAX as u32 {
            return None;
        }
        Some(Image { bytes, width: width as u16, height: height as u16 })
    }
    #[cfg(not(target_arch = "wasm32"))]
    DECODED_IMAGES.with(|d| d.borrow_mut().remove(resolved_path))
}

/// Forget everything decoded ahead that the loader did not take.
pub(super) fn release_decoded() {
    DECODED_IMAGES.with(|d| d.borrow_mut().clear());
    client::types::animated_model::release_predecoded();
}
