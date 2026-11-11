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

//! The null-dereference property's pure decisions (card
//! **NULL-SAFETY-PROPERTY**, contract specification C09.3, gateway rules
//! 14k–14p).
//!
//! This module decides, for one pointer value or one function, nothing but
//! whether a dereference may meet a null pointer:
//!
//! * a pointer's **nullness** at a program point ([`Nullness`]): non-null with
//!   the reasons that prove it on every reaching path, definitely null, or
//!   possibly null with the sources that may make it so; the join of two
//!   reaching paths is [`Nullness::join`];
//! * a **check** at a dereference ([`check`]): a non-null pointer is
//!   discharged (proved, or moved to an earlier dereference), a null one is a
//!   violation, a possibly-null one a candidate;
//! * the **dominating dereference** of user decision 1 ([`after_dereference`]):
//!   once a real read or write through `B` was made, `B` is non-null with its
//!   question moved to that dereference, until `B` is reassigned;
//! * an **offset** `B ± n` is non-null exactly when `B` is ([`offset`]): C11
//!   6.5.6p8 leaves arithmetic on a null pointer undefined, and an offset of a
//!   pointer to an object stays inside or one past it;
//! * a **guard** ([`refine`]): the arm on which a NULL test of `B` says
//!   "non-null" proves `B` there, the other arm makes it null;
//! * the function's **null verdict** ([`null_verdict`]) and the
//!   **composition** with custody ([`compose`]);
//! * the **custody premise** a non-null reading leaves behind
//!   ([`premise_name`]), which a custody verdict lists until this property
//!   proves the check it names;
//! * the **inference** of the clause `quant null-deref #B;` from a body
//!   ([`First`]): a formal is inferred only when every terminating path
//!   dereferences it before any NULL test of it.
//!
//! What it is not: a model of C, of Clang or of the callees' bodies. It never
//! assumes the absence of a null dereference to discharge one, except through
//! the dominating dereference, whose own check stays in the inventory. The
//! Lean model is `QCore.NullDeref`, bound to this module by shared vectors
//! (`crates/qcore-core/tests/null_deref.rs`).

use std::collections::BTreeSet;

/// The property this module decides. Reported separately from custody.
pub const NULL_PROPERTY: &str = "c.null.deref.v1";

/// The version of the decisions below; part of every null row's evidence
/// identity.
pub const NULL_MODEL_VERSION: &str = "qcore.null.model.v1";

/// The prefix of a custody premise's name (rule 14n).
pub const PREMISE_PREFIX: &str = "null-deref";

/// Where a possibly-null value may come from (the candidate's source kind).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    /// The result of a callee whose contract states an owned, nullable result
    /// (`malloc`, `calloc`, `realloc`, `strdup` and wrappers declared so).
    Allocation(String),
    /// A callee result declared nullable that is not an allocation.
    NullableResult(String),
    /// A derived nullable result: a lookup that may fail (`strchr`).
    Lookup(String),
    /// The result of a callee without a resource contract.
    UncontractedResult(String),
    /// A formal not declared (or inferred) non-null.
    Formal(String),
    /// A pointer read from memory: a member, an element or `*pp`.
    MemoryField(String),
    /// A global (or an address-taken local) read from memory.
    Global(String),
    /// A null pointer constant on some path.
    NullLiteral,
    /// A pointer read before any assignment.
    Uninitialized(String),
    /// Any other form (`va_arg`, an integer converted to a pointer, a label
    /// reached from a later `goto`, a loop the analysis widened).
    Other(String),
}

impl Source {
    /// The kind, for ranking.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Allocation(_) => "allocation",
            Self::NullableResult(_) => "nullable-result",
            Self::Lookup(_) => "lookup",
            Self::UncontractedResult(_) => "uncontracted-result",
            Self::Formal(_) => "formal",
            Self::MemoryField(_) => "memory-field",
            Self::Global(_) => "global",
            Self::NullLiteral => "null-literal",
            Self::Uninitialized(_) => "uninitialized",
            Self::Other(_) => "other",
        }
    }

    /// The source as its report text: `kind` or `kind:what`.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Allocation(what)
            | Self::NullableResult(what)
            | Self::Lookup(what)
            | Self::UncontractedResult(what)
            | Self::Formal(what)
            | Self::MemoryField(what)
            | Self::Global(what)
            | Self::Uninitialized(what)
            | Self::Other(what) => format!("{}:{what}", self.kind()),
            Self::NullLiteral => self.kind().to_owned(),
        }
    }
}

