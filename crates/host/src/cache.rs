//! Host-cache implementation: namespaced sled DB backing the host-cache WIT interface.
//!
//! Each station's keys are prefixed with its callsign, so stations can never
//! read each other's cache entries even though they share one sled tree.
//! The host-cache contract is best-effort: the DB can be wiped at any time
//! and stations must tolerate cold starts gracefully.

use anyhow::Result;
use std::path::Path;

pub struct Cache {
    db: sled::Db,
}

impl Cache {
    pub fn open(path: &Path) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Namespaced get. Returns None if the key is absent.
    pub fn get(&self, callsign: &str, key: &str) -> Result<Option<Vec<u8>>> {
        let k = namespaced(callsign, key);
        Ok(self.db.get(k)?.map(|v| v.to_vec()))
    }

    /// Namespaced set.
    pub fn set(&self, callsign: &str, key: &str, value: &[u8]) -> Result<()> {
        let k = namespaced(callsign, key);
        self.db.insert(k, value)?;
        Ok(())
    }

    /// Flush to disk. Call periodically or on clean shutdown.
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}

fn namespaced(callsign: &str, key: &str) -> String {
    format!("{callsign}\x00{key}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn round_trip() {
        let dir = std::env::temp_dir().join("salvage-cache-test");
        let cache = Cache::open(&dir).unwrap();
        cache.set("TECH", "last_seen", b"abc123").unwrap();
        let got = cache.get("TECH", "last_seen").unwrap();
        assert_eq!(got.as_deref(), Some(b"abc123".as_ref()));
        // cleanup
        drop(cache);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn namespace_isolation() {
        let dir = std::env::temp_dir().join("salvage-cache-ns-test");
        let cache = Cache::open(&dir).unwrap();
        cache.set("TECH", "key", b"tech").unwrap();
        cache.set("AI", "key", b"ai").unwrap();
        assert_eq!(cache.get("TECH", "key").unwrap().as_deref(), Some(b"tech".as_ref()));
        assert_eq!(cache.get("AI",   "key").unwrap().as_deref(), Some(b"ai".as_ref()));
        drop(cache);
        let _ = std::fs::remove_dir_all(dir);
    }
}
