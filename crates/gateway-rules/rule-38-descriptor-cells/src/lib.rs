//! Gateway rule 38 (`docs/agents/rules/38.md`; card MK-EVENT-DESCRIPTOR-QUANT,
//! contract specification §C04.6.2).
//!
//! USER DECISION 2026-11-01 (verbatim): "I approve the changes of D1-D5." A
//! **descriptor cell** is a place that holds a descriptor number (the member
//! of an object a formal designates, or the `int` a slot points to). A
//! descriptor is an identity and a cell is a name of it: a release through any
//! name discharges the identity once and leaves every other name stale. A
//! call's integer result guarding a cell state is its **witness**.
//!
//! This package holds the cell table ([`cells::Cells`]) the host's route-G
//! hook reads and writes, and decides nothing else: which actual a cell key
//! is, and when a name is a witness, stay the host's. Its registry id is
//! `38`; with `38` in `QCORE_GATEWAY_RULES_OFF` the host refuses every call
//! of a cell-bearing declaration (the reading is not asked). The Lean model
//! is `QCore.DescriptorCells`, bound by [`cells::DECISION_VECTORS`].
//!
//! Card MK-EVENT-FIRST-MEMBER-CELL: [`initial_member`] decides which object a
//! member cell's actual names, reading a pointer to a record whose first
//! member is the event as that member (C11 6.7.2.1p15), bound by
//! [`initial_member::INITIAL_MEMBER_VECTORS`].
//!
//! Card CELL-PROTOCOL-CLOSE-ACTUAL (rule 38d): [`close_actual`] decides which
//! refused `close(r->CELL)` actual the access gate admits, and what rule 38c's
//! close makes of it on one path, bound by
//! [`close_actual::ADMISSION_VECTORS`] and [`close_actual::READING_VECTORS`].

pub mod cells;
pub mod close_actual;
pub mod initial_member;
pub mod protocol;

pub use cells::{Cells, Equality, Refusal, Released, Status};
pub use initial_member::{Record, View};

/// The package's registry id.
pub const ID: &str = "38";
