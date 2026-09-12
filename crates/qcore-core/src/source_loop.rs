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
//! * arithmetic on the index (`LOOP-INDEX-ARITHMETIC`) — the index is a scalar
//!   binding and this revision has no expression tier.

use std::collections::BTreeMap;

use crate::ownership::{Footprint, OwnershipError, OwnershipOp, OwnershipState, execute_ownership};

/// The largest iteration count this construct admits.
///
/// Equal to `std_buffer::MAX_EXTENT` on purpose: a traversal of the largest
/// buffer the byte-buffer route admits is expressible, and nothing larger is.
/// A greater bound is refused by name rather than truncated, the same
/// discipline every other quantity on these routes follows.
pub const MAX_ITERATIONS: u32 = 4096;

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

/// The header of one `for NAME in 0..BOUND { … }` statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopHeader {
    pub variable: String,
    pub count: u32,
}

/// Why a bound is not one this construct models.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundRefusal {
    /// The bound is not a check-time constant this entry bound.
    NotConstant { spelling: String },
    /// The bound is negative.
    Negative { value: i32 },
    /// The bound is zero: the body would never run, so it would never be
    /// checked, and the construct would admit an unchecked statement.
    Zero,
    /// The bound is above [`MAX_ITERATIONS`].
    TooLarge { value: i64 },
    /// The entry's unrolled event total is above [`MAX_UNROLLED_EVENTS`].
    Budget { unrolled: usize },
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
            entry.owner().to_owned(),
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
/// those points gives the **same number** as scanning every prefix, and it is
/// what keeps a peak computed from an unrolled body linear in the number of
/// acquisitions rather than quadratic in the number of events.
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
