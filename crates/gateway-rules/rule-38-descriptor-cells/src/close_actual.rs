//! Gateway rule 38d (`docs/agents/rules/38d.md`; card
//! CELL-PROTOCOL-CLOSE-ACTUAL, registered by MK-EVENT-CELL-CROSS-FUNCTION).
//! USER DECISION 2026-11-02 (verbatim): "B as recommended"; its Q3: a close
//! of a declared cell member is that cell's `#stale`.
//!
//! Rule 38c reads `close(r->CELL)` of a declared protocol cell as that cell's
//! `#stale`, but the access gate refused the integer member actual at
//! `close`'s custody position first (W44-INDIRECT-CALL-ARGUMENT-ACCESS
//! premise 1). This module holds the two decisions that narrow that refusal,
//! and nothing else: which statement, callee, position, kind and actual the
//! gate admits ([`admits`]: exactly [`ADMITTED`]), and what rule 38c's close
//! makes of an admitted actual on one path ([`reading`]: the cell's `#stale`
//! when it names a live identity, otherwise a refusal, never a silent
//! release). Which statement is rule 38c's close, which callee is in force and
//! which actual is the member read stay the host's. The Lean model is
//! `QCore.CellProtocolCloseActual`, bound by [`ADMISSION_VECTORS`] and
//! [`READING_VECTORS`].

use crate::cells::{JOINED, Refusal, Status};

/// The statement the access gate refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Statement {
    /// Exactly rule 38c's close: `close(r->CELL);` with `r` declared a
    /// pointer to a record whose declared protocol names the one-segment
    /// `CELL` among its cells.
    ProtocolClose,
    /// Anything else: an undeclared record, a member that is not a cell, a
    /// multi-segment path, a copied number (Q4), a used result.
    Other,
}

/// The declaration in force for the callee.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Callee {
    /// A function-map (direct) contract.
    FunctionMap,
    /// A withheld declaration or an inferred lifetime summary (rule 12t).
    Withheld,
    /// No contract.
    Uncontracted,
}

/// The actual's position in that contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    /// A `resource_released` position.
    Release,
    /// Any other custody-taking position (`owned`, `consumed`, `out`,
    /// `stored`, `removed`).
    OtherCustody,
    /// A `resource_borrowed` position (W44's own admission, not this rule's).
    Borrowed,
    /// Beyond the declared parameter vector.
    Beyond,
}

/// The kind the contract declares at that position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Descriptor,
    /// Heap or any other kind.
    Other,
}

/// The actual itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actual {
    /// The integer member read `r->CELL` (or `r->CELL[K]`, a constant
    /// in-bounds element of an integer array member) of the step's own root
    /// and cell, its only base `r`.
    CellRead,
    /// Anything else.
    Other,
}

/// One refused statement, as the host reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Site {
    /// Whether rule 38 is in force (`38` not in `QCORE_GATEWAY_RULES_OFF`).
    pub rule_on: bool,
    pub statement: Statement,
    pub callee: Callee,
    pub position: Position,
    pub kind: Kind,
    pub actual: Actual,
}

/// The one site the gate admits.
pub const ADMITTED: Site = Site {
    rule_on: true,
    statement: Statement::ProtocolClose,
    callee: Callee::FunctionMap,
    position: Position::Release,
    kind: Kind::Descriptor,
    actual: Actual::CellRead,
};

/// Whether the access gate admits the actual, condition by condition; the
/// set is exactly [`ADMITTED`] (`QCore.CellProtocolCloseActual`'s
/// `the_admission_set_is_exactly_the_stated_one`). Every other site keeps
/// the gate's refusal byte for byte.
#[must_use]
pub fn admits(site: Site) -> bool {
    site.rule_on
        && site.statement == Statement::ProtocolClose
        && site.callee == Callee::FunctionMap
        && site.position == Position::Release
        && site.kind == Kind::Descriptor
        && site.actual == Actual::CellRead
}

/// What one path's table says of the closed cell, after its pending name is
/// adopted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// Recorded live under one identity.
    Live,
    /// Live under the identity a join of two disagreeing paths gave.
    JoinedLive,
    /// May be live (a class no witness decided).
    Maybe,
    /// Released through some name.
    Stale,
    /// Not recorded.
    Absent,
}

impl Cell {
    /// The view of a table entry.
    #[must_use]
    pub fn of(entry: Option<(u64, Status)>) -> Self {
        match entry {
            Some((identity, Status::Live)) if identity == JOINED => Self::JoinedLive,
            Some((_, Status::Live)) => Self::Live,
            Some((_, Status::Maybe)) => Self::Maybe,
            Some((_, Status::Stale)) => Self::Stale,
            None => Self::Absent,
        }
    }
}

/// What rule 38c's close makes of an admitted actual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// The cell's identity is released once: every name of it is stale
    /// (Q3), the record's pending names of it too.
    Stale,
    /// Rule 38c reads nothing here (no fact of a caller's record on this
    /// path): the gate's own refusal stands.
    Gate,
    /// Rule 38c refuses.
    Refused(Refusal),
}

