//! Compatibility facade for the provider-free verification adapter.
pub use qcore_verification::research_extensions::*;

#[cfg(test)]
#[path = "../tests/unit/research_extensions.rs"]
mod tests;
