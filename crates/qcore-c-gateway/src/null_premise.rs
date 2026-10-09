//! Card **NULL-SAFETY-PROPERTY** (gateway rule 14n, USER DECISION
//! 2026-10-10, decision 1): the named null premises a custody analysis rests
//! on.
//!
//! A scalar read or write through a **target-free, possibly-null borrowed
//! alias** (what a pointer-valued path read binds, and rule 14b's interior
//! alias) is read as a dereference of a non-null pointer: the NULL question
//! moves to the null-dereference property (`c.null.deref.v1`), whose check at
//! that dereference proves it or publishes a candidate. The custody verdict
//! that relied on the reading lists it as a named premise,
//! `null-deref:<function>:<line>:<variable>`
//! ([`qcore_core::null_deref::premise_name`]), until the null property proves
//! the check:
//!
//! * the function's own premises are recorded on its summary
//!   ([`crate::legacy_c::LegacyFunctionSummary::null_premises`]) by the
//!   analysis that relied on them ([`collecting`]), so no route can publish
//!   the verdict without them;
//! * each is also noted in the open consultation log
//!   ([`qcore_schemas::contract_consultation::note_null_premise`]), so a row
//!   consulting an in-unit summary that rests on one lists it too (rule
//!   13f's transitive dependency set).
//!
//! Nothing here decides a verdict: the reading is the hook's
//! (`legacy_null_premise.rs`); this module only keeps what it relied on.

use std::cell::RefCell;
use std::collections::BTreeSet;

/// One relied-on non-null reading: the dereference's line (in the analysed
/// text) and the variable read non-null there.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NullPremise {
    pub line: usize,
    pub variable: String,
}

impl NullPremise {
    /// The premise's name in `function`.
    #[must_use]
    pub fn name(&self, function: &str) -> String {
        qcore_core::null_deref::premise_name(function, self.line, &self.variable)
    }
}

thread_local! {
    static FRAMES: RefCell<Vec<BTreeSet<NullPremise>>> = const { RefCell::new(Vec::new()) };
}

/// Run one function's analysis and return what it relied on. Frames nest: a
/// nested analysis keeps its own premises.
pub(crate) fn collecting<R>(work: impl FnOnce() -> R) -> (R, Vec<NullPremise>) {
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            FRAMES.with(|frames| {
                frames.borrow_mut().pop();
            });
        }
    }
    FRAMES.with(|frames| frames.borrow_mut().push(BTreeSet::new()));
    let pop = Pop;
    let result = work();
    let premises = FRAMES.with(|frames| {
        frames
            .borrow_mut()
            .last_mut()
            .map(std::mem::take)
            .unwrap_or_default()
    });
    drop(pop);
    (result, premises.into_iter().collect())
}

/// Note that `function`'s analysis read `variable` non-null at `line`.
pub(crate) fn note(function: &str, line: usize, variable: &str) {
    let premise = NullPremise {
        line,
        variable: variable.to_owned(),
    };
    qcore_schemas::contract_consultation::note_null_premise(&premise.name(function));
    FRAMES.with(|frames| {
        if let Some(frame) = frames.borrow_mut().last_mut() {
            frame.insert(premise);
        }
    });
}
