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

//! `source.loop.v1` — the checked bounded counted loop (card `LOOP-V1`).
//!
//! # What the construct is, stated before anything else
//!
//! ```text
//! for NAME in 0..BOUND { STATEMENT* }
//! ```
//!
//! `BOUND` is a **check-time-known positive integer** — a literal, or a name the
//! entry bound to one — so the number of iterations is a number the checker
//! holds. `NAME` is a scalar binding scoped to the body, read-only inside it,
//! taking the values `0, 1, …, BOUND - 1` in that order.
//!
//! # The checking route: exact bounded unrolling, and why not a fixed point
//!
//! The bound is known, so the checker **runs the body once per iteration**
//! rather than computing a fixed point over a back edge. That is not the
//! cheaper of two equivalent options; on the routes this construct is admitted
//! on it is the only one compatible with the evidence.
//!
//! A route that admits this construct publishes a certificate whose contents —
//! the bytes a buffer holds, the lines a program writes — are **exact values**,
//! and its evidence is a byte comparison between the reference execution and a
//! native run. A fixed point over a back edge computes a *sound over-approximation*
//! of those values; it is exactly the operation that replaces "byte 3 is 65"
//! with "byte 3 is some byte". The comparison this route rests on cannot be
//! written against an over-approximation, so widening does not make the loop
//! cheaper to check — it makes the loop's observation unpredictable, which is
//! the same wall `STD-BUFFER-VARYING-INDEX` records for a varying index.
//!
//! Unrolling costs what it says: [`MAX_UNROLLED_EVENTS`] bounds the total, and
//! a program above it is refused by name rather than checked slowly.
//!
//! # Per-iteration custody identity, and where it is decided
//!
//! The body's net custody effect must be the **identity**: one iteration must
//! leave the ownership ledger holding exactly the resources, under exactly the
//! names, with exactly the footprints it started with.
//!
//! Most of that discipline is *inherited* rather than written here. An unrolled
//! body that acquires without releasing presents the cycle-3 ownership checker
//! with two acquisitions of one live name, and a body that releases a resource
//! it did not acquire presents it with two releases of one resource; both are
//! refusals that checker already owns. What the gate below adds is that the rule
//! holds at **`BOUND == 1`** too, where unrolling alone would expose nothing —
//! so the rule is the rule it names rather than an artifact of how many times
//! the body happens to run.
//!
//! [`iteration_is_custody_identity`] is written over [`OwnershipOp`] and knows
//! nothing about any route, so a second route admitting this construct inherits
//! the same gate rather than restating it.
//!
//! # What v1 does not model, by name
//!
//! * a bound the checker cannot compute (`LOOP-VARYING-BOUND`);
//! * `break` and `continue` (`LOOP-BREAK`) — an early exit needs the per-exit
//!   cleanup discharge `IF-ARM-RETURN` names, which no lowering on these routes
//!   has;
//! * a loop over a collection of resources (`LOOP-RESOURCE-COLLECTION`) — no
//!   such type exists;
//! * a zero-iteration loop (`LOOP-ZERO-BOUND`), refused because its body would
//!   never be checked and the construct would admit an unchecked statement;
//! * arithmetic on the index as an expression — since card
//!   `W48-QR-SCALAR-TYPES-R5` the admitting route's one-operation value
//!   statement computes on the index (decided at every iteration), and the
//!   bound and index may be of any integer type ([`typed_bound`]); a nested
//!   expression is `W48-QR-VALUE-STATEMENT-EXPRESSIONS`.

use std::collections::BTreeMap;

use crate::ownership::{Footprint, OwnershipError, OwnershipOp, OwnershipState, execute_ownership};
use crate::scalar_types::{RecordedTarget, ScalarType};

