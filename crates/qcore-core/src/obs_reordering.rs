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

//! `qcore.obs-reordering.v1` — comparing an observation stream up to a
//! **declared** reordering (card `OBS-REORDERING`).
//!
//! Design record: `docs/design/OBS_REORDERING_V1.md`. That document is the
//! normative statement; this module is its decision procedure.
//!
//! # What this is
//!
//! Every route in this repository that writes compares a native run's bytes with
//! a reference execution's rendering **byte for byte, per stream**. That is
//! well-defined exactly while each stream is written by one thread. This module
//! is what makes a comparison well-defined when it is not: a stream is a
//! sequence of **records**, each carrying the *emitter* that wrote it and the
//! *window* it was written in, and two streams are related when they agree
//! window by window and emitter by emitter.
//!
//! # The relation is declared, never inferred
//!
//! An [`AdmittedReordering`] is the **reference** stream, annotated from the
//! checked model's own structure. It is constructed only through
//! [`AdmittedReordering::declare`], which re-checks every premise it will later
//! demand, so an unvalidated sequence cannot become a constraint set that admits
//! anything. It is never derived from, adjusted to, or validated against a
//! native run: a native run is only ever compared to it.
//!
//! # Emitter — and why it is the opposite of F02's attribution
//!
//! [`crate::task_trace_relation`] attributes an observed line to *the task whose
//! state change it reports* (its premise P2), because it relates **causal**
//! order. This relation attributes a record to *the thread that performed the
//! write*, because it relates **byte** order, and bytes are ordered by writers.
//! The two disagree on exactly one family of records — a worker's buffered trace
//! events, replayed by the parent inside `qcore_task_collect` — and each is
//! right for its own question.
//!
//! # What is not claimed
//!
//! * **Not a memory model.** Nothing here is a statement about visibility,
//!   atomicity, data races, or the ordering of anything but complete records. No
//!   TSan claim is made or implied.
//! * **No cross-emitter order is claimed.** Inside a window this relation asserts
//!   the *absence* of order between distinct emitters. It does not assert that
//!   any particular interleaving is reachable or was taken.
//! * **No cross-stream order is claimed**, exactly as on every other route that
//!   writes.
//! * **Premise `C11StreamLock`.** A record is a whole line because one `fprintf`
//!   runs under the stream's own lock (C11 7.21.2p7–p8), so two threads writing
//!   through `QCORE_IO_WRITE` cannot tear one line into another. A stream
//!   without that lock is outside this relation and no attempt is made to detect
//!   one.
//! * **An accepted run is one run.** Acceptance says that run's stream lies in
//!   the admitted set; it says nothing about the other admitted orders being
//!   reachable, and repeating the comparison is not a coverage claim.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Identity of this relation, bound into recorded evidence so that a changed
/// relation retires the record instead of silently reinterpreting it. The same
/// discipline `TRACE_RELATION_IDENTITY` applies to F02.
pub const OBS_REORDERING_IDENTITY: &str = "qcore.obs-reordering.v1";

/// The thread of control that wrote a record's bytes.
///
/// Not the task whose state the record reports: see the module header. A
/// worker's trace events are replayed by the parent, and are therefore
/// [`Emitter::Parent`] records here while being the child's actions in F02.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Emitter {
    /// The thread that runs the entry function.
    Parent,
    /// A spawned worker, by its runtime task identity.
    Worker(u64),
}

impl fmt::Display for Emitter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parent => write!(f, "parent"),
            Self::Worker(task) => write!(f, "worker {task}"),
        }
    }
}

/// One record of a declared stream: who wrote it, in which window, and what it
/// said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredRecord {
    pub emitter: Emitter,
    /// The concurrency window, assigned by the derivation. Window indices are
    /// non-decreasing along the stream; a record alone in its window is a
    /// **barrier** and commutes with nothing.
    pub window: usize,
    /// The bytes of the line, **without** its terminating newline.
    pub text: String,
}

