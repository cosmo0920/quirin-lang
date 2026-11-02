//! Gateway rule 38c (`docs/agents/rules/38c.md`; card
//! MK-EVENT-CELL-CROSS-FUNCTION, contract specification §C04.6.3, grammar
//! G05e). USER DECISION 2026-11-02 (verbatim): "B as recommended".
//!
//! A record type's declared **cell protocol** says: whenever the guard holds,
//! each `#fresh` cell names its own live descriptor and each `#same` cell its
//! target's; otherwise no cell names a live one. This module holds the
//! per-object fact a body's path keeps about one record ([`Fact`]) and the
//! decisions over it: what a guard test introduces, what a store requires,
//! and whether the record keeps its protocol at an exit or a call
//! ([`Fact::holds`]). Which actual is a record, and which statement is a
//! test, a store or a close, stay the host's. Every case it cannot decide
//! refuses; nothing here is `Unsafe`. The Lean model is
//! `QCore.DescriptorCellProtocol`, bound by [`PROTOCOL_VECTORS`].

use std::collections::BTreeMap;

use crate::cells::{JOINED, Refusal, Status};

/// The guard of a protocol (`#B.GUARD OP V`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Guard {
    /// `!= -1`, a sentinel cell: a live descriptor is never `-1`, so a
    /// creation into it makes the guard say live.
    NotMinusOne,
    /// `> 0`, a sentinel cell: descriptor `0` exists, so no creation
    /// establishes it; only a test does.
    Positive,
    /// `== 1`, a flag that is no cell: only a test, or a store of `1` over
    /// live cells, makes it say live.
    EqualOne,
}

impl Guard {
    /// Whether a stored integer makes the guard say live.
    #[must_use]
    pub const fn says_live(self, value: i64) -> bool {
        match self {
            Self::NotMinusOne => value != -1,
            Self::Positive => value > 0,
            Self::EqualOne => value == 1,
        }
    }
}

/// A record type's protocol: each cell path with the `#fresh` cell whose
/// identity it names (itself for a `#fresh` cell), the guard path, and the
/// guard. Paths are member segments joined by `.` (`ch_events[0]`,
/// `event.fd`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Protocol {
    pub record: String,
    pub cells: Vec<(String, String)>,
    pub guard: String,
    pub op: Guard,
}

impl Protocol {
    /// Whether `path` is one of the cells.
    #[must_use]
    pub fn is_cell(&self, path: &str) -> bool {
        self.cells.iter().any(|(cell, _)| cell == path)
    }

    /// The `#fresh` cells, in order: one identity each.
    pub fn groups(&self) -> impl Iterator<Item = &str> {
        self.cells
            .iter()
            .filter(|(cell, group)| cell == group)
            .map(|(cell, _)| cell.as_str())
    }
}

/// What the guard says on this path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GuardFact {
    Live,
    Dead,
    /// Joined from paths that disagree, or broken by a refused store.
    Conflict,
}

/// One record's fact on one path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fact {
    pub protocol: Protocol,
    /// What the guard says, when a test or a store decided it.
    pub guard: Option<GuardFact>,
    /// Whether the body operated on a cell or stored a member of the record:
    /// an untouched record keeps its protocol whatever the path knows.
    pub touched: bool,
    /// The caller's identities a test's live edge introduced, by cell path,
    /// not yet adopted into the table (the host adopts a cell when an
    /// operation first reads it, [`Self::adopt`]); a release through another
    /// name of the identity stales them ([`Self::stale_identity`]).
    pub entry: BTreeMap<String, (u64, Status)>,
}

impl Fact {
    /// The fact a guard test introduces on one edge: on the live edge each
    /// `#fresh` cell's identity (`base + i`) and its `#same` names, on the
    /// other edge none.
    #[must_use]
    pub fn introduced(protocol: &Protocol, live: bool, base: u64) -> Self {
        let mut entry = BTreeMap::new();
        if live {
            let groups = protocol.groups().map(str::to_owned).collect::<Vec<_>>();
            for (cell, group) in &protocol.cells {
                if let Some(index) = groups.iter().position(|g| g == group) {
                    entry.insert(cell.clone(), (base + index as u64, Status::Live));
                }
            }
        }
        Self {
            protocol: protocol.clone(),
            guard: Some(if live {
                GuardFact::Live
            } else {
                GuardFact::Dead
            }),
            touched: false,
            entry,
        }
    }

