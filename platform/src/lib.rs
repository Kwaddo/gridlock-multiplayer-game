//! What the game needs from the platform beyond `std`: clocks, randomness and
//! small persistent storage that also work in the browser.
//!
//! Native builds use `std` directly. Browser builds are loaded by miniquad's JS
//! loader, which cannot satisfy wasm-bindgen imports, so `std::time::Instant`
//! (which panics on wasm32-unknown-unknown) and getrandom's JS backend are
//! replaced by small imports from the `gridlock_web` plugin (`web/gridlock_web.js`).

pub use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;

#[cfg(target_arch = "wasm32")]
pub use web::Instant;

/// Milliseconds since the Unix epoch (wall clock, for log timestamps).
pub fn unix_time_ms() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    return std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    #[cfg(target_arch = "wasm32")]
    return unsafe { web::gridlock_unix_ms() } as u64;
}

/// Whether the page is in a hidden browser tab. Always false natively.
pub fn page_hidden() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    return false;

    #[cfg(target_arch = "wasm32")]
    return unsafe { web::gridlock_page_hidden() } != 0;
}

/// A setting from the page that loaded the game: the string in
/// `window.GRIDLOCK_<name>`, when it is set and not empty. The local server
/// (`registry/local/`) uses it to point the game at itself. Native builds have
/// no page, so this is always `None` there.
pub fn page_setting(name: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = name;
        None
    }

    #[cfg(target_arch = "wasm32")]
    unsafe {
        let mut len = 0u32;
        if web::gridlock_page_setting(name.as_ptr(), name.len() as u32, &mut len) == 0 {
            return None;
        }
        let mut buf = vec![0u8; len as usize];
        web::gridlock_storage_take(buf.as_mut_ptr());
        String::from_utf8(buf).ok().filter(|v| !v.is_empty())
    }
}

/// Small text values kept between runs (settings, crosshair presets).
///
/// Native builds keep a file named `key` in the working directory, which the
/// game sets to its asset root. The browser keeps them in `localStorage`.
pub mod storage {
    pub fn load(key: &str) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        return std::fs::read_to_string(key).ok();

        #[cfg(target_arch = "wasm32")]
        unsafe {
            let mut len = 0u32;
            if super::web::gridlock_storage_get(key.as_ptr(), key.len() as u32, &mut len) == 0 {
                return None;
            }
            let mut buf = vec![0u8; len as usize];
            super::web::gridlock_storage_take(buf.as_mut_ptr());
            String::from_utf8(buf).ok()
        }
    }

    /// Returns false if the value could not be stored (disk error, quota, private mode).
    pub fn save(key: &str, value: &str) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return std::fs::write(key, value).is_ok();

        #[cfg(target_arch = "wasm32")]
        unsafe {
            super::web::gridlock_storage_set(key.as_ptr(), key.len() as u32, value.as_ptr(), value.len() as u32)
                != 0
        }
    }
}

/// Lets miniquad check that the loaded `gridlock_web` JS plugin matches this
/// build (see `PLUGIN_VERSION` in `web/gridlock_web.js`).
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn gridlock_web_crate_version() -> u32 {
    3
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::ops::{Add, AddAssign, Sub, SubAssign};
    use std::time::Duration;

    extern "C" {
        /// `performance.now()`: monotonic milliseconds.
        fn gridlock_now_ms() -> f64;
        pub(crate) fn gridlock_unix_ms() -> f64;
        pub(crate) fn gridlock_page_hidden() -> u32;
        fn gridlock_random_fill(ptr: *mut u8, len: u32);
        /// 1 if `key` is stored; its UTF-8 length goes to `len_out`, bytes via `take`.
        pub(crate) fn gridlock_storage_get(key: *const u8, key_len: u32, len_out: *mut u32) -> u32;
        pub(crate) fn gridlock_storage_take(buf: *mut u8);
        pub(crate) fn gridlock_storage_set(key: *const u8, key_len: u32, value: *const u8, value_len: u32) -> u32;
        /// 1 if `window.GRIDLOCK_<name>` is a string; its UTF-8 length goes to
        /// `len_out`, bytes via `gridlock_storage_take`.
        pub(crate) fn gridlock_page_setting(name: *const u8, name_len: u32, len_out: *mut u32) -> u32;
    }

    /// Monotonic instant with the same API surface as `std::time::Instant`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Instant(Duration);

    impl Instant {
        pub fn now() -> Self {
            Instant(Duration::from_secs_f64(unsafe { gridlock_now_ms() }.max(0.0) / 1000.0))
        }

        pub fn elapsed(&self) -> Duration {
            Instant::now().saturating_duration_since(*self)
        }

        pub fn duration_since(&self, earlier: Instant) -> Duration {
            self.saturating_duration_since(earlier)
        }

        pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
            self.0.saturating_sub(earlier.0)
        }

        pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
            self.0.checked_sub(earlier.0)
        }

        pub fn checked_add(&self, d: Duration) -> Option<Instant> {
            self.0.checked_add(d).map(Instant)
        }

        pub fn checked_sub(&self, d: Duration) -> Option<Instant> {
            self.0.checked_sub(d).map(Instant)
        }
    }

    impl Add<Duration> for Instant {
        type Output = Instant;
        fn add(self, d: Duration) -> Instant {
            Instant(self.0 + d)
        }
    }

    impl AddAssign<Duration> for Instant {
        fn add_assign(&mut self, d: Duration) {
            self.0 += d;
        }
    }

    impl Sub<Duration> for Instant {
        type Output = Instant;
        fn sub(self, d: Duration) -> Instant {
            Instant(self.0.saturating_sub(d))
        }
    }

    impl SubAssign<Duration> for Instant {
        fn sub_assign(&mut self, d: Duration) {
            self.0 = self.0.saturating_sub(d);
        }
    }

    impl Sub<Instant> for Instant {
        type Output = Duration;
        fn sub(self, other: Instant) -> Duration {
            self.saturating_duration_since(other)
        }
    }

    fn random_fill(buf: &mut [u8]) -> Result<(), getrandom::Error> {
        unsafe { gridlock_random_fill(buf.as_mut_ptr(), buf.len() as u32) };
        Ok(())
    }

    getrandom::register_custom_getrandom!(random_fill);
}
