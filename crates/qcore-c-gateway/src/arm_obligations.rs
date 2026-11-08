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
//! Card **ARM-OBLIGATIONS-UNIT-GRAIN**: the grain is the row where the
//! attribution is established, the unit where it is not ([`FrameCarriage`]).
//! An admission of a named declaration (rule 37's checked specification of
//! `rd_calloc`) is carried by the rows whose derivation reaches that
//! declaration: the row itself, or a function its work consulted, through
//! every in-unit function with work in the frame, transitively
//! (`contract_dependencies::attribution::reaching`, read in every run, never
//! from the audit's recording). Every other admission (a host asked during
//! the analysis, a nested analysis handing its carriage up, a test
//! injection) has no established attribution and keeps the unit grain:
//! every row of the unit carries it, as before. Either way a row's set is a
//! subset of the unit's and holds every obligation its derivation used.
//! `QCore.ArmObligations` models the law.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

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

/// Card ARM-OBLIGATIONS-UNIT-GRAIN: one frame's obligations by attribution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[must_use = "a carried obligation must reach a row"]
pub struct FrameCarriage {
    /// Carried by every row of the unit: the attribution is not established.
    pub unit: Carried,
    /// Carried by the rows whose derivation reaches the named declaration.
    pub declarations: BTreeMap<String, Carried>,
}

impl FrameCarriage {
    /// Everything the frame carried, whatever its attribution (the unit grain).
    pub fn all(&self) -> Carried {
        let mut all = self.unit.clone();
        for carried in self.declarations.values() {
            all.extend(carried.clone());
        }
        all
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.unit.is_empty() && self.declarations.values().all(Carried::is_empty)
    }

    /// What one row carries: the unit's obligations, and every declaration's
    /// whose name the row's derivation reaches (`reaches`). A subset of
    /// [`Self::all`].
    pub fn row(&self, reaches: impl Fn(&str) -> bool) -> Carried {
        let mut row = self.unit.clone();
        for (declaration, carried) in &self.declarations {
            if reaches(declaration) {
                row.extend(carried.clone());
            }
        }
        row
    }
}

thread_local! {
    static FRAMES: RefCell<Vec<FrameCarriage>> = const { RefCell::new(Vec::new()) };
}

/// One unit's analysis: the obligations carried while it is open. Closing
/// (or dropping) it hands what it carried to the enclosing frame too, so a
/// nested analysis whose result the enclosing one reads never sheds one;
/// the enclosing frame carries it at unit grain, since which of its rows
/// read the nested result is not attributed.
#[must_use = "a frame collects the obligations its unit's rows carry"]
pub(crate) struct Frame {
    depth: usize,
    attribution: usize,
    closed: bool,
}

#[cfg(test)]
thread_local! {
    /// Test only: what the next frame opens with, standing in for a package
    /// admission with obligations reached during that unit's analysis whose
    /// attribution is not established (unit grain).
    pub(crate) static INJECTED: RefCell<Option<Carried>> = const { RefCell::new(None) };
    /// Test only: likewise, attributed to the named declarations.
    pub(crate) static INJECTED_DECLARATIONS: RefCell<BTreeMap<String, Carried>> =
        const { RefCell::new(BTreeMap::new()) };
}

impl Frame {
    pub(crate) fn open() -> Self {
        #[cfg(test)]
        let opening = FrameCarriage {
            unit: INJECTED.with(|injected| injected.borrow_mut().take().unwrap_or_default()),
            declarations: INJECTED_DECLARATIONS
                .with(|injected| std::mem::take(&mut *injected.borrow_mut())),
        };
        #[cfg(not(test))]
        let opening = FrameCarriage::default();
        let attribution = crate::contract_dependencies::attribution::start();
        let depth = FRAMES.with(|frames| {
            let mut frames = frames.borrow_mut();
            frames.push(opening);
            frames.len()
        });
        Self {
            depth,
            attribution,
            closed: false,
        }
    }

