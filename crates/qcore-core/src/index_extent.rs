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

//! The index/access-bounds property's pure decisions (card
//! **CG16-INDEX-AND-OFFSET-EXTENTS**, contract specification C09.1, gateway
//! rule 11s).
//!
//! This module decides, for one access or one function, nothing but bounds:
//!
//! * an **element access** `base[i]` against an extent of `N` elements is in
//!   bounds exactly when `0 <= i < N` ([`element_access`]); an empty extent
//!   admits no access;
//! * a **formation** `base + i` is valid exactly when `0 <= i <= N`
//!   ([`formation`]): one past the end may be formed but never accessed;
//! * a **byte access** of width `w` at offset `o` in an extent of `B` bytes is
//!   in bounds exactly when `0 <= o`, `0 < w` and `o + w <= B`
//!   ([`byte_access`]), with the end computed by checked arithmetic bounded by
//!   the recorded target's object-size limit, and `element_size * index` by
//!   [`scale`] under the same limit;
//! * a **same-object** obligation of pointer subtraction and ordering
//!   ([`same_object`]);
//! * how a **source-established extent** and a **declared extent** combine
//!   ([`resolve_extent`]): a declaration never enlarges, shrinks or erases a
//!   source-established object; any difference is a reported contradiction;
//! * **fact invalidation** ([`Facts`]): a write to a variable withdraws every
//!   fact about it and every fact whose count names it;
//! * the function's **bounds verdict** under the review-hold standing rule
//!   ([`bounds_verdict`]) and the **composition** with custody
//!   ([`compose`]).
//!
//! What it is not: a model of C, of Clang, or of the declared external bodies.
//! It never assumes the absence of out-of-bounds behaviour to discharge a
//! bounds obligation, and it does not evaluate C arithmetic beyond the checked
//! operations named above (general C integer semantics are card CG20's). A
//! range fact confers no liveness, ownership, alignment, initialization,
//! effective-type or synchronization guarantee. The Lean model is
//! `QCore.IndexExtent`, bound to this module by shared vectors
//! (`crates/qcore-core/tests/index_extent.rs`).

use std::collections::BTreeMap;

/// The property this module decides. Reported separately from custody.
pub const BOUNDS_PROPERTY: &str = "c.bounds.index-extent.v1";

/// The version of the decisions below; part of every bounds row's evidence
/// identity, so a changed decision procedure cannot reuse an old admission.
pub const BOUNDS_MODEL_VERSION: &str = "qcore.bounds.model.v1";

/// What an extent counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Unit {
    Elements,
    Bytes,
}

impl Unit {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Elements => "elements",
            Self::Bytes => "bytes",
        }
    }
}

/// A count: a constant, or a **stable symbol** — the value a named parameter
/// had at entry, valid only while that parameter is never written.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Count {
    Const(u128),
    Symbol(String),
}

impl Count {
    fn mentions(&self, name: &str) -> bool {
        matches!(self, Self::Symbol(symbol) if symbol == name)
    }
}

/// Where a premise comes from. Only `Source` and `Resolved` are in force in
/// an actual judgement; `Held` is in force only in the hypothetical one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Standing {
    /// Derived and checked by the tool from the bound source/build evidence.
    Source,
    /// A declaration a person resolved by editing its review marker away: a
    /// `Declared` assumption, not a proof of its C body.
    Resolved,
    /// A declaration still marked awaiting human review.
    Held,
}

/// What is known about an index (or an offset, in elements) at a use.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexFact {
    /// Exactly this value.
    Const(i128),
    /// Some value in `[lo, hi)`.
    Range { lo: i128, hi: Count },
    /// Nothing usable (a symbolic or unsupported form): never in bounds.
    Unknown,
}

/// Why a decision was not taken. Each is a named Unknown reason.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Refusal {
    /// No extent is established for the base.
    MissingExtent,
    /// The index or offset has no fact this module can compare.
    UnsupportedRange,
    /// A range is known but does not lie inside the extent for every value.
    RangeNotContained,
    /// A count and an index use different symbols.
    SymbolMismatch,
    /// Checked arithmetic overflowed the target's object-size limit.
    Overflow,
    /// A zero or negative access width.
    EmptyWidth,
    /// A constant range with no value: premises that contradict each other
    /// discharge nothing (no vacuous truth).
    EmptyRange,
}

