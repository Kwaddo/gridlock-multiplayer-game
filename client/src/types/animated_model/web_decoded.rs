//! Images the browser decoded in a worker thread (web/gridlock_web/decode.js),
//! so the browser build only uploads them. The browser build has no threads
//! of its own; without this, every PNG/JPEG decodes inside a frame.

/// What [`decode_ahead`] decodes: a `.glb`'s embedded images, or an image file.
pub const KIND_GLB: u32 = 1;
pub const KIND_IMAGE: u32 = 2;
/// The image index of an image file (the file is the image).
pub const WHOLE_FILE: u32 = u32::MAX;

/// Decode the images in `path` in the browser's worker when it is next
/// loaded; loading it then finishes once they are decoded.
pub fn decode_ahead(path: &str, kind: u32) {
    unsafe { web::gridlock_decode_ahead(path.as_ptr(), path.len() as u32, kind) };
}

/// Image `index` of `path` as (width, height, RGBA), if the worker decoded
/// it and nothing took it yet; otherwise decode it here.
pub fn take(path: &str, index: u32) -> Option<(u32, u32, Vec<u8>)> {
    let mut size = [0u32; 2];
    let (ptr, len) = (path.as_ptr(), path.len() as u32);
    if unsafe { web::gridlock_decoded_size(ptr, len, index, size.as_mut_ptr()) } == 0 {
        return None;
    }
    let [width, height] = size;
    let mut rgba = vec![0u8; width as usize * height as usize * 4];
    unsafe { web::gridlock_decoded_copy(ptr, len, index, rgba.as_mut_ptr()) };
    Some((width, height, rgba))
}

/// Forget decoded images nothing took (ones no mesh uses).
pub fn clear() {
    unsafe { web::gridlock_decoded_clear() };
}

mod web {
    extern "C" {
        pub fn gridlock_decode_ahead(ptr: *const u8, len: u32, kind: u32);
        pub fn gridlock_decoded_size(ptr: *const u8, len: u32, index: u32, out: *mut u32) -> u32;
        pub fn gridlock_decoded_copy(ptr: *const u8, len: u32, index: u32, dst: *mut u8);
        pub fn gridlock_decoded_clear();
    }
}
