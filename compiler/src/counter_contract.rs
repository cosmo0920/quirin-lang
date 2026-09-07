//! Compatibility surface for trusted source-contract verification.
pub use qcore_verification::counter_contract::*;

#[cfg(test)]
#[path = "../tests/unit/counter_contract.rs"]
mod tests;
