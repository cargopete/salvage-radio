//! Ergonomic wrapper over the host-cache WIT interface.
//!
//! The host namespaces keys by station callsign automatically,
//! so stations can use simple keys like "last_seen" without collision.
//!
//! Cache is best-effort: the host MAY evict entries at any time.
//! Stations must tolerate a cold cache and still produce correct (possibly
//! temporarily redundant) output.

use serde::{de::DeserializeOwned, Serialize};

pub struct Cache;

impl Cache {
    pub fn new() -> Self {
        Self
    }

    pub fn get_bytes(&self, key: &str) -> Option<Vec<u8>> {
        #[cfg(target_arch = "wasm32")]
        {
            // TODO M1: call host-cache::get() via wit-bindgen generated bindings
            let _ = key;
            None
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;
            None
        }
    }

    pub fn set_bytes(&self, key: &str, value: &[u8]) {
        #[cfg(target_arch = "wasm32")]
        {
            // TODO M1: call host-cache::set() via wit-bindgen generated bindings
            let _ = (key, value);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (key, value);
        }
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        String::from_utf8(self.get_bytes(key)?).ok()
    }

    pub fn set_string(&self, key: &str, value: &str) {
        self.set_bytes(key, value.as_bytes());
    }

    pub fn get_json<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let bytes = self.get_bytes(key)?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn set_json<T: Serialize>(&self, key: &str, value: &T) {
        if let Ok(bytes) = serde_json::to_vec(value) {
            self.set_bytes(key, &bytes);
        }
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}