/// The largest iteration count this construct admits — **the absolute
/// ceiling**, layer one of the three-layer budget (card `LOOP-UNROLL-BUDGET`).
///
/// Equal to `std_buffer::MAX_EXTENT` on purpose: a traversal of the largest
/// buffer the byte-buffer route admits is expressible, and nothing larger is.
/// A greater bound is refused by name rather than truncated, the same
/// discipline every other quantity on these routes follows.
///
/// **Chosen by measurement, not by symmetry.** At this bound the three
/// non-custody bodies measured by card `LOOP-UNROLL-BUDGET` cost 11–45 ms of
/// check time, and the per-iteration acquire/release body costs 3.0 s once the
/// one-pass peak of [`peak_live_axis`] replaces the prefix rescan — down from a
/// pre-repair cost measured in minutes. The alternative ceiling the card
/// considered, 1024, was rejected because it would **withdraw** the
/// full-extent traversal for a worst case the repair already removes.
pub const MAX_ITERATIONS: u32 = 4096;

/// The absolute ceiling, named for what it is. Layer one of the budget.
///
/// No declaration may license a bound above this, which is what makes the
/// ceiling a ceiling rather than a default with a longer spelling.
pub const UNROLL_CEILING: u32 = MAX_ITERATIONS;

/// The bound a `for` carrying **no** unrolling attribute may reach — layer two,
/// the default budget.
///
/// Chosen by the same measurement that chose the ceiling: at 256 iterations
/// every body the card measured costs at most 24 ms of check time, which is
/// inside the cost of compiling the module around it; at 1024 the
/// acquire/release body costs 202 ms, roughly eight times the whole compile.
/// An unattributed loop above this is refused **by name, with the attribute
/// spelling in the message** — the refusal teaches the license rather than
/// merely denying the loop.
pub const DEFAULT_UNROLL_BUDGET: u32 = 256;

/// The largest `N` a `[| #unroll, where #bound < N |]` clause may name.
///
/// One more than [`UNROLL_CEILING`], because `#bound < N` licenses bounds up to
/// `N - 1`: the rule is about what a declaration can *license*, so the ceiling
/// binds the admitted bound rather than the spelled numeral. The full-extent
/// traversal is therefore expressible as `#bound < 4097`, and nothing above
/// 4096 is expressible at all.
pub const MAX_DECLARED_LIMIT: u32 = UNROLL_CEILING + 1;

/// The largest number of unrolled events a single entry may present.
///
/// This is the honest statement of what the checking route costs. Nesting is
/// admitted, and the bound that decides whether a nest is admitted is this
/// **product**, not a depth: two loops of eight are cheaper than one loop of a
/// hundred, and a depth limit would say otherwise.
pub const MAX_UNROLLED_EVENTS: usize = 8192;

/// The separator between a body-local owner's source name and the iteration
/// scope it belongs to, in the **unrolled** custody operation list.
///
/// A loop body's bindings are body-local, so the `c` one iteration acquires and
/// the `c` the next one acquires are two bindings, not one. The flat cycle-3
/// operation list has no scopes, so the scope is carried in the name: `c#3`.
///
/// This is a **faithful encoding of the scoping rule, not a relaxation of the
/// ownership model**, and the distinction is load-bearing. `check_ownership`
/// refuses re-binding a name it has seen dropped (`BindingAlreadyExists`), which
/// is a fail-closed gate that must not be weakened — and it should not fire
/// here, because the second iteration's `c` is a different binding in a
/// different scope, exactly as the emitted C's second `unsigned char *b_c` is a
/// different object. A body that acquires and does **not** release still leaks:
/// each iteration's distinct name is live at the exit and `NormalPathLeak` names
/// every one of them.
///
/// `#` is not a character any QCore identifier may contain, so a scoped name can
/// never collide with one a program wrote.
pub const ITERATION_SCOPE_SEPARATOR: char = '#';

/// The source name behind a scoped owner name, for a message a reader sees.
#[must_use]
pub fn display_owner(owner: &str) -> &str {
    owner
        .split_once(ITERATION_SCOPE_SEPARATOR)
        .map_or(owner, |(name, _)| name)
}

