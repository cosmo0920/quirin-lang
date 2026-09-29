//! Card **CG04-CONDITION-ACQUISITION** (gateway rule 10x): ownership-producing
//! calls inside controlling expressions, and the composition of complex
//! condition leaves (Clang gap entries E09 and E13).
//!
//! # Acquisition in a condition (E09)
//!
//! Example A of the gap analysis:
//!
//! ```c
//! void *p;
//! if ((p = malloc(n)) != NULL) { free(p); }
//! ```
//!
//! The condition desugaring (rule 5h) lowers a leaf's calls where C evaluates
//! them and asks the one application point (rule 10u) for each. A leaf's call
//! value is *tested*, so an owned, reallocated or descriptor result was
//! refused as `custody-result`. This card widens exactly that row: a
//! `(NAME = CALL(…))` group whose leaf tests `NAME` against the contract's
//! failure value is the use [`ResultUse::TestedSplit`]. The leaf then lowers
//! to the instructions of the statement pair `NAME = CALL(…); if (NAME …)`,
//! in the condition's own node, so the five parts are lowered together by the
//! one existing flow:
//!
//! 1. the fresh identity from the contract (the classifier's `ContractCall`,
//!    its produced site);
//! 2. its binding to the destination `NAME` (the call's destination);
//! 3. the split: the leaf's branch is the null test of `NAME`
//!    (`BranchOnNull`), or for a descriptor its validity test against the
//!    sentinel (`BranchOnValidity`);
//! 4. success and failure states on each edge: the flow's refinement keeps
//!    the obligation on the success edge only, and a reallocation's failure
//!    edge keeps the old identity live (its contract's failure arm);
//! 5. the merge: the plan's edges join the successors exactly as the
//!    undesugared statement pair's do.
//!
//! The call is **never hoisted**: it stays a node of the condition's plan, so
//! a loop header re-evaluates it on every visit, and the right operand of
//! `&&` (or `||`) acquires only on the edges C evaluates it on
//! (`QCore.ConditionCallDesugar`).
//!
//! [`leaf_use`] decides the use, [`carrier`] checks that the split tests the
//! carrier the result is (a pointer's NULL, an integer descriptor's
//! sentinel). An out-parameter acquisition (`if (f(&p) == 0)`) needs a
//! contract that states the result/out relation; no contract does, so its
//! refusal is unchanged (CG23).
//!
//! # Complex leaves (E13)
//!
//! The desugaring refused a whole condition holding a type operator or an
//! increment anywhere, before it split the leaves. [`leaf_route`] decides
//! these per leaf instead:
//!
//! * a type operator (`sizeof`, `_Alignof`, `typeof`) whose operand holds no
//!   call is residual text: its operand is not evaluated (C11 6.5.3.4p2) or
//!   holds nothing that acts, and the existing residual routes decide it; an
//!   operand holding a call refuses, since that call is not evaluated and
//!   must never be applied;
//! * an increment or a nested connective in a leaf holding a call refuses
//!   (a call unordered with a write is CG05's, a value connective CG06's);
//! * a call-free leaf holding an increment, assignment or comma is asked of
//!   the existing call-free routes and then of the elaborator (rule 10s),
//!   whose events precede the leaf's branch that refines nothing.
//!
//! The Lean model is `QCore.ConditionAcquisition`; [`USE_VECTORS`],
//! [`CARRIER_VECTORS`] and [`LEAF_VECTORS`] are shared with it.

use crate::call_instantiation::{ResultClass, ResultUse};

/// How a leaf tests the value its one assigned call stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Split {
    /// A null test of a pointer destination.
    Null,
    /// A validity test of a descriptor destination against its sentinel.
    Sentinel,
}

impl Split {
    #[cfg(test)]
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Sentinel => "sentinel",
        }
    }
}

/// The use of the value of a leaf's call: the NULL split when the call is
/// bound to a destination the leaf splits on, the tested use otherwise.
pub(crate) fn leaf_use(bound: bool, split: Option<Split>) -> ResultUse {
    match (bound, split) {
        (true, Some(_)) => ResultUse::TestedSplit,
        _ => ResultUse::Tested,
    }
}

/// Why this card keeps a condition refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// A custody result split on another carrier's failure value.
    CarrierMismatch,
    /// A call inside a type operator's operand: C does not evaluate it.
    CallInTypeOperand,
    /// An increment or decrement in a leaf holding a call (CG05).
    IncrementBesideCall,
    /// A connective below a leaf holding a call (CG06).
    ConnectiveBesideCall,
}

