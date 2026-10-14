//! Retained pointers (card **CALLEE-RETENTION-CLASS1**, stage (b); contract
//! spec §C04.14, grammar G05, gateway rule 12t).
//!
//! USER DECISION 2026-10-05 (`W48-CALLEE-DECISION-TABLE` class 1, "split by
//! shape"): a callee that keeps a pointer the caller passed — LZ4's stream
//! states keep pointers into `src`, a raw chunk keeps its input instance —
//! states it with
//!
//! ```text
//! ensures normal resource_retained_by(p, h);      // h a parameter, or `global`
//! ensures normal resource_holder_reset(h);        // h forgets what it held
//! ```
//!
//! The caller **keeps custody** of `p`'s object (its mode is
//! `resource_borrowed`), but the object must outlive the holder `h`, or the
//! holder must be reset or destroyed before the object is released. A
//! consumer that cannot establish that refuses the call: the result is
//! `Unknown` (or `Pending` with a named human-resolvable reason), never
//! `Safe`, and releasing `p` while a live holder retains it is a hazard the
//! consumer refuses, not a defect it reports.
//!
//! A declaration carrying either clause is **never** an entry of
//! [`crate::contract::ResourceContracts::functions`] (the §C06.1 precedent):
//! a consumer that knows nothing of retention sees an uncontracted callee, so
//! a retained pointer is never read as borrowed and forgotten.

use serde::{Deserialize, Serialize};

use crate::contract::ResourceFunctionContract;

/// What retains the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "holder")]
pub enum RetentionHolder {
    /// The object the parameter at this index designates.
    Parameter { index: usize },
    /// Storage of static duration, or a holder no caller object denotes (a
    /// file-scope object, a queue another thread drains): it lives for the
    /// program as far as the caller can tell.
    Global,
    /// Card **REFCOUNT-REFERENCE-RETURNING-DECLARATIONS** (§C04.14.2, gateway
    /// rule 13e): the object the call returns (`resource_retained_by(p,
    /// result)`), which the declaration's `resource_return_owned(nonnull)`
    /// hands its caller. The caller must show `p`'s object outlives the
    /// handle: the handle is released, or its last reference dropped, first.
    Result,
}

/// `resource_retained_by(p, h)`: the parameter at `retained` is kept by `holder`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Retention {
    pub retained: usize,
    pub holder: RetentionHolder,
    /// Card **W43-ABI-ENUM-AND-CONST-PP** (contract spec §C04.14.1, gateway
    /// rule 12w): `… when X OP K`. `None` (the only form before the card, and
    /// the serialized form it keeps) holds at every call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<RetentionGuard>,
}

/// The comparison of a guard (the QUANT-INDEX-CHECKS operator set, §C09.2,
/// evaluated over the mathematical integers).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardRelation {
    Eq,
    Lt,
    Gt,
    Le,
    Ge,
}

impl GuardRelation {
    /// The relation's spelling.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Eq => "==",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
        }
    }

    /// The relation read from its spelling.
    #[must_use]
    pub fn from_spelling(spelling: &str) -> Option<Self> {
        Some(match spelling {
            "==" => Self::Eq,
            "<" => Self::Lt,
            ">" => Self::Gt,
            "<=" => Self::Le,
            ">=" => Self::Ge,
            _ => return None,
        })
    }

    /// `value OP constant` over the mathematical integers.
    #[must_use]
    pub const fn holds(self, value: i128, constant: i128) -> bool {
        match self {
            Self::Eq => value == constant,
            Self::Lt => value < constant,
            Self::Gt => value > constant,
            Self::Le => value <= constant,
            Self::Ge => value >= constant,
        }
    }
}

/// `when X OP K`: the clause holds exactly at the calls whose actual at the
/// integer parameter `parameter` is decided to satisfy `X OP K`; an actual
/// decided not to satisfy it drops the clause, and an undecided one keeps it
/// (the cautious reading, the union of every guarded clause).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RetentionGuard {
    pub parameter: usize,
    pub relation: GuardRelation,
    pub constant: i128,
    /// The value range of the parameter's declared integer spelling. A call
    /// site's value outside it is converted by C before the callee sees it,
    /// so it is not decided.
    pub low: i128,
    pub high: i128,
}

/// What a call site's actual decides about a guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardDecision {
    /// The actual is a value in range that satisfies the guard: the clause
    /// is in force.
    Holds,
    /// The actual is a value in range that does not: the clause is not.
    Fails,
    /// Anything else: the cautious reading keeps the clause in force.
    Undecided,
}

impl RetentionGuard {
    /// Whether the guard holds of a value.
    #[must_use]
    pub const fn holds(self, value: i128) -> bool {
        self.relation.holds(value, self.constant)
    }