/// What a stream, or a declaration, violated. Reported in a fixed conjunct
/// order, so a classification is stable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReorderingRefusal {
    /// D1 — a declared record sequence whose windows do not increase along the
    /// stream. This derivation cannot produce one; a hand-built one is refused.
    WindowsNotMonotone { index: usize },
    /// D2 — two distinct emitters of one window would write the same text, so a
    /// native line's writer is not a function of its bytes. Refused **when the
    /// relation is declared**, before any run exists: the alternative is a
    /// silent weakening to a multiset comparison.
    AmbiguousAlphabet { window: usize, text: String },
    /// C1 — the observed stream does not carry the declared number of records.
    LengthMismatch { declared: usize, observed: usize },
    /// C2 — a line no emitter of its window could write. The residue case.
    Residue {
        window: usize,
        index: usize,
        text: String,
    },
    /// C3 — one emitter's own lines are not in that emitter's order, or are not
    /// that emitter's lines.
    EmitterOrder { window: usize, emitter: Emitter },
}

impl ReorderingRefusal {
    /// A stable numeric class, so a Lean transcription and a Rust run can be
    /// compared class for class (the F02 precedent).
    pub fn code(&self) -> u32 {
        match self {
            Self::WindowsNotMonotone { .. } => 1,
            Self::AmbiguousAlphabet { .. } => 2,
            Self::LengthMismatch { .. } => 3,
            Self::Residue { .. } => 4,
            Self::EmitterOrder { .. } => 5,
        }
    }
}

impl fmt::Display for ReorderingRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WindowsNotMonotone { index } => write!(
                f,
                "declared record {index} moves the stream to an earlier window"
            ),
            Self::AmbiguousAlphabet { window, text } => write!(
                f,
                "window {window} admits two emitters that would both write {text:?}, so a line's \
                 writer is not a function of its bytes and per-emitter order is not checkable"
            ),
            Self::LengthMismatch { declared, observed } => write!(
                f,
                "the observed stream carries {observed} records where the relation declares \
                 {declared}"
            ),
            Self::Residue {
                window,
                index,
                text,
            } => write!(
                f,
                "record {index} of window {window} is {text:?}, which no emitter admitted in that \
                 window writes"
            ),
            Self::EmitterOrder { window, emitter } => write!(
                f,
                "the {emitter} lines of window {window} are not the declared lines in the declared \
                 order"
            ),
        }
    }
}

impl std::error::Error for ReorderingRefusal {}

/// One window of a declared stream: a half-open range of record indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WindowSpan {
    id: usize,
    start: usize,
    len: usize,
}

/// A declared stream together with the reorderings it admits.
///
/// Constructible only through [`AdmittedReordering::declare`], so an unchecked
/// sequence cannot become a constraint set.
///
/// ```compile_fail
/// use qcore_core::obs_reordering::{AdmittedReordering, DeclaredRecord};
/// fn forge(records: Vec<DeclaredRecord>) -> AdmittedReordering {
///     AdmittedReordering { records }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedReordering {
    records: Vec<DeclaredRecord>,
    windows: Vec<WindowSpan>,
}

/// The witness [`AdmittedReordering::compare`] returns: for each observed
/// position, the position the same record occupies in the declared stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReorderingWitness {
    pub declared_positions: Vec<usize>,
    /// The windows in which the observed order is not the declared order.
    pub reordered_windows: Vec<usize>,
}

impl ReorderingWitness {
    /// Whether the observed stream is literally the declared order. The point of
    /// this capability is that [`AdmittedReordering::compare`] succeeds without
    /// this being true — and, on a strict relation, that it can only succeed
    /// when it is.
    pub fn is_declared_order(&self) -> bool {
        self.reordered_windows.is_empty()
    }
}

/// The canonical form of one window: the emitters in a fixed total order, each
/// with its own lines in its own order.
pub type WindowCanonicalForm = Vec<(Emitter, Vec<String>)>;

