//! Conservative existing-C analysis and Clang transport (L2 gateway).
//!
//! The gateway validates preprocessing, derives and checks custody summaries,
//! and rederives certificate evidence. Contracts remain explicit assumptions.
//! No provider or compiler crate is a dependency. The abstract interpreter,
//! summary inference machinery and entry-state constructors remain private.
//!
//! [`typed_c_ast`] provides raw Clang JSON transport for source-profile
//! interpreters. Its output alone is never a safety certificate.

mod aggregate_store;
pub mod atomic_publication_source;
pub mod boolean_source;
mod borrow_liveness;
mod borrowed_actual;
pub mod call_use_source;
mod clang;
pub mod cleanup_replay_source;
pub mod composition_source;
#[cfg(feature = "indirect-census")]
pub mod condition_call_census;
pub mod counter_source;
#[cfg(feature = "indirect-census")]
pub mod destination_suffix_census;
mod index_traversal;
pub mod indexed_collection;
#[cfg(feature = "indirect-census")]
pub mod indirect_census;
mod indirect_chain_access;
#[cfg(feature = "indirect-census")]
pub mod indirect_chain_census;
pub mod legacy_c;
pub mod legacy_census;
mod legacy_condition_call;
mod legacy_summary;
mod may_alias;
mod neutral_position;
mod publication;
pub mod source_call_use;
pub mod teardown_source;
#[cfg(feature = "indirect-census")]
pub mod traversal_census;
mod type_query;
pub mod typed_c_ast;
mod typed_field_access;
pub mod typed_source;
pub mod typed_source_match;
mod variadic_nested;

use qcore_contract::contract_loader;

pub use clang::ClangIdentity;
/// Construct a validated Clang command using the configured discovery policy.
/// An invalid or empty `QCORE_CLANG` override returns an error without fallback.
/// Callers select arguments and remain responsible for subprocess execution.
pub use clang::command as clang_command;

#[cfg(test)]
pub(crate) mod test_support {
    use std::io::Write;
    use std::sync::{Once, OnceLock};

    static CLANG_AVAILABILITY: OnceLock<Result<(), String>> = OnceLock::new();
    static CLANG_WARNING: Once = Once::new();

    /// Report whether the Clang-backed tests can run, emitting one visible
    /// warning when the executable is absent or unusable.
    pub(crate) fn clang_available() -> bool {
        let availability = CLANG_AVAILABILITY.get_or_init(|| crate::clang_command().map(|_| ()));
        match availability {
            Ok(()) => true,
            Err(reason) => {
                CLANG_WARNING.call_once(|| {
                    // Write directly so libtest's successful-test capture does
                    // not hide the reason these tests did not execute.
                    let _ = writeln!(
                        std::io::stderr().lock(),
                        "warning: skipping Clang-dependent tests: {reason}"
                    );
                });
                false
            }
        }
    }
}