    /// Carry the obligations of admitted declarations whose lowering this
    /// unit's analysis uses, each attributed to its declaration's name.
    pub(crate) fn carry_declarations(&self, carried: BTreeMap<String, Carried>) {
        FRAMES.with(|frames| {
            if let Some(frame) = frames.borrow_mut().get_mut(self.depth - 1) {
                for (declaration, obligations) in carried {
                    if !obligations.is_empty() {
                        frame
                            .declarations
                            .entry(declaration)
                            .or_default()
                            .extend(obligations);
                    }
                }
            }
        });
    }

    /// Everything this frame carried and the attribution its analysis
    /// noted; the enclosing frame keeps a copy of the carriage, at unit
    /// grain.
    pub(crate) fn close(
        mut self,
    ) -> (
        FrameCarriage,
        crate::contract_dependencies::attribution::Attribution,
    ) {
        self.closed = true;
        let carried = pop(self.depth);
        (
            carried,
            crate::contract_dependencies::attribution::finish(self.attribution),
        )
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        if !self.closed {
            let _ = pop(self.depth);
            let _ = crate::contract_dependencies::attribution::finish(self.attribution);
        }
    }
}

fn pop(depth: usize) -> FrameCarriage {
    FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        // Frames close in order; an inner frame left open by a panic is
        // folded into this one, at unit grain, rather than lost.
        let mut carried = FrameCarriage::default();
        let mut inner = Carried::default();
        while frames.len() >= depth {
            let Some(frame) = frames.pop() else {
                break;
            };
            if frames.len() + 1 == depth {
                carried = frame;
            } else {
                inner.extend(frame.all());
            }
        }
        carried.unit.extend(inner);
        if let Some(parent) = frames.last_mut() {
            parent.unit.extend(carried.all());
        }
        carried
    })
}

/// Card OUT-PARAMETER-SCALAR-FRAME-PREMISE (rule 11r(7)): carry the premise
/// `name` of `rule` into the innermost open frame, attributed to the in-unit
/// function `declaration`, so every row whose derivation reaches it lists the
/// premise. `false` when no frame is open (nothing reads the relation then).
pub(crate) fn carry_declaration_premise(declaration: &str, rule: &str, name: &str) -> bool {
    FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        let Some(frame) = frames.last_mut() else {
            return false;
        };
        frame
            .declarations
            .entry(declaration.to_owned())
            .or_default()
            .premises
            .insert(Carriage {
                rule: rule.to_owned(),
                name: name.to_owned(),
            });
        true
    })
}

/// Carry `carried` into the innermost open frame at unit grain (its
/// enclosing frames receive it when it closes); `false` when none is open,
/// so the admission it came from cannot be carried.
fn carry_open(carried: &Carried) -> bool {
    FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        let Some(frame) = frames.last_mut() else {
            return false;
        };
        frame.unit.extend(carried.clone());
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

/// Card ARM-OBLIGATIONS-UNIT-GRAIN: for every declaration `carried`
/// attributes an admission to, the rows of `functions` whose derivation
/// reaches it ([`crate::contract_dependencies::attribution::reaching`]), read
/// back against the unit's input `environment`.
pub(crate) fn reaching_rows(
    carried: &FrameCarriage,
    attribution: &crate::contract_dependencies::attribution::Attribution,
    functions: &BTreeMap<String, crate::legacy_c::LegacyFunctionSummary>,
    environment: &qcore_schemas::contract::ResourceContracts,
) -> BTreeMap<String, BTreeSet<String>> {
    if carried.declarations.is_empty() {
        return BTreeMap::new();
    }
    let rows = functions
        .iter()
        .map(|(name, summary)| {
            (
                name.clone(),
                crate::contract_dependencies::cfg_callees(&summary.cfg),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let targets = carried
        .declarations
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    crate::contract_dependencies::attribution::reaching(attribution, &rows, environment, &targets)
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