impl Refusal {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::MissingExtent => "bounds-missing-extent",
            Self::UnsupportedRange => "bounds-unsupported-range",
            Self::RangeNotContained => "bounds-range-not-contained",
            Self::SymbolMismatch => "bounds-symbol-mismatch",
            Self::Overflow => "bounds-arithmetic-overflow",
            Self::EmptyWidth => "bounds-empty-width",
            Self::EmptyRange => "bounds-inconsistent-premises",
        }
    }
}

/// The decision for one obligation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Decision {
    InBounds,
    OutOfBounds,
    Undecided(Refusal),
}

/// `0 <= i < hi` for every value of the fact, against the count `n`.
fn contained(index: &IndexFact, n: &Count, inclusive: bool) -> Decision {
    match (index, n) {
        (IndexFact::Unknown, _) => Decision::Undecided(Refusal::UnsupportedRange),
        (IndexFact::Const(i), Count::Const(n)) => {
            let n = i128::try_from(*n).unwrap_or(i128::MAX);
            let upper_ok = if inclusive { *i <= n } else { *i < n };
            if *i >= 0 && upper_ok {
                Decision::InBounds
            } else {
                Decision::OutOfBounds
            }
        }
        (IndexFact::Const(_), Count::Symbol(_)) => Decision::Undecided(Refusal::SymbolMismatch),
        (IndexFact::Range { lo, hi }, n) => {
            if matches!(hi, Count::Const(hi) if i128::try_from(*hi).unwrap_or(i128::MAX) <= *lo) {
                return Decision::Undecided(Refusal::EmptyRange);
            }
            if *lo < 0 {
                return Decision::Undecided(Refusal::RangeNotContained);
            }
            match (hi, n) {
                // Every value is < hi <= n (or <= n for a formation).
                (Count::Const(hi), Count::Const(n)) => {
                    if hi <= n {
                        Decision::InBounds
                    } else {
                        Decision::Undecided(Refusal::RangeNotContained)
                    }
                }
                (Count::Symbol(hi), Count::Symbol(n)) if hi == n => Decision::InBounds,
                (Count::Symbol(_), _) | (_, Count::Symbol(_)) => {
                    Decision::Undecided(Refusal::SymbolMismatch)
                }
            }
        }
    }
}

/// An element access `base[i]` against `count` elements: `0 <= i < count`.
/// An empty extent admits no access. A missing extent is never in bounds.
pub fn element_access(count: Option<&Count>, index: &IndexFact) -> Decision {
    match count {
        None => Decision::Undecided(Refusal::MissingExtent),
        Some(count) => contained(index, count, false),
    }
}

/// Forming `base + i` against `count` elements: `0 <= i <= count`. The
/// one-past pointer is formed here and refused by [`element_access`].
pub fn formation(count: Option<&Count>, offset: &IndexFact) -> Decision {
    match count {
        None => Decision::Undecided(Refusal::MissingExtent),
        Some(count) => contained(offset, count, true),
    }
}

/// `element_size * index`, checked: the magnitude must stay within `limit`
/// (the target's largest object size, `PTRDIFF_MAX`).
pub fn scale(index: i128, element_size: u128, limit: u128) -> Result<i128, Refusal> {
    if element_size == 0 {
        return Err(Refusal::EmptyWidth);
    }
    let size = i128::try_from(element_size).map_err(|_| Refusal::Overflow)?;
    let product = index.checked_mul(size).ok_or(Refusal::Overflow)?;
    if product.unsigned_abs() > limit {
        return Err(Refusal::Overflow);
    }
    Ok(product)
}

