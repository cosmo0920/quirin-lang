//! Compatibility surface for the process-transported teardown capabilities.
//!
//! The sealed typed imports live in the C gateway and the trusted replays in
//! the verification adapter; this module only re-exports both under one path.
//! The in-process `verifier_plugins::chunkio::teardown_source` adapter is
//! retained and unedited — this is an added route, not a replacement.
pub use qcore_c_gateway::teardown_source::*;
pub use qcore_verification::teardown_source::*;
