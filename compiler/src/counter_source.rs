//! Compatibility surface for the process-transported counter capability.
//!
//! The sealed typed import lives in the C gateway and the trusted replay in
//! the verification adapter; this module only re-exports both under one path
//! for callers predating the extraction.
pub use qcore_c_gateway::counter_source::*;
pub use qcore_verification::counter_source::*;