fn window_spans(records: &[DeclaredRecord]) -> Result<Vec<WindowSpan>, ReorderingRefusal> {
    let mut windows: Vec<WindowSpan> = Vec::new();
    for (index, record) in records.iter().enumerate() {
        match windows.last_mut() {
            Some(last) if last.id == record.window => last.len += 1,
            Some(last) if last.id > record.window => {
                return Err(ReorderingRefusal::WindowsNotMonotone { index });
            }
            _ => windows.push(WindowSpan {
                id: record.window,
                start: index,
                len: 1,
            }),
        }
    }
    Ok(windows)
}

impl AdmittedReordering {
    /// Declare a reference stream and the reorderings it admits.
    ///
    /// Both premises the comparison will rest on are demanded here, of the
    /// declaration itself: windows increase along the stream, and no two
    /// emitters of one window share a line's bytes. A declaration that fails
    /// either is refused, so a malformed relation cannot become a vacuous
    /// constraint set.
    pub fn declare(records: Vec<DeclaredRecord>) -> Result<Self, ReorderingRefusal> {
        let windows = window_spans(&records)?;
        for window in &windows {
            let slice = &records[window.start..window.start + window.len];
            let mut writers: BTreeMap<&str, Emitter> = BTreeMap::new();
            for record in slice {
                match writers.get(record.text.as_str()) {
                    Some(&emitter) if emitter != record.emitter => {
                        return Err(ReorderingRefusal::AmbiguousAlphabet {
                            window: window.id,
                            text: record.text.clone(),
                        });
                    }
                    _ => {
                        writers.insert(record.text.as_str(), record.emitter);
                    }
                }
            }
        }
        Ok(Self { records, windows })
    }

    pub fn records(&self) -> &[DeclaredRecord] {
        &self.records
    }

    /// The window lengths, in window order. Two `R`-equivalent streams have the
    /// same sizes by definition, which is what makes the decomposition of an
    /// observed stream unique and the comparison a single left-to-right pass.
    pub fn window_sizes(&self) -> Vec<usize> {
        self.windows.iter().map(|window| window.len).collect()
    }

    /// The **barriers**: the records alone in their window, which commute with
    /// nothing. On a program that spawns nothing every record is a barrier.
    pub fn barriers(&self) -> Vec<&DeclaredRecord> {
        self.windows
            .iter()
            .filter(|window| window.len == 1)
            .map(|window| &self.records[window.start])
            .collect()
    }

    /// The emitters this stream declares.
    pub fn emitters(&self) -> BTreeSet<Emitter> {
        self.records.iter().map(|record| record.emitter).collect()
    }

    /// Whether this relation admits **nothing** — every window has at most one
    /// emitter, so the only stream equivalent to this one is this one.
    ///
    /// Every route in this repository before card `OBS-REORDERING` declares a
    /// strict relation, and for those [`Self::compare`] is byte equality. The
    /// existing comparison is not replaced by this one; it is this one, at the
    /// relation that admits nothing.
    pub fn is_strict(&self) -> bool {
        self.windows.iter().all(|window| {
            self.records[window.start..window.start + window.len]
                .iter()
                .map(|record| record.emitter)
                .collect::<BTreeSet<_>>()
                .len()
                <= 1
        })
    }

    /// The declared stream's own bytes, newline-terminated — the rendering a
    /// strict comparison would have used.
    pub fn declared_text(&self) -> String {
        let mut text = String::new();
        for record in &self.records {
            text.push_str(&record.text);
            text.push('\n');
        }
        text
    }

    /// The canonical form: per window, each emitter's own lines, with the
    /// emitters in a fixed total order.
    ///
    /// `s ≡ t` exactly when their window sizes and canonical forms agree, which
    /// is the "sort the commuting window, byte-compare within an emitter" rule
    /// stated as an equation.
    pub fn canonical(&self) -> Vec<WindowCanonicalForm> {
        self.windows
            .iter()
            .map(|window| {
                canonical_window(
                    self.records[window.start..window.start + window.len]
                        .iter()
                        .map(|record| (record.emitter, record.text.clone())),
                )
            })
            .collect()
    }

