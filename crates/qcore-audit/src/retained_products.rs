//! Run-local storage for produced reports, never an evidence admission route.
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Serialize, de::DeserializeOwned};

static NEXT: AtomicU64 = AtomicU64::new(0);
const MAX_BYTES: u64 = 1024 * 1024 * 1024;

/// Caller-held length and digest authenticate this process's own payload.
/// No pathname or manifest read from disk establishes an expected identity.
pub(crate) struct Spool {
    directory: PathBuf,
    length: u64,
    digest: String,
}

impl Spool {
    pub(crate) fn write(value: &impl Serialize) -> Result<Self, String> {
        let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("retained report exceeds the 1 GiB spool object limit".into());
        }
        let root = PathBuf::from("private/audit-retained-products");
        fs::create_dir_all(&root).map_err(|e| format!("retained report directory: {e}"))?;
        let directory = loop {
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!("{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => break path,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(format!("retained report directory: {e}")),
            }
        };
        let spool = Self {
            directory,
            length: bytes.len() as u64,
            digest: qcore_contract::profile_loader::sha256_hex(&bytes),
        };
        let temporary = spool.directory.join("pending.qc");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| format!("retained report create: {e}"))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| format!("retained report write: {e}"))?;
        drop(file);
        fs::rename(&temporary, spool.directory.join("product.qc"))
            .map_err(|e| format!("retained report publish: {e}"))?;
        Ok(spool)
    }

    pub(crate) fn read<T: DeserializeOwned>(&self) -> Result<T, String> {
        let file = File::open(self.directory.join("product.qc"))
            .map_err(|e| format!("retained report open: {e}"))?;
        if self.length > MAX_BYTES
            || file.metadata().map_err(|e| e.to_string())?.len() != self.length
        {
            return Err("retained report length mismatch".into());
        }
        let mut bytes = Vec::new();
        file.take(self.length + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("retained report read: {e}"))?;
        if bytes.len() as u64 != self.length
            || qcore_contract::profile_loader::sha256_hex(&bytes) != self.digest
        {
            return Err("retained report content mismatch".into());
        }
        serde_json::from_slice(&bytes).map_err(|e| format!("retained report decode: {e}"))
    }
}

impl Drop for Spool {
    fn drop(&mut self) {
        // The directory was exclusively created by this owner. Never remove
        // another run's files, including stale directories from a killed run.
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn complete_ordered_payload_round_trip_and_cleanup() {
        let payload =
            json!([{"diagnostic": "first", "unknown": true}, {"n": 18446744073709551615u64}]);
        let spool = Spool::write(&payload).unwrap();
        let directory = spool.directory.clone();
        assert_eq!(spool.read::<Value>().unwrap(), payload);
        assert_eq!(spool.read::<Value>().unwrap(), payload);
        drop(spool);
        assert!(!directory.exists());
    }

    #[test]
    fn incomplete_corrupt_and_missing_payloads_refuse_and_clean_up() {
        for replacement in [Some(b"[1".as_slice()), Some(b"[2]".as_slice()), None] {
            let spool = Spool::write(&json!([1])).unwrap();
            let directory = spool.directory.clone();
            let path = directory.join("product.qc");
            if let Some(bytes) = replacement {
                fs::write(&path, bytes).unwrap();
            } else {
                fs::remove_file(&path).unwrap();
            }
            assert!(spool.read::<Value>().is_err());
            drop(spool);
            assert!(!directory.exists());
        }
    }

    #[test]
    fn unwind_cleans_only_owned_directory() {
        let spool = Spool::write(&json!([])).unwrap();
        let directory = spool.directory.clone();
        let survivor = Spool::write(&json!(["other"])).unwrap();
        let _ = std::panic::catch_unwind(move || {
            let _owner = spool;
            panic!("cancel");
        });
        assert!(!directory.exists());
        assert_eq!(survivor.read::<Value>().unwrap(), json!(["other"]));
    }
}
