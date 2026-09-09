//! Compatibility surface for the process-transported composition capability.
//!
//! The sealed typed import lives in the C gateway, the relocated composition
//! decision procedure and the trusted task in the verification adapter; this
//! module only re-exports all three under one path. The in-process
//! `verifier_plugins::chunkio::unmap_wrapper` adapter is retained (hard
//! invariant 3) and now delegates to the same relocated procedure, so the two
//! routes cannot drift.
pub use qcore_c_gateway::composition_source::*;
pub use qcore_verification::composition_source::*;
pub use qcore_verification::unmap_composition::{
    UnmapCase, UnmapCertificate, UnmapCompositionContracts, check_unmap_composition,
};
