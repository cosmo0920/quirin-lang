//! Compatibility surface for the generic C gateway source importer.
pub use qcore_c_gateway::typed_source::*;

#[cfg(test)]
use crate::legacy_c::C11TranslationUnit;
#[cfg(test)]
#[path = "../tests/unit/monkey_source.rs"]
mod tests;