/// Rule 38c's reading of an admitted close: `fact` says whether the path
/// holds a fact of a caller's record for the step's root. Only a live,
/// single identity is released; never a silent release.
#[must_use]
pub fn reading(fact: bool, cell: Cell) -> Reading {
    if !fact {
        return Reading::Gate;
    }
    match cell {
        Cell::Live => Reading::Stale,
        Cell::Stale => Reading::Refused(Refusal::DoubleRelease),
        Cell::JoinedLive | Cell::Maybe | Cell::Absent => Reading::Refused(Refusal::Undecided),
    }
}

/// [`admits`]'s table: [`ADMITTED`], then each single departure from it.
/// Shared with `QCore.CellProtocolCloseActual.admissionVectors` entry for
/// entry.
pub const ADMISSION_VECTORS: &[(Site, bool)] = &[
    (ADMITTED, true),
    (
        Site {
            rule_on: false,
            ..ADMITTED
        },
        false,
    ),
    (
        Site {
            statement: Statement::Other,
            ..ADMITTED
        },
        false,
    ),
    (
        Site {
            callee: Callee::Withheld,
            ..ADMITTED
        },
        false,
    ),
    (
        Site {
            callee: Callee::Uncontracted,
            ..ADMITTED
        },
        false,
    ),
    (
        Site {
            position: Position::OtherCustody,
            ..ADMITTED
        },
        false,
    ),
    (
        Site {
            position: Position::Borrowed,
            ..ADMITTED
        },
        false,
    ),
    (
        Site {
            position: Position::Beyond,
            ..ADMITTED
        },
        false,
    ),
    (
        Site {
            kind: Kind::Other,
            ..ADMITTED
        },
        false,
    ),
    (
        Site {
            actual: Actual::Other,
            ..ADMITTED
        },
        false,
    ),
];

/// [`reading`]'s table, shared with
/// `QCore.CellProtocolCloseActual.readingVectors` entry for entry.
pub const READING_VECTORS: &[(bool, Cell, Reading)] = &[
    (true, Cell::Live, Reading::Stale),
    (true, Cell::Stale, Reading::Refused(Refusal::DoubleRelease)),
    (true, Cell::JoinedLive, Reading::Refused(Refusal::Undecided)),
    (true, Cell::Maybe, Reading::Refused(Refusal::Undecided)),
    (true, Cell::Absent, Reading::Refused(Refusal::Undecided)),
    (false, Cell::Live, Reading::Gate),
    (false, Cell::Stale, Reading::Gate),
    (false, Cell::JoinedLive, Reading::Gate),
    (false, Cell::Maybe, Reading::Gate),
    (false, Cell::Absent, Reading::Gate),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_vectors_hold() {
        for (site, admitted) in ADMISSION_VECTORS {
            assert_eq!(admits(*site), *admitted, "{site:?}");
        }
    }

    /// The admission set is exactly [`ADMITTED`], over every site.
    #[test]
    fn the_admission_set_is_exactly_the_stated_one() {
        for rule_on in [false, true] {
            for statement in [Statement::ProtocolClose, Statement::Other] {
                for callee in [Callee::FunctionMap, Callee::Withheld, Callee::Uncontracted] {
                    for position in [
                        Position::Release,
                        Position::OtherCustody,
                        Position::Borrowed,
                        Position::Beyond,
                    ] {
                        for kind in [Kind::Descriptor, Kind::Other] {
                            for actual in [Actual::CellRead, Actual::Other] {
                                let site = Site {
                                    rule_on,
                                    statement,
                                    callee,
                                    position,
                                    kind,
                                    actual,
                                };
                                assert_eq!(admits(site), site == ADMITTED, "{site:?}");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn reading_vectors_hold() {
        for (fact, cell, expected) in READING_VECTORS {
            assert_eq!(reading(*fact, *cell), *expected, "{fact} {cell:?}");
        }
    }

    /// The table views every status, the join's identity apart.
    #[test]
    fn the_cell_view_is_total() {
        assert_eq!(Cell::of(Some((7, Status::Live))), Cell::Live);
        assert_eq!(Cell::of(Some((JOINED, Status::Live))), Cell::JoinedLive);
        assert_eq!(Cell::of(Some((7, Status::Maybe))), Cell::Maybe);
        assert_eq!(Cell::of(Some((7, Status::Stale))), Cell::Stale);
        assert_eq!(Cell::of(None), Cell::Absent);
    }

    /// Only a live single identity is released: no other reading of a path
    /// with a fact is `Stale`, and no path without one is.
    #[test]
    fn only_a_live_cell_is_staled() {
        for fact in [false, true] {
            for cell in [
                Cell::Live,
                Cell::JoinedLive,
                Cell::Maybe,
                Cell::Stale,
                Cell::Absent,
            ] {
                assert_eq!(
                    reading(fact, cell) == Reading::Stale,
                    fact && cell == Cell::Live
                );
            }
        }
    }
}