/// Why a pointer is non-null on a path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Proof {
    /// A NULL test's non-null arm, at a line.
    Tested(usize),
    /// `assert(B)`'s surviving arm, at a line: a premise that the deployed
    /// build keeps the assertion (`NDEBUG` absent, as in the analysed text).
    Asserted(usize),
    /// The function's own null-deref precondition (declared or inferred).
    Precondition(String),
    /// A callee contract or validated summary stating a non-null result.
    NonNullResult(String),
    /// An address of an object, an array decay, a string literal or a
    /// function designator.
    Address,
    /// A dominating dereference: the check with this id. The question is
    /// moved there (user decision 1).
    Moved(usize),
    /// Card NULL-TRIAGE-UNRESOLVED-PATHS (rule 15t): the path search explored
    /// every path of the body and none reaches the dereference with the
    /// pointer NULL (the check's other proofs name the declarations the
    /// search read).
    Infeasible,
}

/// A pointer's nullness at a program point.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nullness {
    /// Non-null on every reaching path, for these reasons.
    NonNull(BTreeSet<Proof>),
    /// Null on every reaching path.
    Null,
    /// Possibly null, from these sources.
    Maybe(BTreeSet<Source>),
}

impl Nullness {
    /// A non-null value for one reason.
    #[must_use]
    pub fn proved(proof: Proof) -> Self {
        Self::NonNull(BTreeSet::from([proof]))
    }

    /// A possibly-null value from one source.
    #[must_use]
    pub fn maybe(source: Source) -> Self {
        Self::Maybe(BTreeSet::from([source]))
    }

    /// The join of two reaching paths: non-null only when both are, null only
    /// when both are, otherwise possibly null with every source either path
    /// carries (a null path contributes [`Source::NullLiteral`]).
    #[must_use]
    pub fn join(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::NonNull(a), Self::NonNull(b)) => Self::NonNull(a.union(b).cloned().collect()),
            (Self::Null, Self::Null) => Self::Null,
            (Self::Maybe(a), Self::Maybe(b)) => Self::Maybe(a.union(b).cloned().collect()),
            (Self::Maybe(a), Self::NonNull(_)) | (Self::NonNull(_), Self::Maybe(a)) => {
                Self::Maybe(a.clone())
            }
            (Self::Maybe(a), Self::Null) | (Self::Null, Self::Maybe(a)) => {
                let mut sources = a.clone();
                sources.insert(Source::NullLiteral);
                Self::Maybe(sources)
            }
            (Self::NonNull(_), Self::Null) | (Self::Null, Self::NonNull(_)) => {
                Self::maybe(Source::NullLiteral)
            }
        }
    }

    /// Whether every reaching path has it non-null.
    #[must_use]
    pub fn is_non_null(&self) -> bool {
        matches!(self, Self::NonNull(_))
    }
}

/// What one check decided.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    /// Non-null on every path, by proofs none of which is a moved question.
    Proved(BTreeSet<Proof>),
    /// Non-null on every path, with at least one path's question moved to an
    /// earlier dereference (whose own check stays in the inventory).
    Moved(BTreeSet<Proof>),
    /// Discharged only by a held clause (named); `Pending` at most.
    Held(String),
    /// Possibly null: a NULL-dereference candidate, with its sources.
    Candidate(BTreeSet<Source>),
    /// Null on every path; `reachable` when the consumer validated that the
    /// dereference is reached from entry.
    Violation { reachable: bool },
    /// Proved unreachable by an explicit check; kept, never dropped.
    Excluded,
}

