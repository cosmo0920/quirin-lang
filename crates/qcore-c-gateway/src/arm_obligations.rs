//! Card **ARM-OBLIGATIONS** (`GATEWAY-RULE-API-ARM-OBLIGATIONS`): the
//! gateway carries a rule package's admission obligations into the published
//! row instead of refusing them.
//!
//! A package admission is `(lowering, obligations)`
//! ([`qcore_gateway_rule_api::Admission`]). Before this card every host
//! refused an admission with an obligation, because no position had a row to
//! carry it. Now:
//!
//! * every consultation of a package in this crate goes through [`consult`]
//!   (or [`consult_held`]); none reads the API's own `consult` or an
//!   admission's lowering directly (`tests/unit/arm_obligations.rs` scans the
//!   crate for that, so a host that would drop an obligation fails the suite);
//! * an admission without obligations is its lowering, exactly as before;
//! * an admission with obligations is carried into every open **unit frame**
//!   ([`Frame`]): the analysis of one translation unit
//!   (`legacy_c::analyze_translation_unit_under_holds`) opens one, and every
//!   row that analysis publishes carries what its frame collected ([`apply`]).
//!   No frame open (a route that publishes no unit's rows) means the
//!   obligation cannot be carried, and the admission is refused (fail-closed);
//! * a position whose lowering is used only later (the checked-specification
//!   settlement, where rule 37's `asserted_allocation` premise arm answers)
//!   takes the admission with [`consult_held`] and carries its obligations
//!   wherever it uses the lowering: the settlement hands the obligations of
//!   every specification that stands to the unit's frame, and an admission
//!   whose specification is withdrawn has no row resting on it.
//!
//! What a row carries ([`apply`]): every [`Obligation::Premise`] among its
//! assumptions (`LegacyFunctionSummary::rule_obligations`, published by the
//! audit as the row's `rule_obligations` and folded into its verdict
//! contract's `assumptions`), and every [`Obligation::Open`] as an
//! `Unsupported` diagnostic, so the row is `Unknown` (or `Unsafe`, never
//! better) and stays in the denominator. A row is `Safe` only when it
//! carries no open obligation.
//!
//! The unit is the carriage's grain: an obligation reached anywhere in the
//! unit's analysis is carried by every row of the unit, and a nested analysis
//! hands what it carried to the enclosing frame as well. That over-approximates
//! which rows rest on it (an assumption too many, never one too few).
//! `QCore.ArmObligations` models the law.

use std::cell::RefCell;
use std::collections::BTreeSet;

use qcore_gateway_rule_api::{Admission, Answer, Obligation};

/// One carried obligation: the rule whose package's admission rests on it,
/// and the premise's name or the open obligation's reserved reason key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Carriage {
    pub rule: String,
    pub name: String,
}

/// What a row (or a unit frame) carries: named premises, which the row's
/// assumptions list, and open obligations, which keep it out of `Safe`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[must_use = "a carried obligation must reach a row"]
pub struct Carried {
    pub premises: BTreeSet<Carriage>,
    pub open: BTreeSet<Carriage>,
}

impl Carried {
    /// The obligations of one admission of `rule`'s package.
    pub fn of(rule: &str, obligations: &[Obligation]) -> Self {
        let mut carried = Self::default();
        for obligation in obligations {
            let (set, name) = match obligation {
                Obligation::Premise(name) => (&mut carried.premises, name),
                Obligation::Open(key) => (&mut carried.open, key),
            };
            set.insert(Carriage {
                rule: rule.to_owned(),
                name: name.clone(),
            });
        }
        carried
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.premises.is_empty() && self.open.is_empty()
    }

    /// Union: nothing either side carried is erased.
    pub fn extend(&mut self, other: Self) {
        self.premises.extend(other.premises);
        self.open.extend(other.open);
    }
}

thread_local! {
    static FRAMES: RefCell<Vec<Carried>> = const { RefCell::new(Vec::new()) };
}

/// One unit's analysis: the obligations carried while it is open. Closing
/// (or dropping) it hands what it carried to the enclosing frame too, so a
/// nested analysis whose result the enclosing one reads never sheds one.
#[must_use = "a frame collects the obligations its unit's rows carry"]
pub(crate) struct Frame {
    depth: usize,
    closed: bool,
}

#[cfg(test)]
thread_local! {
    /// Test only: what the next frame opens with, standing in for a package
    /// admission with obligations reached during that unit's analysis.
    pub(crate) static INJECTED: RefCell<Option<Carried>> = const { RefCell::new(None) };
}

impl Frame {
    pub(crate) fn open() -> Self {
        #[cfg(test)]
        let opening = INJECTED.with(|injected| injected.borrow_mut().take().unwrap_or_default());
        #[cfg(not(test))]
        let opening = Carried::default();
        let depth = FRAMES.with(|frames| {
            let mut frames = frames.borrow_mut();
            frames.push(opening);
            frames.len()
        });
        Self {
            depth,
            closed: false,
        }
    }