/// The bound decision at every §G02t type (card `W48-QR-SCALAR-TYPES-R5`):
/// the number of iterations a `for NAME in 0..BOUND` counts when `BOUND` is
/// the check-time number `value` of type `ty`, or the refusal.
///
/// The low end `0` is a literal, and a literal takes its context's type, so
/// the range has **one** type — the bound's — and so does the index. The
/// decision is the `LOOP-V1` one at every width, in the same order: a `bool`
/// is not an integer, an `isize`/`usize` needs the recorded target, then a
/// negative bound, a zero bound and a bound above [`MAX_ITERATIONS`] refuse.
/// What makes it sound at every width is that the admitted count is at most
/// the ceiling **and** a value of `ty` (it is the bound's own number): the
/// index takes `0 … count-1`, every one a value of `ty`, and the step from
/// the last of them reaches `count`, which is a value of `ty` too — so
/// `i + 1` never faults and the loop runs exactly `count` times
/// (`QCore.ScalarLoopBounds`). A `u64` near its maximum, or a signed bound
/// near `MIN`, is refused by the ceiling or the sign rule before any index
/// exists.
pub fn typed_bound(
    spelling: &str,
    ty: ScalarType,
    value: i128,
    target: Option<&RecordedTarget>,
) -> Result<u32, BoundRefusal> {
    if ty == ScalarType::Bool {
        return Err(BoundRefusal::NotAnInteger {
            spelling: spelling.to_owned(),
        });
    }
    // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: a float bound is refused by name,
    // before its number is read — a float's number here is its bit pattern,
    // which is no count, so reading it as one would admit a loop of whatever
    // count the pattern happened to spell (the float card's rule: a float
    // loop bound is refused on every route).
    if ty.is_float() {
        return Err(BoundRefusal::FloatBound {
            spelling: spelling.to_owned(),
            ty,
        });
    }
    if ty.is_target_width() && target.is_none() {
        return Err(BoundRefusal::NeedsRecordedTarget { ty });
    }
    if value < 0 {
        return Err(BoundRefusal::Negative { value });
    }
    if value == 0 {
        return Err(BoundRefusal::Zero);
    }
    if value > i128::from(MAX_ITERATIONS) {
        return Err(BoundRefusal::TooLarge { value });
    }
    let count = u32::try_from(value).expect("the bound is inside 1..=MAX_ITERATIONS");
    index_admits(count, ty, target)?;
    Ok(count)
}

/// Whether a loop counting `count` iterations may have an index of type `ty`
/// (card `W48-QR-SCALAR-TYPES-R5`): the index's values `0 … count-1` and the
/// final step's `count` must all be values of `ty`, which — the range being
/// an interval containing `0` — is exactly `count` being one. The checker
/// asks this again of every lowered loop, so a count the lowering produced
/// for an index type that cannot hold it is refused rather than iterated.
pub fn index_admits(
    count: u32,
    ty: ScalarType,
    target: Option<&RecordedTarget>,
) -> Result<(), BoundRefusal> {
    // Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: a float index holds no count
    // either (its `contains` is "is a bit pattern", not a range).
    if !ty.is_integer() {
        return Err(BoundRefusal::CountOutsideIndex { count, ty });
    }
    if ty.is_target_width() && target.is_none() {
        return Err(BoundRefusal::NeedsRecordedTarget { ty });
    }
    if ty.contains(i128::from(count), target) {
        Ok(())
    } else {
        Err(BoundRefusal::CountOutsideIndex { count, ty })
    }
}

/// The header of one `for NAME in 0..BOUND { … }` statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopHeader {
    pub variable: String,
    pub count: u32,
}

/// The unrolling licence one `[| #unroll, where #bound < N |]` clause list
/// declares (card `LOOP-UNROLL-BUDGET`).
///
/// The declaration is **checked, never trusted**: [`admit_unroll`] compares the
/// loop's actual check-time bound against `limit` and compares `limit` against
/// the ceiling, so a condition the loop violates and a condition that would
/// license more than the construct models are two different refusals and
/// neither is a promotion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnrollLicense {
    /// The `N` of `#bound < N`. The loop's bound must be strictly below it.
    pub limit: u32,
}

