//! Card **MK-EVENT-DESCRIPTOR-QUANT** (contract spec §C04.6.2, grammar G05d,
//! gateway rule 38): descriptor cells.
//!
//! USER DECISION 2026-11-01 (verbatim in the spec and the card's report): "I
//! approve the changes of D1-D5." A **cell** is a place that holds a
//! descriptor number: a member of the object a pointer formal designates
//! (`#data: mk_event.fd`) or the `int` a slot formal points to
//! (`#r_fd: #pointee`). A descriptor is an identity and a cell is a name of
//! it; a call's integer result guarding a cell state is its **witness**.
//!
//! ```text
//! quant live #data: mk_event.fd |-> #fresh(descriptor) when #result != -1;
//! quant live #data: mk_event.fd |-> {#stale(descriptor), #kept} when #result == -1;
//! quant live #data: mk_event.fd |-> #stale(descriptor);
//! quant live #r_fd: #pointee |-> #fresh(descriptor) when #result == 0;
//! quant live #data: mk_event.fd |-> #same(r_fd) when #result == 0;
//! ensures normal resource_released(w_fd, valid) when #data: mk_event.fd == #r_fd;
//! quant live #loop: #holds(data) |-> #absent when #data: mk_event.fd == #r_fd;
//! ```
//!
//! Every spelling, the shorthand and its lower level, loads to the one
//! canonical set below ([`DescriptorCells`]); the Lean model is
//! `QCore.DescriptorCells`.

use serde::{Deserialize, Serialize};

use crate::callback::CallbackMember;
use crate::contract::ResourceKind;

/// `#fresh`.
pub const FRESH: &str = "fresh";
/// `#same`.
pub const SAME: &str = "same";
/// `#pointee`.
pub const POINTEE: &str = "pointee";

/// The rule premise every declaration carrying a cell state publishes with
/// the rows resting on it (decision D2).
pub const NOT_FORWARDED_PREMISE: &str = "P-CELL-STATE-NOT-FORWARDED";

/// Its statement.
pub const NOT_FORWARDED_STATEMENT: &str = "a state a call puts on a descriptor cell of an object this function does not track (a caller's object) is not forwarded into its summary (gateway rule 38)";

/// A place that holds a descriptor number.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "cell")]
pub enum CellPlace {
    /// `#P: TAG.FIELD`: the member of the object the formal designates.
    Member {
        parameter: usize,
        member: CallbackMember,
    },
    /// `#S: #pointee`: the `int` the slot formal points to.
    Slot { parameter: usize },
}

impl CellPlace {
    /// The formal the place is read through.
    #[must_use]
    pub const fn parameter(&self) -> usize {
        match self {
            Self::Member { parameter, .. } | Self::Slot { parameter } => *parameter,
        }
    }
}

/// The state a call leaves at a cell, its kind stated apart
/// ([`DescriptorCells::kinds`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "value")]
pub enum CellValue {
    /// `#fresh(K)`: the call created one identity, named by this cell.
    Fresh,
    /// `#stale(K)`: the identity the cell names is released; the cell still
    /// holds its number.
    Stale,
    /// `{#stale(K), #kept}`: the cell names no live identity; it is
    /// untouched or holds a number the call closed.
    StaleOrKept,
    /// `#same(S)`: the cell is a second name of the identity the slot formal
    /// `S`'s pointee receives.
    Same { slot: usize },
}

/// A witness guard: the class of the call's result a state holds in. The
/// admitted pairs are `!= -1` / `== -1` (a descriptor-valued result, read by
/// validity) and `== 0` / `!= 0` (a status, read by a null test).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WitnessGuard {
    /// `#result != -1`.
    NotSentinel,
    /// `#result == -1`.
    Sentinel,
    /// `#result == 0`.
    Zero,
    /// `#result != 0`.
    NonZero,
}

impl WitnessGuard {
    /// The guard of `#result OP VALUE`, `None` outside the admitted pairs.
    #[must_use]
    pub const fn from_comparison(equal: bool, value: i64) -> Option<Self> {
        match (equal, value) {
            (false, -1) => Some(Self::NotSentinel),
            (true, -1) => Some(Self::Sentinel),
            (true, 0) => Some(Self::Zero),
            (false, 0) => Some(Self::NonZero),
            _ => None,
        }
    }

    /// The other class.
    #[must_use]
    pub const fn complement(self) -> Self {
        match self {
            Self::NotSentinel => Self::Sentinel,
            Self::Sentinel => Self::NotSentinel,
            Self::Zero => Self::NonZero,
            Self::NonZero => Self::Zero,
        }
    }

    /// The spelling after `#result`.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::NotSentinel => "!= -1",
            Self::Sentinel => "== -1",
            Self::Zero => "== 0",
            Self::NonZero => "!= 0",
        }
    }

    /// The lower level's `resource_on_return` operator and value.
    #[must_use]
    pub const fn relation(self) -> (&'static str, i64) {
        match self {
            Self::NotSentinel => ("ne", -1),
            Self::Sentinel => ("eq", -1),
            Self::Zero => ("eq", 0),
            Self::NonZero => ("ne", 0),
        }
    }

    /// Whether the guard is read by validity (`-1`) rather than by a null
    /// test (`0`).
    #[must_use]
    pub const fn is_validity(self) -> bool {
        matches!(self, Self::NotSentinel | Self::Sentinel)
    }
}

/// One cell state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CellState {
    pub place: CellPlace,
    pub value: CellValue,
    /// `None`: every return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<WitnessGuard>,
}

/// `when #P: TAG.FIELD == #X`: the member of the object `P` designates holds
/// the number the integer formal `X` carries.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MemberEquality {
    pub parameter: usize,
    pub member: CallbackMember,
    pub against: usize,
}

/// A declaration's descriptor cells, canonical: every spelling of each state
/// elaborated into its parts.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DescriptorCells {
    /// Each cell's kind, once (`#fresh(K)` / `#stale(K)` carry it at the
    /// shorthand level; `resource_member_kind` / `resource_kind` at the lower
    /// one).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<(CellPlace, ResourceKind)>,
    /// In canonical order (sorted).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<CellState>,
    /// The descriptor formals released under [`Self::guard`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guarded_releases: Vec<usize>,
    /// Whether the declaration's removals hold only under [`Self::guard`].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub guarded_removal: bool,
    /// The one member-equality guard of the declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<MemberEquality>,
}

impl DescriptorCells {
    /// Whether the declaration states nothing here.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
            && self.states.is_empty()
            && self.guarded_releases.is_empty()
            && !self.guarded_removal
            && self.guard.is_none()
    }

    /// The kind of `place`.
    #[must_use]
    pub fn kind(&self, place: &CellPlace) -> Option<ResourceKind> {
        self.kinds
            .iter()
            .find(|(stated, _)| stated == place)
            .map(|(_, kind)| *kind)
    }

    /// Whether any state is guarded by the call's result.
    #[must_use]
    pub fn witnessed(&self) -> bool {
        self.states.iter().any(|state| state.guard.is_some())
    }
}
