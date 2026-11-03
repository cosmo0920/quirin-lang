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

/// Card **MK-EVENT-CELL-CROSS-FUNCTION** (contract spec §C04.6.3, grammar
/// G05i, gateway rule 38c; USER DECISION 2026-11-02, verbatim: "B as
/// recommended"): the guard of a record-type cell protocol, `#B.GUARD OP V`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolGuard {
    /// `!= -1`: a sentinel cell; a live descriptor is never `-1`.
    NotMinusOne,
    /// `> 0`: a sentinel cell; a creation never establishes it (descriptor
    /// `0` exists). Card CELL-PROTOCOL-PER-END-GUARDS (gateway rule 38f): its
    /// dead edge (`<= 0`) holds descriptor 0, a success value of every
    /// creator and calloc's absent value at once, so it is never read as
    /// "no cell is live".
    Positive,
    /// `== 1`: a flag member that is no cell.
    EqualOne,
}

impl ProtocolGuard {
    /// The guard of `OP V`, `None` outside the three admitted pairs.
    #[must_use]
    pub fn from_spelling(op: &str, value: i64) -> Option<Self> {
        match (op, value) {
            ("!=", -1) => Some(Self::NotMinusOne),
            (">", 0) => Some(Self::Positive),
            ("==", 1) => Some(Self::EqualOne),
            _ => None,
        }
    }

    /// The spelling after the guard path.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::NotMinusOne => "!= -1",
            Self::Positive => "> 0",
            Self::EqualOne => "== 1",
        }
    }

    /// Whether the guard is one of the protocol's cells (a sentinel) rather
    /// than a flag.
    #[must_use]
    pub const fn is_sentinel(self) -> bool {
        !matches!(self, Self::EqualOne)
    }

    /// Whether a stored integer `value` makes the guard say live (`true`) or
    /// dead (`false`).
    #[must_use]
    pub const fn says_live(self, value: i64) -> bool {
        match self {
            Self::NotMinusOne => value != -1,
            Self::Positive => value > 0,
            Self::EqualOne => value == 1,
        }
    }
}

/// One cell of a protocol: a member path of the record (segments such as
/// `ch_events[0]`, `event`, `fd`), stated `#fresh(descriptor)` or, with
/// `same`, `#same(CELL2)`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProtocolCell {
    pub path: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same: Option<Vec<String>>,
}

/// A record type's declared cell protocol: whenever the guard holds, each
/// `#fresh` cell names its own live descriptor and each `#same` cell its
/// target's; otherwise no cell names a live one. A declared assumption
/// (rule 13i), read by gateway rules 38c and 38f only.
///
/// Card CELL-PROTOCOL-PER-END-GUARDS (USER DECISION 2026-11-03, verbatim:
/// "Per-end sentinels"): one value per guard. A record states at most one
/// record guard (a flag, or a sentinel over several `#fresh` cells) and any
/// number of per-end sentinels ([`Self::per_end`]), so a record may carry
/// several of these, one per guard.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CellProtocol {
    pub record: String,
    pub cells: Vec<ProtocolCell>,
    pub guard: Vec<String>,
    pub op: ProtocolGuard,
}

impl CellProtocol {
    /// The `#fresh` cell whose identity `path` names, `None` for a path that
    /// is no cell of the protocol.
    #[must_use]
    pub fn group(&self, path: &[String]) -> Option<&[String]> {
        let cell = self.cells.iter().find(|cell| cell.path == path)?;
        Some(cell.same.as_deref().unwrap_or(&cell.path))
    }

    /// Whether `path` is the guard or one of the cells.
    #[must_use]
    pub fn names(&self, path: &[String]) -> bool {
        self.guard == path || self.cells.iter().any(|cell| cell.path == path)
    }

    /// Card CELL-PROTOCOL-PER-END-GUARDS: whether this is a per-end
    /// sentinel, a sentinel guard over exactly one `#fresh` cell, the guard
    /// itself (its `#same` names aside).
    #[must_use]
    pub fn per_end(&self) -> bool {
        let mut fresh = self.cells.iter().filter(|cell| cell.same.is_none());
        self.op.is_sentinel()
            && fresh.next().is_some_and(|cell| cell.path == self.guard)
            && fresh.next().is_none()
    }
}

/// The rule premise a protocol's host publishes with the rows resting on it.
pub const PROTOCOL_PREMISE: &str = "P-CELL-PROTOCOL-DECLARED";

/// Its statement.
pub const PROTOCOL_STATEMENT: &str = "a declared descriptor-cell protocol (quant cell) holds of every object of its record type at every store gateway rule 38c does not read: it is a declared assumption (rule 13i), never a checked closure";
