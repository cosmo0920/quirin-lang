//! Compatibility surface for trusted source-contract verification.
pub use qcore_verification::field_destructor::*;

#[cfg(test)]
#[path = "../tests/unit/field_destructor.rs"]
mod tests;