/// The three-layer budget decision, as one total function's result.
///
/// Two admitting answers and three refusing ones, and the difference between
/// the two admitting answers is *why* the loop is admitted rather than what it
/// costs — an admitted loop is unrolled identically either way, which is the
/// erasure the emission half rests on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetVerdict {
    /// No attribute, and the bound is inside [`DEFAULT_UNROLL_BUDGET`].
    Default,
    /// An attribute whose declared condition is inside the ceiling and which
    /// this loop's own bound satisfies.
    Licensed,
    /// No attribute, and the bound is above [`DEFAULT_UNROLL_BUDGET`].
    AboveDefault,
    /// The declared condition would license a bound above [`UNROLL_CEILING`].
    LicenseAboveCeiling,
    /// The declaration is present and this loop's bound does not satisfy it.
    LicenseViolated,
}

impl BudgetVerdict {
    /// Whether this verdict admits the loop.
    #[must_use]
    pub fn is_admitted(self) -> bool {
        matches!(self, Self::Default | Self::Licensed)
    }
}

/// The three-layer budget decision (card `LOOP-UNROLL-BUDGET`), total in both
/// arguments.
///
/// The order of the two licensed tests is load-bearing and is the reason the
/// ceiling survives a declaration: a `#bound < 99999` clause is refused for
/// naming more than the construct models **before** this loop's own bound is
/// looked at, so a tiny loop cannot carry a declaration that licenses a huge
/// one. Nothing here reads the body, the route or the cost — only the two
/// numbers and the two constants.
#[must_use]
pub fn admit_unroll(bound: u32, license: Option<UnrollLicense>) -> BudgetVerdict {
    match license {
        None => {
            if bound <= DEFAULT_UNROLL_BUDGET {
                BudgetVerdict::Default
            } else {
                BudgetVerdict::AboveDefault
            }
        }
        Some(UnrollLicense { limit }) => {
            if limit > MAX_DECLARED_LIMIT {
                BudgetVerdict::LicenseAboveCeiling
            } else if bound >= limit {
                BudgetVerdict::LicenseViolated
            } else {
                BudgetVerdict::Licensed
            }
        }
    }
}

/// What the reader knows about one loop's unrolling licence.
///
/// The third arm is the only one that is not a statement about the program: a
/// clause list this reader already refused licenses nothing, but holding the
/// loop to the default budget on top of that would print **two** refusals for
/// **one** mistake — card `LOOP-V1` §9.9's rule, applied to the attribute.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoopBudget {
    /// No attribute at all: the default budget governs.
    #[default]
    Default,
    /// A clause list that read: this loop is checked against it.
    Licensed(UnrollLicense),
    /// A clause list already refused at `Q097` or `Q098`. The budget decision
    /// is suppressed; every other rule still applies.
    Unreadable,
}

/// The budget decision presented as this construct's own `Q098` refusal, or
/// `None` when the loop is admitted (or when the clause list before it was
/// already refused).
#[must_use]
pub fn budget_refusal(bound: u32, budget: LoopBudget) -> Option<BoundRefusal> {
    let license = match budget {
        LoopBudget::Default => None,
        LoopBudget::Licensed(license) => Some(license),
        LoopBudget::Unreadable => return None,
    };
    match admit_unroll(bound, license) {
        BudgetVerdict::Default | BudgetVerdict::Licensed => None,
        BudgetVerdict::AboveDefault => Some(BoundRefusal::AboveDefaultBudget { bound }),
        BudgetVerdict::LicenseAboveCeiling => Some(BoundRefusal::LicenseAboveCeiling {
            limit: license.expect("a ceiling refusal carries a licence").limit,
        }),
        BudgetVerdict::LicenseViolated => Some(BoundRefusal::LicenseViolated {
            bound,
            limit: license
                .expect("a violation refusal carries a licence")
                .limit,
        }),
    }
}

