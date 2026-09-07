//! Compatibility facade for the provider-free verification adapter.
pub use qcore_verification::external_source::*;

#[cfg(test)]
#[path = "../tests/unit/external_source.rs"]
mod tests;
