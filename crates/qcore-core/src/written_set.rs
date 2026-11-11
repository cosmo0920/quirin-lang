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

//! Card **CELL-RESULT-SET-WRITTEN-SET-DERIVATION** (contract specification
//! C09.4, grammar G05j as amended, gateway rule 38e as amended): the
//! combination rules of the pin-time slot derivation.
//!
//! USER DECISIONS 2026-11-04 (verbatim): "Basically, Q1(a) and Q2(i) But we
//! must not handle descriptor 0 as a valid handle. So, we need to exclude the
//! 0 descriptor like: written_set(pipedes, [1, MAX]); a live descriptor is in
//! [1, MAX]"; "Write [1, MAX]".
//!
//! A pinned `written_set(SLOT[K], SET) when #result == C` is derived from the
//! configured unit's body (`qcore_c_gateway::null_deref`'s walk reads the
//! AST); this module is the part of the rule that does not read C, shared
//! entry for entry with the Lean model `QCore.WrittenSetDerivation`:
//!
//! * [`combine`]: every path of the body that ends at a `return` carries the
//!   set its returned value lies in (narrowed by each test on the path) and
//!   the set the `int` at `SLOT[K]` lies in as the body left it (`None`: not
//!   written on the path, or not known). The written set for class `C` is
//!   the union of the slot sets of the paths that may return `C`; a path
//!   that may return `C` with no slot set refuses, and so does a body none
//!   of whose paths returns `C`.
//! * [`narrow`]: a test `v OP k` narrows a local's set on each edge.
//! * [`resolve`]: an out array a call wrote holds the callee's written set of
//!   class `c` once the call's result, narrowed within the callee's result
//!   set, is exactly `{c}`; an empty narrowing makes the path infeasible;
//!   anything else leaves the array unknown.
//! * [`after_store`]: a later store through a pointer that may alias the
//!   slot adds its value to the slot's set.
//!
//! The derivation only reads: nothing here assumes a set. The one set the
//! production pins rest on is `pipe`'s declared `[1, MAX]`, a DECLARED
//! POLICY, not POSIX's `[0, MAX]` (follow-up
//! CELL-RESULT-SET-DESCRIPTOR-ZERO-POLICY); under POSIX's set the same rules
//! give `[0, MAX]` ([`COMBINE_VECTORS`]' contrast row).

use crate::result_set::{Cmp, Interval, ResultSet, comparison_set};

/// One path of a body ending at a `return`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathEnd {
    /// The set the returned value lies in on this path.
    pub returns: ResultSet,
    /// The set the slot's `int` lies in as the path left it; `None` when the
    /// path does not write it or the walk does not know what it holds.
    pub slot: Option<ResultSet>,
}

/// Why no written set is derived.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A path that may return the class leaves the slot unwritten or unknown.
    Unwritten,
    /// No path returns the class.
    NoReturn,
}

impl Refusal {
    /// The code a pin refusal names.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unwritten => "written-set-unwritten",
            Self::NoReturn => "written-set-no-return",
        }
    }
}

/// The written set of class `class` over a body's path ends (see the
/// module docs), in `universe` (the slot's `int`).
///
/// # Errors
///
/// [`Refusal::Unwritten`] for a path that may return `class` without a slot
/// set, [`Refusal::NoReturn`] when no path may return it.
pub fn combine(universe: Interval, class: i128, ends: &[PathEnd]) -> Result<ResultSet, Refusal> {
    let mut out: Option<ResultSet> = None;
    for end in ends {
        if !end.returns.contains(class) {
            continue;
        }
        let slot = end.slot.as_ref().ok_or(Refusal::Unwritten)?;
        let slot = ResultSet::from_intervals(universe, slot.intervals());
        out = Some(match out {
            Some(acc) => acc.union(&slot),
            None => slot,
        });
    }
    out.ok_or(Refusal::NoReturn)
}

/// A local's set on one edge of `v OP k`.
#[must_use]
pub fn narrow(set: &ResultSet, cmp: Cmp, k: i128, true_edge: bool) -> ResultSet {
    let test = comparison_set(set.universe(), cmp, k);
    if true_edge {
        set.intersection(&test)
    } else {
        set.difference(&test)
    }
}

/// What an out array a call wrote holds once the call's result is narrowed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// The narrowed result is empty: the path cannot occur.
    Infeasible,
    /// The callee's written set of the one class the result is in.
    Known(ResultSet),
    /// The result is in more than one class, or the callee states nothing
    /// for its class.
    Unknown,
}

/// The out array's set after the call's result was narrowed to `narrowed`,
/// read within the callee's result set `results`; `written(c)` is the
/// callee's written set of class `c` for that element, if it states one.
#[must_use]
pub fn resolve(
    narrowed: &ResultSet,
    results: &ResultSet,
    mut written: impl FnMut(i128) -> Option<ResultSet>,
) -> Resolution {
    let within = narrowed.intersection(results);
    match within.intervals() {
        [] => Resolution::Infeasible,
        [(lo, hi)] if lo == hi => written(*lo).map_or(Resolution::Unknown, Resolution::Known),
        _ => Resolution::Unknown,
    }
}

/// The slot's set after a store through a pointer that may alias it; a
/// slot not yet known stays unknown.
#[must_use]
pub fn after_store(slot: Option<&ResultSet>, stored: &ResultSet) -> Option<ResultSet> {
    slot.map(|slot| {
        slot.union(&ResultSet::from_intervals(
            slot.universe(),
            stored.intervals(),
        ))
    })
}

/// `int`'s universe.
pub const INT: Interval = crate::result_set::INT;