/// Why a bound is not one this construct models.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundRefusal {
    /// The bound is not a check-time constant this entry bound.
    NotConstant { spelling: String },
    /// The bound is negative. Card `W48-QR-SCALAR-TYPES-R5`: the value is
    /// the bound's own number at its own type, so it is carried at the full
    /// width of the §G02t table (`i64::MIN` included).
    Negative { value: i128 },
    /// The bound is zero: the body would never run, so it would never be
    /// checked, and the construct would admit an unchecked statement.
    Zero,
    /// The bound is above [`MAX_ITERATIONS`] — at any width, `u64::MAX`
    /// included (card `W48-QR-SCALAR-TYPES-R5`).
    TooLarge { value: i128 },
    /// The entry's unrolled event total is above [`MAX_UNROLLED_EVENTS`].
    Budget { unrolled: usize },
    /// Card `LOOP-UNROLL-BUDGET`, layer two: the loop carries no unrolling
    /// attribute and its bound is above [`DEFAULT_UNROLL_BUDGET`]. The message
    /// names the attribute and spells the clause that would license this loop.
    AboveDefaultBudget { bound: u32 },
    /// Layer one, reached through a declaration: the declared condition would
    /// license a bound above [`UNROLL_CEILING`].
    LicenseAboveCeiling { limit: u32 },
    /// Layer three: the declaration is present and this loop's bound does not
    /// satisfy it. The declaration is checked, not trusted.
    LicenseViolated { bound: u32, limit: u32 },
    /// A declared condition no bound at all can satisfy — `#bound < 0` or
    /// `#bound < 1`, when the construct counts at least one iteration.
    ///
    /// Reported **without** looking at the loop, because it is a fact about the
    /// clause list alone: reporting it as a violated condition would have to
    /// name a bound, and the reader has not read one yet.
    LicenseLicensesNothing { limit: u32 },
    /// Card `W48-QR-SCALAR-TYPES-R5`: the bound is a `bool`, which is not an
    /// integer and counts nothing — there is no conversion to make it one.
    NotAnInteger { spelling: String },
    /// Card `W48-QR-FLOAT-BUFFER-PAYLOADS`: the bound is an `f32`/`f64`. A
    /// counted loop counts an integer; a float converts to one only with an
    /// explicit, checked `as`, and a float loop bound is refused on every route.
    FloatBound { spelling: String, ty: ScalarType },
    /// Card `W48-QR-SCALAR-TYPES-R5`: an `isize`/`usize` bound (and index)
    /// with no recorded target, whose width — and so whose range — nothing
    /// states. Refused, never guessed.
    NeedsRecordedTarget { ty: ScalarType },
    /// Card `W48-QR-SCALAR-TYPES-R5`: the count is not a value of the index
    /// type the loop declares — unreachable for a lowered program, whose count
    /// is the bound's own number, and refused by the checker rather than
    /// iterated on an index the type cannot hold.
    CountOutsideIndex { count: u32, ty: ScalarType },
    /// Card `W48-QR-SCALAR-TYPES-R5`: the checker's value of a typed bound is
    /// not the `ty` number `count` the lowering decided. Unreachable while the
    /// two fold with the one shared decision; refused rather than trusted.
    BoundDisagrees {
        bound: String,
        count: u32,
        ty: ScalarType,
    },
}