    /// Compare an observed stream — a native run's lines, in the order the run
    /// wrote them — with this relation.
    ///
    /// Success means the observed stream carries, in every window, exactly the
    /// declared lines of every emitter in that emitter's declared order. It does
    /// **not** mean the observed stream is the declared order, and it makes no
    /// statement about the schedule the run took beyond what §3.1 of the design
    /// record lists.
    pub fn compare(&self, observed: &[&str]) -> Result<ReorderingWitness, ReorderingRefusal> {
        if observed.len() != self.records.len() {
            return Err(ReorderingRefusal::LengthMismatch {
                declared: self.records.len(),
                observed: observed.len(),
            });
        }
        let mut declared_positions = vec![0usize; observed.len()];
        let mut reordered_windows = Vec::new();
        for window in &self.windows {
            let range = window.start..window.start + window.len;
            let declared = &self.records[range.clone()];
            let seen = &observed[range.clone()];

            // Attribution. `declare` already refused a window in which two
            // emitters share a line's bytes, so this map is a function.
            let mut writers: BTreeMap<&str, Emitter> = BTreeMap::new();
            for record in declared {
                writers.insert(record.text.as_str(), record.emitter);
            }

            // Per emitter, the declared record positions in declared order.
            let mut declared_by_emitter: BTreeMap<Emitter, Vec<usize>> = BTreeMap::new();
            for (offset, record) in declared.iter().enumerate() {
                declared_by_emitter
                    .entry(record.emitter)
                    .or_default()
                    .push(window.start + offset);
            }

            let mut taken: BTreeMap<Emitter, usize> = BTreeMap::new();
            for (offset, text) in seen.iter().enumerate() {
                let Some(&emitter) = writers.get(text) else {
                    return Err(ReorderingRefusal::Residue {
                        window: window.id,
                        index: window.start + offset,
                        text: (*text).to_owned(),
                    });
                };
                let cursor = taken.entry(emitter).or_default();
                let positions = &declared_by_emitter[&emitter];
                // More of this emitter's lines than it declared, or a line whose
                // bytes are not the ones it declared at this point of its own
                // order. Both are the same violation: this emitter's projection
                // is not the declared one.
                if *cursor >= positions.len() || self.records[positions[*cursor]].text != **text {
                    return Err(ReorderingRefusal::EmitterOrder {
                        window: window.id,
                        emitter,
                    });
                }
                declared_positions[window.start + offset] = positions[*cursor];
                *cursor += 1;
            }
            for (&emitter, positions) in &declared_by_emitter {
                if taken.get(&emitter).copied().unwrap_or(0) != positions.len() {
                    return Err(ReorderingRefusal::EmitterOrder {
                        window: window.id,
                        emitter,
                    });
                }
            }
            if declared_positions[range.clone()]
                .iter()
                .enumerate()
                .any(|(offset, position)| window.start + offset != *position)
            {
                reordered_windows.push(window.id);
            }
        }
        Ok(ReorderingWitness {
            declared_positions,
            reordered_windows,
        })
    }

    /// [`Self::compare`] over the raw bytes of a stream. Empty trailing lines
    /// are not records; a `\r\n` line ending is read as a line ending.
    pub fn compare_text(&self, observed: &str) -> Result<ReorderingWitness, ReorderingRefusal> {
        let lines: Vec<&str> = observed
            .lines()
            .map(|line| line.trim_end_matches('\r'))
            .collect();
        self.compare(&lines)
    }