/// A byte access of `width` bytes at `offset` in an extent of `extent` bytes:
/// `0 <= offset`, `0 < width`, `offset + width <= extent`, the end computed
/// with checked arithmetic bounded by `limit`.
pub fn byte_access(extent: u128, offset: i128, width: u128, limit: u128) -> Decision {
    if width == 0 {
        return Decision::Undecided(Refusal::EmptyWidth);
    }
    if offset < 0 {
        return Decision::OutOfBounds;
    }
    let Some(end) = (offset as u128).checked_add(width) else {
        return Decision::Undecided(Refusal::Overflow);
    };
    if end > limit {
        return Decision::Undecided(Refusal::Overflow);
    }
    if end <= extent {
        Decision::InBounds
    } else {
        Decision::OutOfBounds
    }
}

/// The identity of an object or subobject, as the source consumer names it.
pub type ObjectId = String;

/// Pointer subtraction and ordering need both operands in one object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SameObject {
    Same,
    /// Two distinct objects both established by the source.
    Different,
    Unresolved,
}

pub fn same_object(
    left: Option<&ObjectId>,
    right: Option<&ObjectId>,
    distinct: bool,
) -> SameObject {
    match (left, right) {
        (Some(left), Some(right)) if left == right => SameObject::Same,
        (Some(_), Some(_)) if distinct => SameObject::Different,
        _ => SameObject::Unresolved,
    }
}

/// How a source-established extent and a declared one combine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtentResolution {
    /// The source extent stands (any declaration agreed with it exactly).
    Source(u128),
    /// No source extent: the declaration, with its standing.
    Declared(Count, Standing),
    /// The declaration differs from the source: the source extent stands for
    /// every decision and the contradiction is reported and withheld.
    Conflict {
        source: u128,
        declared: Count,
    },
    Missing,
}

impl ExtentResolution {
    /// The count every decision uses, and the standing it rests on.
    pub fn in_force(&self) -> Option<(Count, Standing)> {
        match self {
            Self::Source(n) | Self::Conflict { source: n, .. } => {
                Some((Count::Const(*n), Standing::Source))
            }
            Self::Declared(count, standing) => Some((count.clone(), *standing)),
            Self::Missing => None,
        }
    }
}

/// A declaration never enlarges (or otherwise overrides) a source extent.
pub fn resolve_extent(
    source: Option<u128>,
    declared: Option<(Count, Standing)>,
) -> ExtentResolution {
    match (source, declared) {
        (Some(source), None) => ExtentResolution::Source(source),
        (Some(source), Some((Count::Const(declared), _))) if declared == source => {
            ExtentResolution::Source(source)
        }
        (Some(source), Some((declared, _))) => ExtentResolution::Conflict { source, declared },
        (None, Some((declared, standing))) => ExtentResolution::Declared(declared, standing),
        (None, None) => ExtentResolution::Missing,
    }
}

/// What a pointer variable is known to point into.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PointerFact {
    pub object: ObjectId,
    /// The extent of the object, in `unit`.
    pub count: Count,
    pub unit: Unit,
    /// The offset from the object's start, in elements of the pointer's type.
    pub offset: IndexFact,
    /// The weakest standing among the premises this fact rests on.
    pub standing: Standing,
    /// Established by the source alone: no call can resize or release it.
    pub source_object: bool,
}

impl PointerFact {
    fn mentions(&self, name: &str) -> bool {
        self.count.mentions(name)
            || matches!(&self.offset, IndexFact::Range { hi, .. } if hi.mentions(name))
    }
}

/// A scalar fact and the standing of the premise it rests on.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScalarFact {
    pub value: IndexFact,
    pub standing: Standing,
}

/// The facts in force at one program point, keyed by variable.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub scalars: BTreeMap<String, ScalarFact>,
    pub pointers: BTreeMap<String, PointerFact>,
}

impl Facts {
    /// A write to `name` (an assignment, an increment, an escaped address):
    /// withdraw its own facts and every fact whose count or bound names it.
    pub fn write(&mut self, name: &str) {
        self.scalars.remove(name);
        self.pointers.remove(name);
        self.scalars.retain(
            |_, fact| !matches!(&fact.value, IndexFact::Range { hi, .. } if hi.mentions(name)),
        );
        self.pointers.retain(|_, fact| !fact.mentions(name));
    }