impl std::fmt::Display for BoundRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConstant { spelling } => write!(
                f,
                "`{spelling}` is not a check-time-constant bound; `source.loop.v1` counts a known \
                 number of iterations, because a route admitting this construct compares its \
                 reference execution against a native run byte for byte and a bound the checker \
                 cannot compute leaves that comparison with nothing to predict"
            ),
            Self::Negative { value } => write!(
                f,
                "{value} is not a bound `source.loop.v1` models; a counted loop runs 0..BOUND for \
                 a positive BOUND"
            ),
            Self::Zero => f.write_str(
                "a bound of 0 runs no iteration, so this body would never be checked; \
                 `source.loop.v1` refuses it rather than admitting an unchecked statement",
            ),
            Self::TooLarge { value } => write!(
                f,
                "{value} is above the {MAX_ITERATIONS} iterations `source.loop.v1` models; the \
                 bound is refused rather than truncated"
            ),
            Self::Budget { unrolled } => write!(
                f,
                "this entry unrolls to {unrolled} events, above the {MAX_UNROLLED_EVENTS} \
                 `source.loop.v1` checks; the construct is checked by running the body once per \
                 iteration, and that cost is bounded rather than paid"
            ),
            Self::AboveDefaultBudget { bound } => write!(
                f,
                "a `for` with no unrolling attribute is budgeted at {DEFAULT_UNROLL_BUDGET} \
                 iterations and this one counts {bound}; write \
                 `[| #unroll, where #bound < {} |]` immediately before the `for` to license it, \
                 up to the {UNROLL_CEILING}-iteration ceiling",
                u64::from(*bound) + 1
            ),
            Self::LicenseAboveCeiling { limit } => write!(
                f,
                "`#bound < {limit}` would license a bound of {}, above the {UNROLL_CEILING} \
                 iterations `source.loop.v1` models; the largest condition this construct admits \
                 is `#bound < {MAX_DECLARED_LIMIT}`, and the ceiling is a ceiling rather than a \
                 default a declaration can raise",
                u64::from(*limit) - 1
            ),
            Self::LicenseViolated { bound, limit } => write!(
                f,
                "`[| #unroll, where #bound < {limit} |]` licenses a bound below {limit} and this \
                 loop counts {bound}; the declared condition is checked against the bound the \
                 checker computes rather than taken on trust"
            ),
            Self::LicenseLicensesNothing { limit } => write!(
                f,
                "`#bound < {limit}` licenses no bound at all — `source.loop.v1` counts at least \
                 one iteration — so this clause list refuses the loop it was written to permit; \
                 the smallest condition that licenses anything is `#bound < 2`"
            ),
            Self::FloatBound { spelling, ty } => write!(
                f,
                "`{spelling}` is an `{ty}`, and a counted loop counts an integer; a float loop \
                 bound is refused on every route — convert it with `as` to an integer type first, \
                 which truncates and is refused when it does not fit"
            ),
            Self::NotAnInteger { spelling } => write!(
                f,
                "`{spelling}` is a `bool`, and a counted loop counts an integer; `bool` is not an \
                 integer and nothing converts it to one"
            ),
            Self::NeedsRecordedTarget { ty } => write!(
                f,
                "this bound is a `{ty}`, which takes its width from a recorded target, none of \
                 which was supplied; the index would be a `{ty}` too, and its width is never \
                 guessed (supply one with `--recorded-target FILE`)"
            ),
            Self::BoundDisagrees { bound, count, ty } => write!(
                f,
                "the loop was lowered to count {count} iterations from the `{ty}` bound \
                 `{bound}`, and the checker's value of `{bound}` is not that number of that \
                 type; the count is re-derived rather than trusted, so the loop is refused"
            ),
            Self::CountOutsideIndex { count, ty } => write!(
                f,
                "a loop counting {count} iterations cannot have a `{ty}` index; its last value \
                 {} and its final step {count} must both be values of the index type",
                i64::from(*count) - 1
            ),
        }
    }
}

/// The refusals this construct owns, with their `Q097`–`Q099` codes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoopRefusal {
    /// `Q097`: a shape `source.loop.v1` does not model.
    Shape(String),
    /// `Q098`: the bound.
    Bound(BoundRefusal),
    /// `Q099`: one iteration of the body is not the identity on custody.
    NotIdentity { variable: String, detail: String },
}

impl LoopRefusal {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Shape(_) => "Q097",
            Self::Bound(_) => "Q098",
            Self::NotIdentity { .. } => "Q099",
        }
    }
}

impl std::fmt::Display for LoopRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shape(reason) => f.write_str(reason),
            Self::Bound(refusal) => refusal.fmt(f),
            Self::NotIdentity { variable, detail } => write!(
                f,
                "one iteration of `for {variable} in …` does not leave the custody state it \
                 found: {detail}. `source.loop.v1` runs a body a known number of times, so a \
                 body whose net custody effect is not the identity would owe a different \
                 obligation on every iteration"
            ),
        }
    }
}

impl std::error::Error for LoopRefusal {}