    /// Every stream this relation admits, as newline-terminated text. Bounded on
    /// purpose, on the F02 precedent: the caller supplies the bound and gets
    /// `None` when the enumeration would exceed it, rather than an answer of
    /// unknown cost.
    ///
    /// This exists for evidence — "how much order does this relation actually
    /// give up" — and for tests that must exhibit an admitted order other than
    /// the declared one. Nothing in the comparison path calls it.
    pub fn admitted_streams(&self, bound: usize) -> Option<Vec<String>> {
        let mut all: Vec<Vec<&str>> = vec![Vec::new()];
        for window in &self.windows {
            let declared = &self.records[window.start..window.start + window.len];
            let mut per_emitter: BTreeMap<Emitter, Vec<&str>> = BTreeMap::new();
            for record in declared {
                per_emitter
                    .entry(record.emitter)
                    .or_default()
                    .push(record.text.as_str());
            }
            let interleavings = interleave(&per_emitter.values().cloned().collect::<Vec<_>>());
            let mut next = Vec::new();
            for prefix in &all {
                for tail in &interleavings {
                    if next.len() >= bound {
                        return None;
                    }
                    let mut stream = prefix.clone();
                    stream.extend(tail.iter().copied());
                    next.push(stream);
                }
            }
            all = next;
        }
        Some(
            all.into_iter()
                .map(|stream| {
                    stream
                        .into_iter()
                        .map(|line| format!("{line}\n"))
                        .collect::<String>()
                })
                .collect(),
        )
    }
}

/// The canonical form of one window's records.
fn canonical_window(
    records: impl Iterator<Item = (Emitter, String)>,
) -> Vec<(Emitter, Vec<String>)> {
    let mut grouped: BTreeMap<Emitter, Vec<String>> = BTreeMap::new();
    for (emitter, text) in records {
        grouped.entry(emitter).or_default().push(text);
    }
    grouped.into_iter().collect()
}

/// Every interleaving of a family of sequences that preserves each sequence's
/// own order. Used only by [`AdmittedReordering::admitted_streams`].
fn interleave<'a>(sequences: &[Vec<&'a str>]) -> Vec<Vec<&'a str>> {
    fn walk<'a>(
        sequences: &[Vec<&'a str>],
        cursors: &mut Vec<usize>,
        prefix: &mut Vec<&'a str>,
        total: usize,
        out: &mut Vec<Vec<&'a str>>,
    ) {
        if prefix.len() == total {
            out.push(prefix.clone());
            return;
        }
        for index in 0..sequences.len() {
            if cursors[index] >= sequences[index].len() {
                continue;
            }
            prefix.push(sequences[index][cursors[index]]);
            cursors[index] += 1;
            walk(sequences, cursors, prefix, total, out);
            cursors[index] -= 1;
            prefix.pop();
        }
    }
    let total = sequences.iter().map(Vec::len).sum();
    let mut out = Vec::new();
    walk(
        sequences,
        &mut vec![0; sequences.len()],
        &mut Vec::new(),
        total,
        &mut out,
    );
    out
}

/// The canonical form of an observed stream, attributed through a declared
/// relation. `Ok(a) == Ok(b)` for two observed streams exactly when they are
/// `R`-equivalent — the equational characterisation, exposed so a consumer can
/// state it rather than take [`AdmittedReordering::compare`]'s word.
pub fn canonical_of_observed(
    order: &AdmittedReordering,
    observed: &[&str],
) -> Result<Vec<WindowCanonicalForm>, ReorderingRefusal> {
    order.compare(observed)?;
    let mut windows = Vec::new();
    for window in &order.windows {
        let range = window.start..window.start + window.len;
        let mut writers: BTreeMap<&str, Emitter> = BTreeMap::new();
        for record in &order.records[range.clone()] {
            writers.insert(record.text.as_str(), record.emitter);
        }
        windows.push(canonical_window(
            observed[range]
                .iter()
                .map(|text| (writers[text], (*text).to_owned())),
        ));
    }
    Ok(windows)
}

#[cfg(test)]
#[path = "../tests/unit/obs_reordering.rs"]
mod tests;