    /// A call that may resize, release or reallocate what it can reach:
    /// withdraw every pointer fact on an object the source did not establish.
    pub fn call(&mut self) {
        self.pointers.retain(|_, fact| fact.source_object);
    }

    /// The facts holding on both incoming edges.
    pub fn join(&self, other: &Self) -> Self {
        Self {
            scalars: self
                .scalars
                .iter()
                .filter(|(name, fact)| other.scalars.get(*name) == Some(fact))
                .map(|(name, fact)| (name.clone(), fact.clone()))
                .collect(),
            pointers: self
                .pointers
                .iter()
                .filter(|(name, fact)| other.pointers.get(*name) == Some(fact))
                .map(|(name, fact)| (name.clone(), fact.clone()))
                .collect(),
        }
    }
}

/// Intersect a range with a new bound pair (the guard's).
pub fn narrow(previous: Option<&IndexFact>, lo: Option<i128>, hi: Option<Count>) -> IndexFact {
    match previous {
        Some(IndexFact::Const(value)) => IndexFact::Const(*value),
        Some(IndexFact::Range {
            lo: old_lo,
            hi: old_hi,
        }) => {
            let lo = lo.map_or(*old_lo, |lo| lo.max(*old_lo));
            let hi = match (hi, old_hi) {
                (Some(Count::Const(new)), Count::Const(old)) => Count::Const(new.min(*old)),
                (Some(new @ Count::Symbol(_)), _) => new,
                (_, old) => old.clone(),
            };
            IndexFact::Range { lo, hi }
        }
        _ => match (lo, hi) {
            (Some(lo), Some(hi)) => IndexFact::Range { lo, hi },
            _ => IndexFact::Unknown,
        },
    }
}

/// One access or call-site obligation, judged twice: with only `Source` and
/// `Resolved` premises in force (`actual`), and with the held declarations
/// also in force as written (`hypothetical`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Judgement {
    pub actual: Outcome,
    pub hypothetical: Outcome,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    InBounds,
    /// Out of bounds under in-force premises; `reachable` when the source
    /// consumer validated that the access is reached.
    Violation {
        reachable: bool,
    },
    /// Discharged only by a held declaration (named).
    Held(Vec<String>),
    /// Anything else unresolved (named).
    Unresolved(String),
    /// Proved unreachable by an explicit check; excluded, never dropped.
    Excluded,
}

/// The four values, ordered `Safe < Pending < Unknown < Unsafe` (language
/// specification §L21.2). Mapped one to one onto `qcore_schemas::Verdict`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoundsVerdict {
    Safe,
    Pending,
    Unknown,
    Unsafe,
}

impl BoundsVerdict {
    pub fn join(self, other: Self) -> Self {
        self.max(other)
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Pending => "Pending",
            Self::Unknown => "Unknown",
            Self::Unsafe => "Unsafe",
        }
    }
}

/// The standing reason a hypothetical violation produces: it rests on a held
/// declaration, so it accuses nothing.
pub const HELD_VIOLATION: &str = "bounds-held-premise-violation";
/// A violation whose reachability the consumer did not establish.
pub const UNREACHED_VIOLATION: &str = "bounds-violation-reachability-unresolved";
/// The reason a held declaration contributes.
pub const AWAITING_REVIEW: &str = "awaiting-human-review";
/// Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the reason a declaration held
/// because its pin is not in force contributes (C06.2's code). A held clause
/// that is a pin hold is rendered with this prefix by its declaration and
/// reported as it is; every other held clause is a review marker's.
pub const CONTRACT_PIN_MISMATCH: &str = "contract-pin-mismatch";

