//! Locating this repository's in-tree source-set profiles.
//!
//! W2 moved the pinned source-set tables into
//! `contract/profiles/<source-set>/`. The loader itself
//! ([`qcore_contract::profile_loader`]) takes an explicit directory and knows
//! nothing about this repository; this module supplies only the in-tree
//! default used by the audit driver, the examples and the tests, plus the
//! `QCORE_PROFILES` override for pointing them at a different profile set.
//!
//! Resolution is fail-closed in the same way the loader is: a missing or
//! invalid profile is a diagnostic, never a built-in fallback table.

use std::env;
use std::path::PathBuf;

use qcore_contract::profile_loader::{LoadedProfile, ProfileDiagnostic, load_source_set_profile};

/// The source set holding the pinned Fluent Bit corpus and its vendored
/// libraries (CMetrics, CTraces, librdkafka, CFL, ChunkIO, input-chunk).
pub const FLUENT_BIT_SOURCE_SET: &str = "fluent-bit";

/// The source set holding the pinned Libevent checkout.
pub const LIBEVENT_SOURCE_SET: &str = "libevent";

/// `QCORE_PROFILES` when explicitly set, otherwise this repository's
/// `contract/profiles`. A root that holds no matching profile directory is a
/// load failure (F1), not a fallback to built-in tables.
#[must_use]
pub fn profiles_root() -> PathBuf {
    match env::var_os("QCORE_PROFILES") {
        Some(value) => PathBuf::from(value),
        // Preserve the historical non-canonical spelling in assumption keys
        // and contract sidecars while the source adapter changes owning crate.
        // Both parent calls reach the repository root from this crate.
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("source-analysis crate must be inside repository crates")
            .join("compiler/../contract/profiles"),
    }
}

/// Load one source-set profile from [`profiles_root`].
pub fn load_profile(source_set: &str) -> Result<LoadedProfile, ProfileDiagnostic> {
    load_source_set_profile(&profiles_root(), source_set)
}