impl Refusal {
    #[cfg_attr(not(any(test, feature = "indirect-census")), allow(dead_code))]
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::CarrierMismatch => "carrier-mismatch",
            Self::CallInTypeOperand => "call-in-type-operand",
            Self::IncrementBesideCall => "increment-beside-call",
            Self::ConnectiveBesideCall => "connective-beside-call",
        }
    }
}

/// A custody result is split only on its own carrier's failure value: a
/// pointer's NULL, an integer-carried descriptor's sentinel. A resource-free
/// result tracks nothing, so its split refines nothing it owns.
pub(crate) fn carrier(result: ResultClass, integer: bool, split: Split) -> Result<(), Refusal> {
    if result.resource_free() {
        return Ok(());
    }
    match (integer, split) {
        (false, Split::Null) | (true, Split::Sentinel) => Ok(()),
        _ => Err(Refusal::CarrierMismatch),
    }
}

/// The lexical features of one condition leaf this card reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LeafFeatures {
    /// A direct call outside every type operator's operand.
    pub(crate) call: bool,
    /// A call inside a type operator's operand.
    pub(crate) call_in_type_operand: bool,
    /// `++` or `--` anywhere in the leaf.
    pub(crate) increment: bool,
    /// `&&`, `||` or `?` below the leaf's top level, outside a call's actuals.
    pub(crate) connective: bool,
}

/// Which lowering decides a leaf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LeafRoute {
    /// The desugaring's calls in evaluation order, then the residual branch.
    Calls,
    /// The call-free routes, then the elaborator (rule 10s).
    CallFree,
}

impl LeafRoute {
    #[cfg(test)]
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Calls => "calls",
            Self::CallFree => "call-free",
        }
    }
}

/// The per-leaf decision replacing the whole-condition refusals of a type
/// operator and an increment.
pub(crate) fn leaf_route(features: LeafFeatures) -> Result<LeafRoute, Refusal> {
    if features.call_in_type_operand {
        return Err(Refusal::CallInTypeOperand);
    }
    if !features.call {
        return Ok(LeafRoute::CallFree);
    }
    if features.increment {
        return Err(Refusal::IncrementBesideCall);
    }
    if features.connective {
        return Err(Refusal::ConnectiveBesideCall);
    }
    Ok(LeafRoute::Calls)
}

/// `(bound, split, use)`, shared with `proof/QCore/ConditionAcquisition.lean`
/// (`useVectors`).
#[cfg(test)]
pub(crate) const USE_VECTORS: &[(bool, &str, &str)] = &[
    (true, "null", "tested-split"),
    (true, "sentinel", "tested-split"),
    (true, "none", "tested"),
    (false, "null", "tested"),
    (false, "none", "tested"),
];

/// `(result, integer carrier, split, decision)`, shared with
/// `proof/QCore/ConditionAcquisition.lean` (`carrierVectors`).
#[cfg(test)]
pub(crate) const CARRIER_VECTORS: &[(&str, bool, &str, &str)] = &[
    ("owned", false, "null", "accept"),
    ("owned", true, "sentinel", "accept"),
    ("reallocated", false, "null", "accept"),
    ("owned", true, "null", "carrier-mismatch"),
    ("owned", false, "sentinel", "carrier-mismatch"),
    ("reallocated", true, "null", "carrier-mismatch"),
    ("none", false, "null", "accept"),
    ("derived", false, "null", "accept"),
];

/// `(call, call in type operand, increment, connective, decision)`, shared
/// with `proof/QCore/ConditionAcquisition.lean` (`leafVectors`).
#[cfg(test)]
pub(crate) const LEAF_VECTORS: &[(bool, bool, bool, bool, &str)] = &[
    (true, false, false, false, "calls"),
    (false, false, false, false, "call-free"),
    (false, false, true, false, "call-free"),
    (false, false, false, true, "call-free"),
    (true, false, true, false, "increment-beside-call"),
    (true, false, false, true, "connective-beside-call"),
    (true, true, false, false, "call-in-type-operand"),
    (false, true, false, false, "call-in-type-operand"),
    (true, false, true, true, "increment-beside-call"),
];

#[cfg(test)]
#[path = "../tests/unit/condition_acquisition.rs"]
mod tests;
