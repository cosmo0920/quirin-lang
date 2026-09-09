//! Compatibility surface for the process-transported cleanup-replay capability.
//!
//! The sealed typed import lives in the C gateway and the trusted replay in the
//! verification adapter; this module only re-exports both under one path. The
//! in-process `verifier_plugins::chunkio::{file_outcomes, file_delete}`
//! adapters are retained (hard invariant 3) and are unedited — both routes call
//! the same unedited `cleanup_outcome` checker, so the two cannot drift.
pub use qcore_c_gateway::cleanup_replay_source::*;
pub use qcore_verification::cleanup_replay_source::*;