/// The bounds verdict of one function, from every judgement and the
/// function-level reasons (contradicted declarations, import failures, an
/// exhausted analysis bound). Returns the verdict and its reasons.
///
/// * `Unsafe` only for a reachable violation under in-force premises;
/// * `Safe` only when every judgement is in bounds or excluded and no
///   function-level reason remains;
/// * `Pending` only when the held declarations are the sole obstacle **and**
///   putting them in force as written leaves no other obstacle;
/// * `Unknown` otherwise, with every reason retained (a hypothetical
///   violation is a standing reason, not an accusation).
pub fn bounds_verdict(
    judgements: &[Judgement],
    function_reasons: &[String],
) -> (BoundsVerdict, Vec<String>) {
    let mut reasons: Vec<String> = function_reasons.to_vec();
    if judgements
        .iter()
        .any(|j| j.actual == Outcome::Violation { reachable: true })
    {
        return (BoundsVerdict::Unsafe, reasons);
    }
    let mut held = false;
    let mut other = !function_reasons.is_empty();
    for judgement in judgements {
        match &judgement.actual {
            Outcome::InBounds | Outcome::Excluded => {}
            Outcome::Held(clauses) => {
                held = true;
                reasons.extend(clauses.iter().map(|c| {
                    if c.starts_with(CONTRACT_PIN_MISMATCH) {
                        c.clone()
                    } else {
                        format!("{AWAITING_REVIEW}: {c}")
                    }
                }));
            }
            Outcome::Violation { .. } => {
                other = true;
                reasons.push(UNREACHED_VIOLATION.into());
            }
            Outcome::Unresolved(reason) => {
                other = true;
                reasons.push(reason.clone());
            }
        }
    }
    if !held && !other {
        return (BoundsVerdict::Safe, reasons);
    }
    if held && !other {
        // The standing check: the held declarations in force as written.
        let mut standing = Vec::new();
        for judgement in judgements {
            match &judgement.hypothetical {
                Outcome::InBounds | Outcome::Excluded => {}
                Outcome::Violation { .. } => standing.push(HELD_VIOLATION.to_owned()),
                Outcome::Held(clauses) => standing.extend(clauses.iter().cloned()),
                Outcome::Unresolved(reason) => standing.push(reason.clone()),
            }
        }
        if standing.is_empty() {
            return (BoundsVerdict::Pending, reasons);
        }
        reasons.extend(standing);
    }
    (BoundsVerdict::Unknown, reasons)
}

/// Which properties a request names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub custody: bool,
    pub bounds: bool,
}

/// The combined verdict of a request: the join of every requested row; a
/// requested row that is absent is `Unknown`, never omitted.
pub fn compose(
    request: Request,
    custody: Option<BoundsVerdict>,
    bounds: Option<BoundsVerdict>,
) -> BoundsVerdict {
    let row = |requested: bool, value: Option<BoundsVerdict>| {
        if requested {
            value.unwrap_or(BoundsVerdict::Unknown)
        } else {
            BoundsVerdict::Safe
        }
    };
    if !request.custody && !request.bounds {
        return BoundsVerdict::Unknown;
    }
    row(request.custody, custody).join(row(request.bounds, bounds))
}

/// The identity a bounds row is admitted under: property, model version, the
/// source/build identity, the target identity and the bounds contracts.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceIdentity {
    pub property: String,
    pub model_version: String,
    pub source_sha256: String,
    pub target: String,
    pub contracts_sha256: String,
}

/// Admission of a recorded bounds row against a fresh derivation: every
/// identity component equal, the access inventory equal, the verdict equal.
/// Any difference refuses; the denominator keeps the function as `Unknown`.
pub fn admit(
    recorded: (&EvidenceIdentity, usize, BoundsVerdict),
    fresh: (&EvidenceIdentity, usize, BoundsVerdict),
) -> Result<(), &'static str> {
    if recorded.0.property != BOUNDS_PROPERTY || recorded.0.model_version != BOUNDS_MODEL_VERSION {
        return Err("bounds evidence names another property or model version");
    }
    if recorded.0 != fresh.0 {
        return Err("bounds evidence identity differs (source, target or contracts changed)");
    }
    if recorded.1 != fresh.1 {
        return Err("bounds evidence does not cover the selected accesses");
    }
    if recorded.2 != fresh.2 {
        return Err("bounds evidence verdict does not match its derivation");
    }
    Ok(())
}
