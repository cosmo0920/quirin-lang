// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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

/// Decision 1's binding shape (`QCore.NullDeref.premised`): no tracked
/// target, unknown, possibly null, the identity derivation, in a state that
/// is not widened.
#[must_use]
pub fn premised_shape(
    targets_empty: bool,
    unknown: bool,
    may_be_null: bool,
    identity: bool,
    widened: bool,
) -> bool {
    targets_empty && unknown && may_be_null && identity && !widened
}

/// The Lean `premisedVectors`: (targets empty, unknown, possibly null,
/// identity, widened, premised).
#[cfg(test)]
pub(crate) const PREMISED_VECTORS: &[(bool, bool, bool, bool, bool, bool)] = &[
    (true, true, true, true, false, true),
    (false, true, true, true, false, false),
    (true, false, true, true, false, false),
    (true, true, false, true, false, false),
    (true, true, true, false, false, false),
    (true, true, true, true, true, false),
];

#[cfg(test)]
mod tests {
    use super::*;

    const LEAN: &str = include_str!("../../../proof/QCore/NullDeref.lean");

    #[test]
    fn premised_vectors_hold_and_are_the_lean_vectors() {
        for (t, u, m, i, w, premised) in PREMISED_VECTORS {
            assert_eq!(premised_shape(*t, *u, *m, *i, *w), *premised);
        }
        let header = "def premisedVectors : List (Bool \u{d7} Bool \u{d7} Bool \u{d7} Bool \u{d7} Bool \u{d7} Bool) := [";
        let start = LEAN.find(header).expect("the Lean vector list") + header.len();
        let body = &LEAN[start..];
        let end = body.find("\n]").expect("the list's end");
        let rows = body[..end]
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.trim_end_matches(',').to_owned())
            .collect::<Vec<_>>();
        let rust = PREMISED_VECTORS
            .iter()
            .map(|(t, u, m, i, w, p)| format!("({t}, {u}, {m}, {i}, {w}, {p})"))
            .collect::<Vec<_>>();
        assert_eq!(rows, rust);
    }

    #[test]
    fn a_premise_is_kept_by_the_analysis_that_relied_on_it() {
        let ((), outer) = collecting(|| {
            note("f", 3, "n");
            let ((), inner) = collecting(|| note("g", 9, "m"));
            assert_eq!(
                inner,
                vec![NullPremise {
                    line: 9,
                    variable: "m".into()
                }]
            );
            note("f", 3, "n");
        });
        assert_eq!(
            outer,
            vec![NullPremise {
                line: 3,
                variable: "n".into()
            }]
        );
        assert_eq!(outer[0].name("f"), "null-deref:f:3:n");
    }
}