    /// The fact of a record a creation or a store reached first: nothing
    /// decided, touched.
    #[must_use]
    pub fn reached(protocol: &Protocol) -> Self {
        Self {
            protocol: protocol.clone(),
            guard: None,
            touched: true,
            entry: BTreeMap::new(),
        }
    }

    /// The pending identity of `path`, removed: the host records it in the
    /// table under the cell's key. The record is touched.
    pub fn adopt(&mut self, path: &str) -> Option<(u64, Status)> {
        self.touched = true;
        self.entry.remove(path)
    }

    /// A release of `identity` through any name stales its pending names.
    pub fn stale_identity(&mut self, identity: u64) {
        for (named, status) in self.entry.values_mut() {
            if *named == identity {
                *status = Status::Stale;
            }
        }
    }

    /// A cell's status on this path: the table's, else the pending entry's.
    fn status(
        &self,
        path: &str,
        table: &impl Fn(&str) -> Option<(u64, Status)>,
    ) -> Option<(u64, Status)> {
        table(path).or_else(|| self.entry.get(path).copied())
    }

    /// Every cell live, each under its group's identity, distinct groups
    /// distinct.
    fn all_live(&self, table: &impl Fn(&str) -> Option<(u64, Status)>) -> bool {
        let mut identities: BTreeMap<&str, u64> = BTreeMap::new();
        for group in self.protocol.groups() {
            match self.status(group, table) {
                Some((id, Status::Live)) if id != JOINED => {
                    if identities.values().any(|other| *other == id) {
                        return false;
                    }
                    identities.insert(group, id);
                }
                _ => return false,
            }
        }
        self.protocol.cells.iter().all(|(cell, group)| {
            matches!(self.status(cell, table), Some((id, Status::Live)) if Some(&id) == identities.get(group.as_str()))
        })
    }

    /// No cell may be live.
    fn none_live(&self, table: &impl Fn(&str) -> Option<(u64, Status)>) -> bool {
        self.protocol
            .cells
            .iter()
            .all(|(cell, _)| matches!(self.status(cell, table), None | Some((_, Status::Stale))))
    }

    /// Whether the record keeps its protocol on this path: untouched, or
    /// every cell live and the guard says live, or none live and the guard
    /// says dead. A `!= -1` guard says live whenever its cell holds a live
    /// descriptor.
    #[must_use]
    pub fn holds(&self, table: impl Fn(&str) -> Option<(u64, Status)>) -> bool {
        if !self.touched {
            return true;
        }
        let live = self.all_live(&table)
            && (self.protocol.op == Guard::NotMinusOne || self.guard == Some(GuardFact::Live));
        let dead = self.none_live(&table) && self.guard == Some(GuardFact::Dead);
        live || dead
    }

    /// Whether some cell may name a live descriptor (the release of the
    /// record holding it refuses).
    #[must_use]
    pub fn may_hold_live(&self, table: impl Fn(&str) -> Option<(u64, Status)>) -> bool {
        !self.none_live(&table)
    }