    /// The decision for a call site whose actual is `value` (`None`: not
    /// decided).
    #[must_use]
    pub const fn decide(self, value: Option<i128>) -> GuardDecision {
        match value {
            Some(value) if self.low <= value && value <= self.high => {
                if self.holds(value) {
                    GuardDecision::Holds
                } else {
                    GuardDecision::Fails
                }
            }
            _ => GuardDecision::Undecided,
        }
    }

    /// Whether the clause stays in force under the decision: everywhere but
    /// a decided failure.
    #[must_use]
    pub const fn in_force(decision: GuardDecision) -> bool {
        !matches!(decision, GuardDecision::Fails)
    }
}

/// A declaration with retention clauses: its custody contract (every retained
/// pointer and every reset holder `resource_borrowed`), its retentions and
/// the parameters whose holders it resets. Card SLOT-MOVE-AND-INTERIOR-BORROW:
/// also a declaration with slot-move or derived-out-slot clauses, withheld
/// from the function map the same way.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RetainingContract {
    pub contract: ResourceFunctionContract,
    /// In clause order; at most one per retained parameter.
    pub retentions: Vec<Retention>,
    /// `resource_holder_reset(h)`, in clause order; at most one per parameter.
    pub resets: Vec<usize>,
    /// Card **SLOT-MOVE-AND-INTERIOR-BORROW** (§C04.15, rule 12x):
    /// `resource_slot_moved(s, n)`, in clause order; at most one per slot.
    /// Omitted when empty, so every retaining declaration before the card
    /// serializes as it did.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slot_moves: Vec<crate::slot_move::SlotMove>,
    /// Card **SLOT-MOVE-AND-INTERIOR-BORROW** (§C04.16, rule 12z):
    /// `resource_out_derived(s, p, n)`, in clause order; at most one per slot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derived_outs: Vec<crate::slot_move::DerivedOut>,
    /// Card **REFCOUNT-RESOURCE-KIND** (§C04.17, rule 13b):
    /// `resource_reference_kept(p)` / `resource_reference_dropped(p)`, in
    /// clause order; at most one per declaration. Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<crate::reference_count::ReferenceClause>,
    /// Card **REFCOUNT-RESOURCE-KIND** (rule 13c): set only in a translation
    /// unit whose own body of the declaration does not validate its reference
    /// clause. Every call there refuses (`reference-withdrawn`); the
    /// declaration is not dropped from the table, so no inference reads the
    /// body in its place (the neutral reading is never admitted). Never set
    /// by the loader; omitted when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub references_withdrawn: bool,
    /// Card **CG22-CALLBACK-REGISTRY-CONTRACT** (§C04.19, rules 13u–13x):
    /// the callback clauses, in clause order. Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub callbacks: Vec<crate::callback::CallbackClause>,
    /// Card **CG22-CALLBACK-REGISTRY-CONTRACT** (rule 13y): set only in a
    /// translation unit whose own body of the declaration does not validate
    /// a checked callback clause. Every call there refuses
    /// (`callback-withdrawn`); the declaration is not dropped, so no
    /// inference reads the body in its place. Never set by the loader;
    /// omitted when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub callbacks_withdrawn: bool,
    /// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (§C04.19.2, rule 12o):
    /// `requires resource_discriminant(P, FIELD, MASK)`. Omitted when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discriminant: Option<crate::arm_release::Discriminant>,
    /// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (rule 12o): the arm
    /// clauses, in clause order. Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arms: Vec<crate::arm_release::ArmClause>,
    /// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (rule 12o): set only in a
    /// translation unit whose own body of the declaration does not validate
    /// its arms. Every call there refuses (`arm-withdrawn`); never set by the
    /// loader; omitted when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub arms_withdrawn: bool,
    /// Card **OP-CONSTRUCTOR-DECLARATIONS** (§C04.19.4, rules 12o, 13w,
    /// 14e): the result state a constructor states. Omitted when empty.
    #[serde(default, skip_serializing_if = "crate::constructed_state::ConstructedState::is_empty")]
    pub constructed: crate::constructed_state::ConstructedState,
    /// Card **OP-CONSTRUCTOR-DECLARATIONS** (rule 14e): set only in a
    /// translation unit whose own body of the declaration does not validate
    /// its stated result state. The state is then read nowhere in that unit
    /// (the result is tracked, its member and discriminant undecided); never
    /// set by the loader; omitted when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub constructed_withdrawn: bool,
}

impl RetainingContract {
    /// Whether the declaration retains a pointer or resets a holder (rule
    /// 12t's own clauses), as opposed to carrying only slot clauses.
    #[must_use]
    pub fn retains(&self) -> bool {
        !self.retentions.is_empty() || !self.resets.is_empty()
    }
}