impl Outcome {
    /// The report spelling of the outcome's class.
    #[must_use]
    pub fn class(&self) -> &'static str {
        match self {
            Self::Proved(_) => "proved",
            Self::Moved(_) => "moved",
            Self::Held(_) => "held",
            Self::Candidate(_) => "candidate",
            Self::Violation { reachable: true } => "violation",
            Self::Violation { reachable: false } => "violation-reachability-unresolved",
            Self::Excluded => "excluded",
        }
    }

    /// Whether the check accuses nothing and leaves no obstacle.
    #[must_use]
    pub fn discharged(&self) -> bool {
        matches!(self, Self::Proved(_) | Self::Moved(_) | Self::Excluded)
    }
}

/// The check at one dereference of a pointer whose nullness is `value`.
#[must_use]
pub fn check(value: &Nullness, reachable: bool) -> Outcome {
    match value {
        Nullness::NonNull(proofs) => {
            if proofs.iter().any(|p| matches!(p, Proof::Moved(_))) {
                Outcome::Moved(proofs.clone())
            } else {
                Outcome::Proved(proofs.clone())
            }
        }
        Nullness::Null => Outcome::Violation { reachable },
        Nullness::Maybe(sources) => Outcome::Candidate(sources.clone()),
    }
}

/// The pointer after a real read or write through it at check `site`
/// (user decision 1): non-null, its question moved to `site` on every path
/// on which it was not already non-null. A pointer already non-null keeps
/// its proofs (the dereference adds nothing a reader needs).
#[must_use]
pub fn after_dereference(value: &Nullness, site: usize) -> Nullness {
    match value {
        Nullness::NonNull(_) => value.clone(),
        Nullness::Null | Nullness::Maybe(_) => Nullness::proved(Proof::Moved(site)),
    }
}

/// `B ± n`: non-null exactly when `B` is, null-ness otherwise unknown (an
/// offset of a null pointer is undefined, so a null `B` gives no value).
#[must_use]
pub fn offset(value: &Nullness) -> Nullness {
    match value {
        Nullness::Null => Nullness::maybe(Source::NullLiteral),
        other => other.clone(),
    }
}

/// A NULL test of a pointer, on one arm: `non_null_arm` is the arm on which
/// the test says the pointer is not null. `None` when the arm is infeasible
/// (a pointer the arm contradicts).
#[must_use]
pub fn refine(value: &Nullness, non_null_arm: bool, proof: Proof) -> Option<Nullness> {
    match (value, non_null_arm) {
        (Nullness::NonNull(_), true) => Some(value.clone()),
        (Nullness::NonNull(_), false) => None,
        (Nullness::Null, true) => None,
        (Nullness::Null, false) => Some(Nullness::Null),
        (Nullness::Maybe(_), true) => Some(Nullness::proved(proof)),
        (Nullness::Maybe(_), false) => Some(Nullness::Null),
    }
}

/// The four values, ordered `Safe < Pending < Unknown < Unsafe` (language
/// specification §L21.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NullVerdict {
    Safe,
    Pending,
    Unknown,
    Unsafe,
}

impl NullVerdict {
    #[must_use]
    pub fn join(self, other: Self) -> Self {
        self.max(other)
    }
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Pending => "Pending",
            Self::Unknown => "Unknown",
            Self::Unsafe => "Unsafe",
        }
    }
}

/// The reason a candidate contributes.
pub const CANDIDATE: &str = "null-deref-candidate";
/// A violation whose reachability the consumer did not establish.
pub const UNREACHED_VIOLATION: &str = "null-violation-reachability-unresolved";
/// The reason a held clause contributes.
pub const AWAITING_REVIEW: &str = "awaiting-human-review";