    /// A store of `value` (`None`: not an integer literal) to the member
    /// `path` (the guard or a cell).
    ///
    /// # Errors
    /// [`Refusal::Overwritten`] when the store overwrites a cell that may
    /// name a live descriptor; [`Refusal::ProtocolBroken`] for a value the
    /// protocol cannot read, a sentinel set to a live-looking number, or a
    /// flag set against the cells.
    pub fn store(
        &mut self,
        path: &str,
        value: Option<i64>,
        table: impl Fn(&str) -> Option<(u64, Status)>,
    ) -> Result<(), Refusal> {
        self.touched = true;
        let Some(value) = value else {
            self.guard = Some(GuardFact::Conflict);
            return Err(Refusal::ProtocolBroken);
        };
        if self.protocol.is_cell(path)
            && !matches!(self.status(path, &table), None | Some((_, Status::Stale)))
        {
            self.guard = Some(GuardFact::Conflict);
            return Err(Refusal::Overwritten);
        }
        if path != self.protocol.guard {
            return Ok(());
        }
        let live = self.protocol.op.says_live(value);
        if self.protocol.is_cell(path) {
            // A sentinel cell then holds a number no descriptor of the
            // protocol names: only its dead value keeps the protocol.
            if live {
                self.guard = Some(GuardFact::Conflict);
                return Err(Refusal::ProtocolBroken);
            }
            self.guard = Some(GuardFact::Dead);
            return Ok(());
        }
        let consistent = if live {
            self.all_live(&table)
        } else {
            self.none_live(&table)
        };
        if !consistent {
            self.guard = Some(GuardFact::Conflict);
            return Err(Refusal::ProtocolBroken);
        }
        self.guard = Some(if live {
            GuardFact::Live
        } else {
            GuardFact::Dead
        });
        Ok(())
    }

    /// Join an arriving path's fact of the same record: the guard where both
    /// agree (else `Conflict`), the pending names both share.
    #[must_use]
    pub fn join(&self, other: &Self) -> Self {
        let guard = if self.guard == other.guard {
            self.guard
        } else {
            Some(GuardFact::Conflict)
        };
        let entry = self
            .entry
            .iter()
            .filter(|(path, named)| other.entry.get(*path) == Some(*named))
            .map(|(path, named)| (path.clone(), *named))
            .collect();
        Self {
            protocol: self.protocol.clone(),
            guard,
            touched: self.touched || other.touched,
            entry,
        }
    }
}

/// The shared vectors (`QCore.DescriptorCellProtocol.protocol_vectors_hold`):
/// a trace over one record of a `!= -1` protocol with cells `r` (the guard),
/// `w`, and `m` (`#same(r)`), and whether it ends with the protocol kept
/// (`holds`) or refuses (`broken`, `overwritten`, `double-release`,
/// `undecided`). Steps: `test live` / `test dead` (introduce), `release c`
/// (a close or guarded release of a cell), `store c V`, `create` (a channel
/// creation into `r`, `w`, `m` its witness decides live), `fail` (its
/// failure class), `join` (with the dead edge of a test, untouched).
pub const PROTOCOL_VECTORS: &[(&str, &str)] = &[
    ("test live", "holds"),
    ("test dead", "holds"),
    ("test live; release r; release w; store r -1", "holds"),
    ("test live; release r; release w", "broken"),
    (
        "test live; release r; release w; store r -1; store w -1",
        "holds",
    ),
    ("test live; release r; release m", "double-release"),
    ("test live; store r -1", "overwritten"),
    ("test dead; release r", "undecided"),
    ("store r -1; create", "holds"),
    ("store r -1; fail", "holds"),
    ("store r 5", "broken"),
    ("test live; release r; release w; store r -1; join", "holds"),
    ("test live; release w; join", "broken"),
];

/// The protocol the vectors run over.
#[must_use]
pub fn vector_protocol() -> Protocol {
    Protocol {
        record: "ch".to_owned(),
        cells: vec![
            ("m".to_owned(), "r".to_owned()),
            ("r".to_owned(), "r".to_owned()),
            ("w".to_owned(), "w".to_owned()),
        ],
        guard: "r".to_owned(),
        op: Guard::NotMinusOne,
    }
}

const fn outcome_word(refusal: Refusal) -> &'static str {
    match refusal {
        Refusal::Undecided => "undecided",
        Refusal::Overwritten => "overwritten",
        Refusal::DoubleRelease => "double-release",
        Refusal::ProtocolBroken => "broken",
        _ => "other",
    }
}

