//! Compatibility surface for the process-transported call-result use capability.
//!
//! The sealed typed import lives in the C gateway and the trusted replay in the
//! verification adapter; this module only re-exports both under one path,
//! together with the shared decision procedure. The in-process
//! `verifier_plugins::chunkio::file_attempts` adapter is retained — it now
//! delegates to the same decision procedure, so the two routes cannot drift.
pub use qcore_c_gateway::call_use_source::*;
pub use qcore_verification::call_use_contract::*;
pub use qcore_verification::call_use_source::*;