/// The null verdict of one function, from every check's outcome and the
/// function-level reasons (unread forms, an import failure). Returns the
/// verdict and its reasons.
///
/// * `Unsafe` only for a reachable violation;
/// * `Safe` only when every check is proved, moved or excluded and no
///   function-level reason remains (a moved check's origin is itself a check
///   of the same function, so its own outcome is counted);
/// * `Pending` only when held clauses are the sole obstacle;
/// * `Unknown` otherwise (a candidate is a reason, never an accusation).
#[must_use]
pub fn null_verdict(
    outcomes: &[Outcome],
    function_reasons: &[String],
) -> (NullVerdict, Vec<String>) {
    let mut reasons = function_reasons.to_vec();
    if outcomes.contains(&Outcome::Violation { reachable: true }) {
        return (NullVerdict::Unsafe, reasons);
    }
    let mut held = false;
    let mut other = !function_reasons.is_empty();
    for outcome in outcomes {
        match outcome {
            Outcome::Proved(_) | Outcome::Moved(_) | Outcome::Excluded => {}
            Outcome::Held(clause) => {
                held = true;
                let reason = format!("{AWAITING_REVIEW}: {clause}");
                if !reasons.contains(&reason) {
                    reasons.push(reason);
                }
            }
            Outcome::Candidate(_) => {
                other = true;
                if !reasons.iter().any(|r| r == CANDIDATE) {
                    reasons.push(CANDIDATE.to_owned());
                }
            }
            Outcome::Violation { .. } => {
                other = true;
                if !reasons.iter().any(|r| r == UNREACHED_VIOLATION) {
                    reasons.push(UNREACHED_VIOLATION.to_owned());
                }
            }
        }
    }
    let verdict = match (held, other) {
        (_, true) => NullVerdict::Unknown,
        (true, false) => NullVerdict::Pending,
        (false, false) => NullVerdict::Safe,
    };
    (verdict, reasons)
}

/// Which properties a request names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub custody: bool,
    pub null: bool,
}

/// The combined verdict of a request: the join of every requested row; a
/// requested row that is absent is `Unknown`, never omitted. A custody row
/// resting on a null premise is `Safe` here only when the null row is.
#[must_use]
pub fn compose(
    request: Request,
    custody: Option<NullVerdict>,
    null: Option<NullVerdict>,
) -> NullVerdict {
    let row = |requested: bool, value: Option<NullVerdict>| {
        if requested {
            value.unwrap_or(NullVerdict::Unknown)
        } else {
            NullVerdict::Safe
        }
    };
    if !request.custody && !request.null {
        return NullVerdict::Unknown;
    }
    row(request.custody, custody).join(row(request.null, null))
}

/// The name of the custody premise a non-null reading of `variable` at
/// `line` of `function` leaves (rule 14n).
#[must_use]
pub fn premise_name(function: &str, line: usize, variable: &str) -> String {
    format!("{PREMISE_PREFIX}:{function}:{line}:{variable}")
}

/// Card NULL-SAFETY-PROPERTY (rule 14p): what a path did first with a
/// formal's entry value, for the inference of `quant null-deref #B;`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum First {
    /// Neither dereferenced nor tested yet.
    Pending,
    /// Dereferenced before any NULL test.
    Dereferenced,
    /// Tested (or reassigned, or escaped to an unread position) first.
    Other,
}

impl First {
    /// The effect of a dereference of the formal's value.
    #[must_use]
    pub fn dereference(self) -> Self {
        match self {
            Self::Pending => Self::Dereferenced,
            other => other,
        }
    }

    /// The effect of a NULL test of it, or of losing track of it.
    #[must_use]
    pub fn test(self) -> Self {
        match self {
            Self::Pending => Self::Other,
            other => other,
        }
    }
}

/// Whether the clause is inferred: at least one terminating path, and every
/// terminating path dereferenced the formal before anything else.
#[must_use]
pub fn inferred(exits: &BTreeSet<First>) -> bool {
    !exits.is_empty() && exits.iter().all(|first| *first == First::Dereferenced)
}

/// Card NULL-SAFETY-PROPERTY (rule 14o): what the body check reports of a
/// declared clause on its own body. `necessary` when every terminating path
/// dereferences the formal before any test; `stronger-than-necessary`
/// otherwise (a test first, or a path without a dereference): a precondition
/// stronger than the body needs is never wrong, it only asks more of the
/// callers.
#[must_use]
pub fn clause_standing(exits: &BTreeSet<First>) -> &'static str {
    if inferred(exits) {
        "necessary"
    } else {
        "stronger-than-necessary"
    }
}