    /// Carry obligations whose lowering this unit's analysis uses.
    pub(crate) fn carry(&self, carried: Carried) {
        FRAMES.with(|frames| {
            if let Some(frame) = frames.borrow_mut().get_mut(self.depth - 1) {
                frame.extend(carried);
            }
        });
    }

    /// Everything this frame carried; the enclosing frame keeps a copy.
    pub(crate) fn close(mut self) -> Carried {
        self.closed = true;
        pop(self.depth)
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        if !self.closed {
            let _ = pop(self.depth);
        }
    }
}

fn pop(depth: usize) -> Carried {
    FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        // Frames close in order; an inner frame left open by a panic is
        // folded into this one rather than lost.
        let mut carried = Carried::default();
        while frames.len() >= depth {
            if let Some(frame) = frames.pop() {
                carried.extend(frame);
            }
        }
        if let Some(parent) = frames.last_mut() {
            parent.extend(carried.clone());
        }
        carried
    })
}

/// Carry `carried` into every open frame; `false` when none is open, so the
/// admission it came from cannot be carried.
fn carry_open(carried: &Carried) -> bool {
    FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        if frames.is_empty() {
            return false;
        }
        for frame in frames.iter_mut() {
            frame.extend(carried.clone());
        }
        true
    })
}

/// An admission's lowering once its obligations are carried, or `None` when
/// they cannot be (no unit frame is open): never the lowering without them.
pub(crate) fn carry<L>(rule: &str, admission: Admission<L>) -> Option<L> {
    let Admission {
        lowering,
        obligations,
    } = admission;
    if obligations.is_empty() {
        return Some(lowering);
    }
    carry_open(&Carried::of(rule, &obligations)).then_some(lowering)
}

/// [`carry`] of an answer: a refusal is `None`.
pub(crate) fn carry_answer<L>(rule: &str, answer: Answer<L>) -> Option<L> {
    carry(rule, answer.into_admission()?)
}

/// The position's decision: the index of the entry that admitted and the
/// lowering, its obligations carried.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Decided<L> {
    pub(crate) index: usize,
    pub(crate) lowering: L,
}

/// `qcore_gateway_rule_api::consult`, carrying the winning admission's
/// obligations ([`carry`]). An admission whose obligations cannot be carried
/// ends the table, as a refusal of the position did before this card.
pub(crate) fn consult<E, L>(
    entries: &[E],
    rule: impl Fn(&E) -> &str,
    enabled: impl Fn(&E) -> bool,
    ask: impl FnMut(usize, &E) -> Answer<L>,
) -> Option<Decided<L>> {
    let consulted = qcore_gateway_rule_api::consult(entries, enabled, ask)?;
    let rule = rule(&entries[consulted.index]);
    carry(rule, consulted.admission).map(|lowering| Decided {
        index: consulted.index,
        lowering,
    })
}

/// An admission whose lowering its host uses later: the obligations travel
/// with it and the host carries them wherever it uses the lowering.
#[must_use = "a held admission's obligations must be carried where its lowering is used"]
pub(crate) struct Held<L> {
    pub(crate) lowering: L,
    pub(crate) obligations: Vec<Obligation>,
    pub(crate) carried: Carried,
}

/// `qcore_gateway_rule_api::consult` for a host that carries the
/// obligations itself ([`Held`]).
pub(crate) fn consult_held<E, L>(
    entries: &[E],
    rule: impl Fn(&E) -> &str,
    enabled: impl Fn(&E) -> bool,
    ask: impl FnMut(usize, &E) -> Answer<L>,
) -> Option<Held<L>> {
    let consulted = qcore_gateway_rule_api::consult(entries, enabled, ask)?;
    let rule = rule(&entries[consulted.index]);
    let Admission {
        lowering,
        obligations,
    } = consulted.admission;
    Some(Held {
        lowering,
        carried: Carried::of(rule, &obligations),
        obligations,
    })
}

/// The diagnostic an open obligation leaves on a row.
#[must_use]
pub fn open_description(open: &Carriage) -> String {
    format!(
        "gateway rule {} admits this unit's analysis only with the obligation `{}` left open",
        open.rule, open.name
    )
}

/// Carry a unit frame's obligations into one of its rows: the premises into
/// its assumptions, each open obligation as a diagnostic that keeps it out
/// of `Safe` (`QCore.ArmObligations.apply`).
pub(crate) fn apply(
    summary: &mut crate::legacy_c::LegacyFunctionSummary,
    carried: &Carried,
    line: usize,
) {
    if carried.is_empty() {
        return;
    }
    summary.rule_obligations.extend(carried.clone());
    if carried.open.is_empty() {
        return;
    }
    for open in &carried.open {
        summary.diagnostics.push(crate::legacy_c::LegacyDiagnostic {
            function: summary.function.clone(),
            line,
            kind: crate::legacy_c::LegacyDiagnosticKind::Unsupported {
                description: open_description(open),
            },
        });
    }
    summary.diagnostics.sort();
    summary.diagnostics.dedup();
    summary.status = crate::legacy_c::verdict::classify(&summary.diagnostics);
}

#[cfg(test)]
#[path = "../tests/unit/arm_obligations.rs"]
mod tests;
