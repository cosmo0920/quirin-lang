//! First-party in-process verifier implementations.
//!
//! Each plugin owns its library semantics and manifest. Typed analysis APIs
//! remain available during extraction; their results are not protocol-v1 core
//! certificates. Registry adapters retain the fail-closed v1 behavior until
//! semantic inputs and independently checked witnesses are supported.

pub mod chunkio;
pub mod libevent;
