//! Source-analysis adapters shared by the compiler facade and audit runner.
//!
//! These adapters validate pinned C imports. The certificate constructors remain
//! private to their owning modules; the audit receives checked summaries only.

pub mod chunk_contract;
pub mod chunkio;
// Card W47-CONDVAR-JOIN-PUBLICATION (gateway rule 10q).
pub mod condvar_join_census;
pub mod libevent;
// Card W47-FLB-LIB-LIFECYCLE (gateway rule 10q as amended, C04.11).
pub mod lifecycle_census;
// Card W47-LOOP-OWNER-CONTRACTS (gateway rule 10p).
pub mod loop_owner_census;
pub mod source_profiles;
pub mod plugin_source_witnesses;
pub mod typed_source_match;

pub use qcore_c_gateway::legacy_c;
pub use qcore_c_gateway::source_call_use;
pub use qcore_c_gateway::typed_source;
pub use qcore_contract::contract_observation;
pub use qcore_core::chunk_lifecycle;
pub use qcore_frontend::frontend;
pub use qcore_verification::{
    cleanup_outcome, counter_contract, field_destructor, list_unlink_contract,
};
pub mod source_contract {
    pub(crate) use qcore_c_gateway::typed_source::DEFAULT_SOURCE_RULES as RULES;
}
pub mod external_integration {
    pub use crate::chunkio::PINNED_FLUENT_BIT_REVISION;
}

// Compatibility paths for migrated source-adapter tests.
#[cfg(test)]
pub(crate) mod verifier_plugins {
    pub(crate) use crate::chunkio;
}
#[cfg(test)]
pub(crate) use chunkio::chunk_source;

#[cfg(test)]
mod test_support;