/// The custody facts one iteration must not move.
///
/// Read entirely through [`OwnershipState`]'s public surface — the live
/// footprint and the ledger — so this is a statement about the cycle-3 ownership
/// model rather than a second copy of it. The resource **identity** is
/// deliberately not part of the signature: a body that releases a buffer and
/// acquires a fresh one of the same extent under the same name *is* the identity
/// on custody, and calling it a change would refuse the shape the discipline
/// exists to admit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustodySignature {
    /// Owner name → (resource type, footprint) for every live resource.
    pub owned: BTreeMap<String, (String, Footprint)>,
    /// The aggregate live footprint, read from the state's own `live()`.
    pub live: Footprint,
}

#[must_use]
pub fn custody_signature(state: &OwnershipState) -> CustodySignature {
    let mut owned = BTreeMap::new();
    for entry in state.ledger() {
        owned.insert(
            entry.owner_spelling(),
            (entry.resource_type().to_owned(), entry.footprint().clone()),
        );
    }
    CustodySignature {
        owned,
        live: state.live().clone(),
    }
}

/// Human-readable difference between two signatures, for the refusal message.
#[must_use]
pub fn signature_difference(before: &CustodySignature, after: &CustodySignature) -> String {
    let mut parts = Vec::new();
    for (name, (kind, _)) in &after.owned {
        if !before.owned.contains_key(name) {
            let name = display_owner(name);
            parts.push(format!(
                "it acquires `{name}` ({kind}) and does not release it"
            ));
        }
    }
    for (name, (kind, _)) in &before.owned {
        if !after.owned.contains_key(name) {
            let name = display_owner(name);
            parts.push(format!(
                "it releases `{name}` ({kind}) and does not replace it"
            ));
        }
    }
    for (name, (kind, footprint)) in &after.owned {
        if let Some((was_kind, was_footprint)) = before.owned.get(name)
            && (was_kind != kind || was_footprint != footprint)
        {
            parts.push(format!(
                "`{name}` is held as {kind} {footprint:?} where it was {was_kind} {was_footprint:?}"
            ));
        }
    }
    if parts.is_empty() {
        format!(
            "the live footprint moves from {:?} to {:?}",
            before.live, after.live
        )
    } else {
        parts.join("; ")
    }
}

/// Whether running `body` once, from the state `prefix` reaches, leaves the
/// custody signature unmoved.
///
/// An `Err` means the ownership model refused the operations outright, which is
/// a refusal the *inherited* checker reports at its own code; this gate says
/// nothing about it and the caller must not silence it.
pub fn iteration_is_custody_identity(
    prefix: &[OwnershipOp],
    body: &[OwnershipOp],
) -> Result<Result<(), String>, OwnershipError> {
    let before = custody_signature(&execute_ownership(prefix)?);
    let mut once = prefix.to_vec();
    once.extend_from_slice(body);
    let after = custody_signature(&execute_ownership(&once)?);
    if before == after {
        Ok(Ok(()))
    } else {
        Ok(Err(signature_difference(&before, &after)))
    }
}

/// The prefix lengths at which the live footprint can be at its maximum.
///
/// The live footprint of the cycle-3 ownership model grows at exactly one
/// operation — an [`OwnershipOp::Acquire`] — so the maximum over every prefix is
/// attained at length zero or immediately after an acquisition. Scanning only
/// those points gives the **same number** as scanning every prefix.
///
/// **Retained as the statement of that equality, not as the route that runs.**
/// Card `LOOP-V1` used this list to drive the peak, re-executing each candidate
/// prefix from the start — linear in the number of acquisitions, but quadratic
/// in ownership steps, which a body acquiring once per iteration pays in full
/// (56.4 s at a bound of 1024). Card `LOOP-UNROLL-BUDGET` moved the computation
/// to [`crate::ownership::peak_live_axis`], one pass over the same operations
/// yielding the same number. This function still names the prefixes that can be
/// the maximum, and its test still compares them against a full scan, so the
/// claim both routes rest on stays checked.
#[must_use]
pub fn peak_candidate_prefixes(operations: &[OwnershipOp]) -> Vec<usize> {
    let mut lengths = vec![0usize];
    for (index, operation) in operations.iter().enumerate() {
        if matches!(operation, OwnershipOp::Acquire { .. }) {
            lengths.push(index + 1);
        }
    }
    lengths
}