/// The largest `int`.
const MAX: i128 = INT.1;

/// One path end of a vector: the returned set and the slot's set.
pub type VectorEnd = (&'static [Interval], Option<&'static [Interval]>);

/// One combine vector: the path ends, the class, and the written set or the
/// refusal's code.
pub type CombineVector = (
    &'static [VectorEnd],
    i128,
    Result<&'static [Interval], &'static str>,
);

/// `(path ends, class, written set or the refusal's code)` in `int`, shared
/// with `QCore.WrittenSetDerivation.combineVectors` entry for entry. Rows:
/// the epoll channel creator's three exits (pipe's failure exit narrowed to
/// -1, `_mk_event_add`'s narrowed to -1, the store after both); its
/// forwarding caller; an unnarrowed failure exit (it may return 0 with the
/// slot unwritten); a store before the failure exit (the slot holds every
/// `int`, which no declared bound contains); a later aliasing store; no path
/// returning the class; and the POSIX contrast, where pipe's `[0, MAX]`
/// gives `[0, MAX]`.
pub const COMBINE_VECTORS: &[CombineVector] = &[
    (
        &[
            (&[(-1, -1)], None),
            (&[(-1, -1)], None),
            (&[(0, 0)], Some(&[(1, MAX)])),
        ],
        0,
        Ok(&[(1, MAX)]),
    ),
    (&[(&[(-1, 0)], Some(&[(1, MAX)]))], 0, Ok(&[(1, MAX)])),
    (
        &[(&[(-1, 0)], None), (&[(0, 0)], Some(&[(1, MAX)]))],
        0,
        Err("written-set-unwritten"),
    ),
    (&[(&[(0, 0)], Some(&[INT]))], 0, Ok(&[INT])),
    (
        &[
            (&[(0, 0)], Some(&[(1, MAX)])),
            (&[(0, 0)], Some(&[(-1, -1)])),
        ],
        0,
        Ok(&[(-1, -1), (1, MAX)]),
    ),
    (&[(&[(-1, -1)], None)], 0, Err("written-set-no-return")),
    (&[(&[(0, 0)], Some(&[(0, MAX)]))], 0, Ok(&[(0, MAX)])),
];

/// `(narrowed result, callee result set, class the callee states, its
/// written set, resolution)` in `int`, shared with
/// `QCore.WrittenSetDerivation.resolveVectors`: `None` is infeasible,
/// `Some(None)` unknown, `Some(Some(set))` known.
pub type ResolveVector = (
    &'static [Interval],
    &'static [Interval],
    i128,
    &'static [Interval],
    Option<Option<&'static [Interval]>>,
);

/// The resolve vectors: pipe's success edge `ret >= 0` within `{-1, 0}`
/// (known), its failure edge (no class stated), an unnarrowed result (two
/// classes), a narrowing outside the callee's set (infeasible), and a
/// callee that states nothing for the class (unknown).
pub const RESOLVE_VECTORS: &[ResolveVector] = &[
    (
        &[(0, MAX)],
        &[(-1, 0)],
        0,
        &[(1, MAX)],
        Some(Some(&[(1, MAX)])),
    ),
    (&[(INT.0, -1)], &[(-1, 0)], 0, &[(1, MAX)], Some(None)),
    (&[INT], &[(-1, 0)], 0, &[(1, MAX)], Some(None)),
    (&[(1, MAX)], &[(-1, 0)], 0, &[(1, MAX)], None),
    (&[(0, 0)], &[(-1, 0)], -1, &[(1, MAX)], Some(None)),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn set(intervals: &[Interval]) -> ResultSet {
        ResultSet::from_intervals(INT, intervals)
    }

    #[test]
    fn the_combine_vectors_hold() {
        for (ends, class, expected) in COMBINE_VECTORS {
            let ends = ends
                .iter()
                .map(|(returns, slot)| PathEnd {
                    returns: set(returns),
                    slot: slot.map(set),
                })
                .collect::<Vec<_>>();
            let got = combine(INT, *class, &ends);
            match expected {
                Ok(intervals) => assert_eq!(got, Ok(set(intervals)), "{ends:?}"),
                Err(code) => assert_eq!(got.map_err(Refusal::code), Err(*code), "{ends:?}"),
            }
        }
    }

    #[test]
    fn the_resolve_vectors_hold() {
        for (narrowed, results, class, written, expected) in RESOLVE_VECTORS {
            let got = resolve(&set(narrowed), &set(results), |c| {
                (c == *class).then(|| set(written))
            });
            let want = match expected {
                None => Resolution::Infeasible,
                Some(None) => Resolution::Unknown,
                Some(Some(intervals)) => Resolution::Known(set(intervals)),
            };
            assert_eq!(got, want);
        }
    }

    #[test]
    fn narrowing_reads_both_edges() {
        let pipe = set(&[(-1, 0)]);
        assert_eq!(narrow(&pipe, Cmp::Lt, 0, true), set(&[(-1, -1)]));
        assert_eq!(narrow(&pipe, Cmp::Lt, 0, false), set(&[(0, 0)]));
        assert_eq!(narrow(&pipe, Cmp::Ne, 0, true), set(&[(-1, -1)]));
        assert_eq!(narrow(&pipe, Cmp::Eq, -2, false), pipe);
    }

    #[test]
    fn an_aliasing_store_widens_a_known_slot_only() {
        let fd = set(&[(1, MAX)]);
        assert_eq!(after_store(None, &fd), None);
        assert_eq!(
            after_store(Some(&fd), &set(&[(-1, -1)])),
            Some(set(&[(-1, -1), (1, MAX)]))
        );
    }
}