/// Adopt `cell`'s pending identity into the table, as the host does when an
/// operation first reads a cell of a record with a fact.
fn adopt(cells: &mut crate::cells::Cells<String>, fact: &mut Fact, cell: &str) {
    if cells.get(&cell.to_owned()).is_none()
        && let Some((identity, status)) = fact.adopt(cell)
    {
        cells.adopt(cell.to_owned(), identity, status);
    }
}

/// Run one vector; the outcome word.
#[must_use]
pub fn run_protocol_vector(trace: &str) -> &'static str {
    use crate::cells::Cells;
    let protocol = vector_protocol();
    let mut cells: Cells<String> = Cells::default();
    let mut fact: Option<Fact> = None;
    for step in trace.split(';').map(str::trim) {
        let outcome: Result<(), Refusal> =
            match step.split_whitespace().collect::<Vec<_>>().as_slice() {
                ["test", edge] => {
                    fact = Some(Fact::introduced(&protocol, *edge == "live", 100));
                    Ok(())
                }
                ["release", cell] => {
                    let fact = fact.get_or_insert_with(|| Fact::reached(&protocol));
                    adopt(&mut cells, fact, cell);
                    let key = (*cell).to_owned();
                    match cells.get(&key) {
                        None => Err(Refusal::Undecided),
                        Some((identity, _)) => {
                            let released = cells.release(Some(&key), |_| true).map(|_| ());
                            if released.is_ok() {
                                fact.stale_identity(identity);
                            }
                            released
                        }
                    }
                }
                ["store", cell, value] => {
                    let fact = fact.get_or_insert_with(|| Fact::reached(&protocol));
                    let value = value.parse::<i64>().ok();
                    let table = cells.clone();
                    fact.store(cell, value, |path| table.get(&path.to_owned()))
                }
                [kind @ ("create" | "fail")] => {
                    fact.get_or_insert_with(|| Fact::reached(&protocol)).touched = true;
                    let created = cells
                        .create(Some(&"r".to_owned()), 1, true)
                        .and_then(|()| cells.create(Some(&"w".to_owned()), 2, true))
                        .and_then(|()| cells.alias(Some(&"m".to_owned()), &"r".to_owned()));
                    cells.witness(1, *kind == "create");
                    cells.witness(2, *kind == "create");
                    created
                }
                ["join"] => {
                    let other = Fact::introduced(&protocol, false, 100);
                    if let Some(current) = &fact {
                        fact = Some(current.join(&other));
                    }
                    cells.join(&Cells::default());
                    Ok(())
                }
                _ => return "malformed",
            };
        if let Err(refusal) = outcome {
            return outcome_word(refusal);
        }
    }
    match &fact {
        Some(fact) if !fact.holds(|path| cells.get(&path.to_owned())) => "broken",
        _ => "holds",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shared_protocol_vectors_hold() {
        for (trace, expected) in PROTOCOL_VECTORS {
            assert_eq!(run_protocol_vector(trace), *expected, "{trace}");
        }
    }

    #[test]
    fn a_creation_never_establishes_a_positive_or_flag_guard() {
        for op in [Guard::Positive, Guard::EqualOne] {
            let mut protocol = vector_protocol();
            protocol.op = op;
            let mut fact = Fact::reached(&protocol);
            fact.guard = Some(GuardFact::Dead);
            let live = |path: &str| match path {
                "r" | "m" => Some((1, Status::Live)),
                "w" => Some((2, Status::Live)),
                _ => None,
            };
            assert!(!fact.holds(live), "{op:?}");
        }
    }

    #[test]
    fn an_untouched_record_keeps_its_protocol() {
        let fact = Fact::introduced(&vector_protocol(), true, 7);
        assert!(fact.holds(|_| None));
        let mut joined = fact.join(&Fact::introduced(&vector_protocol(), false, 7));
        assert_eq!(joined.guard, Some(GuardFact::Conflict));
        assert!(joined.holds(|_| None));
        joined.touched = true;
        assert!(!joined.holds(|_| Some((7, Status::Stale))));
    }
}
