//! Card **MK-EVENT-DESCRIPTOR-QUANT** (gateway rule 38, contract spec
//! §C04.6.2): the route-G host of the descriptor-cell package
//! (`rule_38_descriptor_cells`). USER DECISION 2026-11-01 (verbatim): "I
//! approve the changes of D1-D5."
//!
//! Thin hooks, each the identity in every function that calls no declaration
//! carrying a cell state (the table and the witnesses stay empty, so the join
//! and the work-list schedule are as they were):
//!
//! 1. [`operands`] (asked by rule 12t's `retaining_call`): each state's
//!    actual as spelled (a member cell: the `data` actual's object viewed as
//!    `struct TAG`, checked against Clang's type of every actual of the call,
//!    or, card MK-EVENT-FIRST-MEMBER-CELL, a pointer to a record whose first
//!    member is the `struct TAG`, read as that member;
//!    a slot cell: `&L` or `&X->f[K]`), the guard's actuals, and the removals
//!    the guard governs.
//! 2. [`apply`] (the transfer of the call, after rule 12t's records): the
//!    creations, second names and releases against the path's table; the
//!    guard decided from identities (equal, distinct, or undecided: the call
//!    refuses, and a guarded removal keeps its record and poisons what it may
//!    name).
//! 3. [`record_witness`] / [`forget_witnesses`] / [`refine`]: the call's
//!    result held by a plain name witnesses the creation's class until the
//!    name is written; a validity test (`-1` guards) or a null test (`0`
//!    guards) of it decides the class on each edge.
//! 4. [`report_exit`] and [`refuse_release_of_holder`]: a cell that may be
//!    live at an exit refuses (`descriptor-cell-escapes`), and so does the
//!    release of the object holding one.
//!
//! Everything refuses rather than approximates; nothing here is `Unsafe`.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::descriptor_cell::{CellPlace, CellValue, DescriptorCells, WitnessGuard};
use rule_38_descriptor_cells::cells::Key;
use rule_38_descriptor_cells::{Equality, Record, Refusal, View};

use super::retained_by_route::{HolderSpelling, RetainedSpelling};
use super::{AbstractState, LegacyDiagnostic, LegacyDiagnosticKind};
use crate::retained_by::HeldObject;

/// The view a slot cell's `int` is keyed under.
const SLOT_VIEW: &str = "int";

/// What a cell key names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CellKey {
    /// A member path (members, and `[K]` steps) of an object a record keys
    /// on, viewed as `struct TAG` (`int` for a slot's pointee).
    Object(HeldObject, String, Vec<String>),
    /// A local `int` of this frame (`&L` at a slot).
    Local(String),
}

impl CellKey {
    fn site(&self) -> Option<usize> {
        match self {
            Self::Object(object, ..) => object.site(),
            Self::Local(_) => None,
        }
    }

    /// The package's key: the spelling (unique per cell), the tracked
    /// identity the object is rooted at, and whether it is a local.
    fn table(&self) -> Key {
        Key {
            name: match self {
                Self::Object(object, view, _) => {
                    format!("{} [{object:?} as {view}]", self.spelling())
                }
                Self::Local(name) => name.clone(),
            },
            site: self.site(),
            local: matches!(self, Self::Local(_)),
        }
    }

    fn spelling(&self) -> String {
        match self {
            Self::Object(object, view, path) => {
                let root = match object {
                    HeldObject::Name(name) => name.clone(),
                    HeldObject::Site(_) | HeldObject::SiteInterior(..) => {
                        "an object this body tracks".to_owned()
                    }
                };
                format!("({root} as {view}).{}", path.join("."))
            }
            Self::Local(name) => name.clone(),
        }
    }
}

/// A cell's actual, as spelled.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CellSpelling {
    /// The object the root names, viewed as `struct TAG`, then the members
    /// (and `[K]` steps).
    Object(RetainedSpelling, String, Vec<String>),
    /// `&L`, a local `int`.
    Local(String),
    /// No spelling a cell can be keyed on.
    Unresolved(String),
}

impl CellSpelling {
    fn text(&self) -> String {
        match self {
            Self::Object(root, view, path) => {
                format!("({root:?} as {view}).{}", path.join("."))
            }
            Self::Local(name) => format!("&{name}"),
            Self::Unresolved(text) => text.clone(),
        }
    }
}

/// The guard's operands.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GuardOperands {
    pub member: CellSpelling,
    pub against: CellSpelling,
    /// The integer actuals released under the guard.
    pub releases: Vec<CellSpelling>,
    /// The removals the guard governs, as rule 12t spells them.
    pub removals: Vec<(Option<RetainedSpelling>, Option<HolderSpelling>)>,
}

/// The cell operands of one call.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CellOperands {
    /// The call's first identity; the `i`th creation is `identity + i`.
    pub identity: u64,
    /// `#fresh(K)` states, with whether a witness decides them.
    pub creations: Vec<(CellSpelling, bool)>,
    /// `#same(S)` states: the cell and the slot's cell.
    pub aliases: Vec<(CellSpelling, CellSpelling)>,
    /// Unguarded `#stale(K)` states.
    pub releases: Vec<CellSpelling>,
    /// Unguarded `{#stale(K), #kept}` states.
    pub maybe_released: Vec<CellSpelling>,
    /// The creation's class (the guard of its `#fresh` states).
    pub witness: Option<WitnessGuard>,
    pub guard: Option<GuardOperands>,
    /// Set when rule 38 is switched off or a member cell's view does not
    /// match Clang's type of the actual: the call refuses.
    pub refused: Option<String>,
    /// Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): each root name of a
    /// cell actual declared `struct TAG *` for a record with a declared cell
    /// protocol, whether it is an unrebound formal, and the protocol.
    /// Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protocols: Vec<(String, bool, ProtocolSpec)>,
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn merged(words: &[&str]) -> Vec<String> {
    let mut out = Vec::with_capacity(words.len());
    let mut at = 0;
    while at < words.len() {
        if words[at] == "-" && words.get(at + 1) == Some(&">") {
            out.push("->".to_owned());
            at += 2;
            continue;
        }
        out.push(words[at].to_owned());
        at += 1;
    }
    out
}

/// `X (-> f | . f | [ K ])+` (an integer place) as its root and steps.
fn place_steps(words: &[String]) -> Option<(String, Vec<String>)> {
    let (root, mut rest) = words.split_first()?;
    if !is_identifier(root) {
        return None;
    }
    let mut steps = Vec::new();
    while !rest.is_empty() {
        match rest {
            [access, name, tail @ ..]
                if (access == "->" || access == ".") && is_identifier(name) =>
            {
                steps.push(name.clone());
                rest = tail;
            }
            [open, digits, close, tail @ ..]
                if open == "["
                    && close == "]"
                    && digits.bytes().all(|byte| byte.is_ascii_digit())
                    && !steps.is_empty() =>
            {
                let last = steps.pop()?;
                steps.push(format!("{last}[{digits}]"));
                rest = tail;
            }
            _ => return None,
        }
    }
    (!steps.is_empty()).then(|| (root.clone(), steps))
}

/// Card MK-EVENT-FIRST-MEMBER-CELL: a pointer-valued place `X (-> f | . f)*`
/// read as its initial member, `& X ... -> member`: the words the direct
/// reading of `&X->member` is given, so both readings key one cell. `None`
/// for any other spelling (a call, a cast, an index, an operator).
fn initial_member_words(words: &[String], member: &str) -> Option<Vec<String>> {
    let (root, mut rest) = words.split_first()?;
    if !is_identifier(root) {
        return None;
    }
    while !rest.is_empty() {
        match rest {
            [access, name, tail @ ..]
                if (access == "->" || access == ".") && is_identifier(name) =>
            {
                rest = tail;
            }
            _ => return None,
        }
    }
    let mut out = Vec::with_capacity(words.len() + 3);
    out.push("&".to_owned());
    out.extend(words.iter().cloned());
    out.push("->".to_owned());
    out.push(member.to_owned());
    Some(out)
}

/// Card MK-EVENT-FIRST-MEMBER-CELL: what the analysed unit holds for
/// `struct name` (exactly one complete definition and its first member, or
/// undecided).
fn record_first_member(name: &str) -> Record {
    crate::typed_access_path::with_indirect_call_unit(|unit| {
        match unit.records().first_members.get(name).map(Vec::as_slice) {
            Some([first]) => Record::Struct(first.clone()),
            _ => Record::Undecided,
        }
    })
    .unwrap_or(Record::Undecided)
}

/// Hook (rule 38): the operands of one call of `declaration`. `actual(i)`
/// is the `i`th actual as spelled; `root(words)` reads a member cell's
/// object actual as rule 12t reads a retained pointer; `formal(name)` says
/// whether `name` is an unrebound formal of the calling function;
/// `pointee(i)` is Clang's type of the `i`th actual of every call of the
/// callee in the calling function, when they all agree.
#[allow(clippy::too_many_arguments)]
pub(super) fn operands(
    declaration: &DescriptorCells,
    removals: Vec<(Option<RetainedSpelling>, Option<HolderSpelling>)>,
    identity: u64,
    actual: &dyn Fn(usize) -> Option<Vec<String>>,
    root: &dyn Fn(&[&str]) -> Option<RetainedSpelling>,
    formal: &dyn Fn(&str) -> bool,
    pointee: &dyn Fn(usize) -> Option<String>,
) -> CellOperands {
    let mut refused = None;
    if !crate::gateway_rules::enabled(rule_38_descriptor_cells::ID) {
        refused = Some(format!(
            "descriptor cells are not read: rule {} is switched off ({})",
            rule_38_descriptor_cells::ID,
            Refusal::Undecided.key()
        ));
    }
    let mut cell = |place: &CellPlace| -> CellSpelling {
        let words = actual(place.parameter()).unwrap_or_default();
        let text = words.join(" ");
        let words = merged(&words.iter().map(String::as_str).collect::<Vec<_>>());
        match place {
            CellPlace::Member { parameter, member } => {
                // The object is read through a pointer to `struct TAG`: every
                // actual of the callee here must be one (Clang's type), or,
                // card MK-EVENT-FIRST-MEMBER-CELL, a pointer to a record whose
                // first member is the `struct TAG`, read as that member
                // (`X` as `&X->m`, the direct reading's own key).
                let view = rule_38_descriptor_cells::initial_member::member_view(
                    pointee(*parameter).as_deref(),
                    &member.tag,
                    &record_first_member,
                );
                let (words, why) = match &view {
                    View::Direct => (Some(words), ""),
                    View::InitialMember(first) => (
                        initial_member_words(&words, first),
                        "not a place its member can be taken of",
                    ),
                    View::Refused(why) => (None, *why),
                };
                let Some(words) = words else {
                    refused.get_or_insert_with(|| {
                        format!(
                            "the actual `{text}` is not a `struct {tag} *`, nor a pointer to a record whose first member is a `struct {tag}`, in every call here: {why} ({})",
                            Refusal::Undecided.key(),
                            tag = member.tag,
                        )
                    });
                    return CellSpelling::Unresolved(text);
                };
                let path = member
                    .path
                    .split('.')
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                let words = words.iter().map(String::as_str).collect::<Vec<_>>();
                match root(&words) {
                    Some(RetainedSpelling::Literal) | None => CellSpelling::Unresolved(text),
                    Some(spelled) => CellSpelling::Object(spelled, member.tag.clone(), path),
                }
            }
            CellPlace::Slot { .. } => match words.as_slice() {
                [amp, name] if amp == "&" && is_identifier(name) && !formal(name) => {
                    CellSpelling::Local(name.clone())
                }
                [amp, rest @ ..] if amp == "&" => match place_steps(rest) {
                    Some((name, steps)) => CellSpelling::Object(
                        if formal(&name) {
                            RetainedSpelling::Formal(name)
                        } else {
                            RetainedSpelling::Name(name)
                        },
                        SLOT_VIEW.to_owned(),
                        steps,
                    ),
                    None => CellSpelling::Unresolved(text),
                },
                _ => CellSpelling::Unresolved(text),
            },
        }
    };
    let mut out = CellOperands {
        identity,
        creations: Vec::new(),
        aliases: Vec::new(),
        releases: Vec::new(),
        maybe_released: Vec::new(),
        witness: None,
        guard: None,
        refused: None,
        protocols: Vec::new(),
    };
    for state in &declaration.states {
        let spelled = cell(&state.place);
        match state.value {
            CellValue::Fresh => {
                out.witness = state.guard;
                out.creations.push((spelled, state.guard.is_some()));
            }
            CellValue::Same { slot } => out
                .aliases
                .push((spelled, cell(&CellPlace::Slot { parameter: slot }))),
            CellValue::Stale if state.guard.is_none() => out.releases.push(spelled),
            CellValue::StaleOrKept if state.guard.is_none() => out.maybe_released.push(spelled),
            // A guarded stale state is the creation's other class: the
            // witness reads it ([`refine`]).
            CellValue::Stale | CellValue::StaleOrKept => {}
        }
    }
    if let Some(guard) = &declaration.guard {
        let integer = |index: usize| -> CellSpelling {
            let words = actual(index).unwrap_or_default();
            let text = words.join(" ");
            let words = merged(&words.iter().map(String::as_str).collect::<Vec<_>>());
            match place_steps(&words) {
                Some((name, steps)) => CellSpelling::Object(
                    if formal(&name) {
                        RetainedSpelling::Formal(name)
                    } else {
                        RetainedSpelling::Name(name)
                    },
                    SLOT_VIEW.to_owned(),
                    steps,
                ),
                None => CellSpelling::Unresolved(text),
            }
        };
        let member = cell(&CellPlace::Member {
            parameter: guard.parameter,
            member: guard.member.clone(),
        });
        out.guard = Some(GuardOperands {
            member,
            against: integer(guard.against),
            releases: declaration
                .guarded_releases
                .iter()
                .map(|index| integer(*index))
                .collect(),
            removals: if declaration.guarded_removal {
                removals
            } else {
                Vec::new()
            },
        });
    }
    out.refused = refused;
    out
}

/// A cell spelling's key on this path, `None` when it names no object a
/// cell keys on.
fn key(state: &AbstractState, spelled: &CellSpelling) -> Option<Key> {
    cell_key(state, spelled).map(|key| key.table())
}

fn cell_key(state: &AbstractState, spelled: &CellSpelling) -> Option<CellKey> {
    match spelled {
        CellSpelling::Local(name) => {
            (!state.pointers.contains_key(name)).then(|| CellKey::Local(name.clone()))
        }
        CellSpelling::Object(root, view, path) => {
            let object = match super::retained_by_route::retained_class(state, root) {
                crate::retained_by::Retained::CallerObject(name) => HeldObject::Name(name),
                crate::retained_by::Retained::Tracked(site) => HeldObject::Site(site),
                crate::retained_by::Retained::TrackedInterior(site, fields) => {
                    HeldObject::SiteInterior(site, fields)
                }
                crate::retained_by::Retained::Static | crate::retained_by::Retained::Other => {
                    return None;
                }
            };
            Some(CellKey::Object(object, view.clone(), path.clone()))
        }
        CellSpelling::Unresolved(_) => None,
    }
}

fn refuse(
    function: &str,
    line: usize,
    callee: &str,
    what: &str,
    refusal: Refusal,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!(
                "call to `{callee}`: descriptor cell `{what}` is not decided ({})",
                refusal.key()
            ),
        },
    });
    state.unmodeled = true;
}

/// Release through `spelled`, refusing what the table cannot decide.
fn release(
    function: &str,
    line: usize,
    callee: &str,
    spelled: &CellSpelling,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    // A plain name bound to a tracked descriptor is released through no cell
    // this table holds: refused.
    if let CellSpelling::Object(
        RetainedSpelling::Name(name) | RetainedSpelling::Formal(name),
        _,
        path,
    ) = spelled
        && path.is_empty()
        && state.pointers.contains_key(name)
    {
        refuse(
            function,
            line,
            callee,
            name,
            Refusal::Undecided,
            state,
            diagnostics,
        );
        return;
    }
    let key = key(state, spelled);
    let identity = key
        .as_ref()
        .and_then(|key| state.cells.get(key))
        .map(|(identity, _)| identity);
    let outcome = rule_38_descriptor_cells::cells::release(&mut state.cells, key.as_ref());
    // Rule 38c: the identity's pending names a protocol test introduced are
    // stale too.
    if let (Ok(rule_38_descriptor_cells::Released::Identity), Some(identity)) = (&outcome, identity)
    {
        for hosted in state.cell_protocols.values_mut() {
            hosted.fact.stale_identity(identity);
        }
    }
    if let Err(refusal) = outcome {
        refuse(
            function,
            line,
            callee,
            &spelled.text(),
            refusal,
            state,
            diagnostics,
        );
    }
}

/// Hook (rule 38): the call's cell effects on this path.
pub(super) fn apply(
    function: &str,
    line: usize,
    callee: &str,
    operands: &CellOperands,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    if let Some(reason) = &operands.refused {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: format!("call to `{callee}`: {reason}"),
            },
        });
        state.unmodeled = true;
        return;
    }
    // Rule 38c: a record with a declared protocol keeps a fact on this path;
    // each cell this call reads is adopted from it first.
    if !operands.protocols.is_empty() {
        for (root, formal, spec) in &operands.protocols {
            if let Some(root) = protocol_root(state, root, *formal) {
                state.cell_protocols.entry(root).or_insert_with(|| {
                    HostedFact::new(rule_38_descriptor_cells::protocol::Fact::reached(
                        &spec.package(),
                    ))
                });
            }
        }
        let mut spelled_cells: Vec<&CellSpelling> = Vec::new();
        spelled_cells.extend(operands.creations.iter().map(|(spelled, _)| spelled));
        for (spelled, slot) in &operands.aliases {
            spelled_cells.push(spelled);
            spelled_cells.push(slot);
        }
        spelled_cells.extend(&operands.releases);
        spelled_cells.extend(&operands.maybe_released);
        if let Some(guard) = &operands.guard {
            spelled_cells.push(&guard.member);
            spelled_cells.push(&guard.against);
            spelled_cells.extend(&guard.releases);
        }
        for spelled in spelled_cells {
            if let Some(cell) = cell_key(state, spelled) {
                adopt(state, &cell);
            }
        }
        // A creation into a protocol cell overwrites whatever the record
        // held: admitted only where the path knows no cell of it is live
        // (the guard said dead) or the cell itself is recorded.
        for (spelled, _) in &operands.creations {
            let Some(cell) = cell_key(state, spelled) else {
                continue;
            };
            let Some((root, path)) = cell_root_path(&cell) else {
                continue;
            };
            let Some(hosted) = state.cell_protocols.get(&root) else {
                continue;
            };
            let unknown = hosted.fact.protocol.is_cell(&path)
                && state.cells.get(&cell.table()).is_none()
                && hosted.fact.guard != Some(rule_38_descriptor_cells::protocol::GuardFact::Dead);
            if unknown {
                refuse(
                    function,
                    line,
                    callee,
                    &spelled.text(),
                    Refusal::Undecided,
                    state,
                    diagnostics,
                );
            }
        }
    }
    for spelled in &operands.releases {
        release(function, line, callee, spelled, state, diagnostics);
    }
    for spelled in &operands.maybe_released {
        let key = key(state, spelled);
        let recorded = key.as_ref().is_none_or(|key| {
            state
                .cells
                .escaping()
                .any(|recorded| recorded == key || !key.distinct(recorded))
        });
        if recorded && !state.cells.is_empty() {
            refuse(
                function,
                line,
                callee,
                &spelled.text(),
                Refusal::Undecided,
                state,
                diagnostics,
            );
        }
    }
    let mut slots: BTreeMap<String, Key> = BTreeMap::new();
    for (offset, (spelled, witnessed)) in operands.creations.iter().enumerate() {
        let key = key(state, spelled);
        let identity = operands.identity + offset as u64;
        match state.cells.create(key.as_ref(), identity, *witnessed) {
            Ok(()) => {
                if let Some(key) = key {
                    slots.insert(spelled.text(), key);
                }
            }
            Err(refusal) => {
                refuse(
                    function,
                    line,
                    callee,
                    &spelled.text(),
                    refusal,
                    state,
                    diagnostics,
                );
            }
        }
    }
    for (spelled, slot) in &operands.aliases {
        let key = key(state, spelled);
        let outcome = match slots.get(&slot.text()) {
            Some(of) => state.cells.alias(key.as_ref(), of),
            None => Err(Refusal::Undecided),
        };
        if let Err(refusal) = outcome {
            refuse(
                function,
                line,
                callee,
                &spelled.text(),
                refusal,
                state,
                diagnostics,
            );
        }
    }
    if let Some(guard) = &operands.guard {
        let member = key(state, &guard.member);
        let against = key(state, &guard.against);
        match rule_38_descriptor_cells::cells::decide_equality(
            &state.cells,
            member.as_ref(),
            against.as_ref(),
        ) {
            Equality::Equal => {
                for spelled in &guard.releases {
                    release(function, line, callee, spelled, state, diagnostics);
                }
                for (retained, holder) in &guard.removals {
                    super::retained_by_route::apply_removal(
                        state,
                        retained.as_ref(),
                        holder.as_ref(),
                        None,
                    );
                }
            }
            Equality::Distinct => {}
            Equality::Undecided => {
                refuse(
                    function,
                    line,
                    callee,
                    &guard.member.text(),
                    Refusal::GuardUndecided,
                    state,
                    diagnostics,
                );
                // The cautious union of a removal and none: the record stays,
                // and what the removal may name is poisoned.
                for (retained, _) in &guard.removals {
                    super::retained_by_route::apply_removal(state, retained.as_ref(), None, None);
                }
            }
        }
    }
}

/// Hook (rule 38): the call's result held by a plain name witnesses its
/// creations' class. Asked after [`forget_witnesses`].
pub(super) fn record_witness(
    operands: &CellOperands,
    destination: Option<&str>,
    state: &mut AbstractState,
) {
    let (Some(guard), Some(destination)) = (operands.witness, destination) else {
        return;
    };
    if !is_identifier(destination) || operands.refused.is_some() {
        return;
    }
    let identities = (0..operands.creations.len())
        .map(|offset| operands.identity + offset as u64)
        .collect::<Vec<_>>();
    state
        .cell_witnesses
        .insert(destination.to_owned(), (identities, guard));
}

/// Hook (rule 38): a name the instruction writes witnesses nothing any more.
pub(super) fn forget_witnesses(
    instruction: &super::LegacyInstruction,
    state: &mut AbstractState,
    writes: impl Fn(&super::LegacyInstruction, &str) -> bool,
) {
    if state.cell_witnesses.is_empty() {
        return;
    }
    state
        .cell_witnesses
        .retain(|name, _| !writes(instruction, name));
}

/// What a branch edge says of a witness's value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EdgeFact {
    /// `v == 0` / `v != 0` (a null test).
    Zero(bool),
    /// `v >= 0` / `v == -1` under a descriptor's value set (a validity test).
    Valid(bool),
}

/// Hook (rule 38): refine the witness `source` on an edge. `true` when the
/// name is a witness (the edge is then read here, whatever it decided).
pub(super) fn refine(source: &str, fact: EdgeFact, state: &mut AbstractState) -> bool {
    let Some((identities, guard)) = state.cell_witnesses.get(source).cloned() else {
        return false;
    };
    let holds = match (guard, fact) {
        (WitnessGuard::Zero, EdgeFact::Zero(zero)) => Some(zero),
        (WitnessGuard::NonZero, EdgeFact::Zero(zero)) => Some(!zero),
        (WitnessGuard::NotSentinel, EdgeFact::Valid(valid)) => Some(valid),
        (WitnessGuard::Sentinel, EdgeFact::Valid(valid)) => Some(!valid),
        // A null test of a descriptor-valued result, and a validity test of a
        // status, decide nothing.
        _ => None,
    };
    if let Some(holds) = holds {
        for identity in identities {
            state.cells.witness(identity, holds);
        }
    }
    true
}

/// Join the witnesses: a witness survives only where both paths agree.
pub(super) fn join_witnesses(
    current: &mut BTreeMap<String, (Vec<u64>, WitnessGuard)>,
    incoming: &BTreeMap<String, (Vec<u64>, WitnessGuard)>,
) {
    current.retain(|name, witness| incoming.get(name) == Some(witness));
}

/// Hook (rule 38): at an exit, a cell that may still name a live descriptor
/// outlives the frame (a caller's object, an escaping object, or a local
/// that dies with the descriptor in it).
pub(super) fn report_exit(
    function: &str,
    line: usize,
    state: &AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    // Rule 38c: a record that keeps its declared protocol forwards its cells
    // under it; one that does not refuses, and its cells still escape.
    let mut forwarded = BTreeSet::new();
    for (root, hosted) in &state.cell_protocols {
        if hosted.holds(state) {
            forwarded.extend(hosted.keys.values().cloned());
        } else {
            report_broken(function, line, root, hosted, diagnostics);
        }
    }
    for key in state.cells.escaping() {
        if forwarded.contains(key) {
            continue;
        }
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: format!(
                    "the descriptor in cell `{}` may outlive this function's frame ({})",
                    key.name,
                    Refusal::Escapes.key()
                ),
            },
        });
    }
}

/// Hook (rule 38): a release of `site` while a cell of it may name a live
/// descriptor refuses. `true` when it refused.
pub(super) fn refuse_release_of_holder(
    function: &str,
    line: usize,
    source: &str,
    site: usize,
    state: &AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> bool {
    // Rule 38c: a pending name a protocol test introduced counts too.
    let held = state.cells.holds(|key| key.site == Some(site))
        || state
            .cell_protocols
            .get(&ProtocolRoot::Site(site))
            .is_some_and(|hosted| hosted.may_hold_live(state));
    if held {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: format!(
                    "`{source}` is released while a cell of it may name a live descriptor ({})",
                    Refusal::OwnerReleased.key()
                ),
            },
        });
    }
    held
}

/// Card MK-EVENT-DESCRIPTOR-QUANT: Clang's type of the `index`th actual of
/// every call of `callee` in `function`, when every call agrees (the actual
/// before its conversion to `void *`).
pub(crate) fn actual_pointee(function: &str, callee: &str, index: usize) -> Option<String> {
    crate::typed_access_path::with_indirect_call_unit(|unit| {
        let ast = unit.function(function)?;
        let mut types = BTreeSet::new();
        let mut calls = 0usize;
        visit_calls(ast, callee, index, &mut types, &mut calls);
        (calls > 0 && types.len() == 1)
            .then(|| types.into_iter().next())
            .flatten()
    })
    .flatten()
}

fn visit_calls(
    node: &serde_json::Value,
    callee: &str,
    index: usize,
    types: &mut BTreeSet<String>,
    calls: &mut usize,
) {
    use crate::typed_c_ast::{children, kind};
    if kind(node) == "CallExpr" {
        let parts = children(node);
        if parts.first().is_some_and(|head| names_callee(head, callee)) {
            *calls += 1;
            let ty = parts.get(index + 1).map_or_else(
                || "?".to_owned(),
                |argument| unconverted_type(argument).unwrap_or_else(|| "?".to_owned()),
            );
            types.insert(ty);
        }
    }
    for child in children(node) {
        visit_calls(child, callee, index, types, calls);
    }
}

fn names_callee(node: &serde_json::Value, callee: &str) -> bool {
    use crate::typed_c_ast::{children, kind};
    match kind(node) {
        "DeclRefExpr" => node["referencedDecl"]["name"].as_str() == Some(callee),
        "ImplicitCastExpr" | "ParenExpr" => children(node)
            .first()
            .is_some_and(|inner| names_callee(inner, callee)),
        _ => false,
    }
}

/// The type of an argument before its implicit conversion to `void *`.
fn unconverted_type(node: &serde_json::Value) -> Option<String> {
    use crate::typed_c_ast::{children, kind};
    let ty = |node: &serde_json::Value| {
        node["type"]["desugaredQualType"]
            .as_str()
            .or_else(|| node["type"]["qualType"].as_str())
            .map(str::to_owned)
    };
    let mut node = node;
    loop {
        let here = ty(node)?;
        let is_void = here == "void *";
        if (kind(node) == "ImplicitCastExpr" && is_void) || kind(node) == "ParenExpr" {
            node = children(node).first()?;
            continue;
        }
        return Some(here);
    }
}

// ---------------------------------------------------------------------------
// Rule 38c (card MK-EVENT-CELL-CROSS-FUNCTION, contract spec §C04.6.3,
// grammar G05e; USER DECISION 2026-11-02, verbatim: "B as recommended"):
// declared record-type cell protocols. Every hook below is the identity in an
// environment that declares no `quant cell` clause: the parser emits no step
// and no cell operand carries a protocol, so no path holds a fact.
// ---------------------------------------------------------------------------

/// A declared protocol as the parser embeds it in a step, with the
/// declarations stating it (the rows that read it depend on them).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProtocolSpec {
    pub record: String,
    /// Each cell path with the `#fresh` cell whose identity it names.
    pub cells: Vec<(String, String)>,
    pub guard: String,
    pub op: qcore_schemas::descriptor_cell::ProtocolGuard,
    pub hosts: Vec<String>,
}

impl ProtocolSpec {
    fn from_schema(
        hosts: &BTreeSet<String>,
        protocol: &qcore_schemas::descriptor_cell::CellProtocol,
    ) -> Self {
        Self {
            record: protocol.record.clone(),
            cells: protocol
                .cells
                .iter()
                .map(|cell| {
                    (
                        cell.path.join("."),
                        cell.same.as_ref().unwrap_or(&cell.path).join("."),
                    )
                })
                .collect(),
            guard: protocol.guard.join("."),
            op: protocol.op,
            hosts: hosts.iter().cloned().collect(),
        }
    }

    /// The package's reading of the protocol.
    #[must_use]
    pub fn package(&self) -> rule_38_descriptor_cells::protocol::Protocol {
        use qcore_schemas::descriptor_cell::ProtocolGuard;
        use rule_38_descriptor_cells::protocol::Guard;
        rule_38_descriptor_cells::protocol::Protocol {
            record: self.record.clone(),
            cells: self.cells.clone(),
            guard: self.guard.clone(),
            op: match self.op {
                ProtocolGuard::NotMinusOne => Guard::NotMinusOne,
                ProtocolGuard::Positive => Guard::Positive,
                ProtocolGuard::EqualOne => Guard::EqualOne,
            },
        }
    }

    /// Note that the work relies on the declarations stating it.
    fn relied(&self) {
        for host in &self.hosts {
            crate::contract_dependencies::relied(host);
        }
    }
}

/// Every record's protocol in `contracts`, keyed by record tag.
pub(crate) fn protocol_specs(
    contracts: &qcore_schemas::contract::ResourceContracts,
) -> BTreeMap<String, ProtocolSpec> {
    contracts
        .cell_protocols()
        .into_iter()
        .map(|(record, (hosts, protocol))| (record, ProtocolSpec::from_schema(&hosts, &protocol)))
        .collect()
}

/// One statement rule 38c reads.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProtocolStep {
    /// `if (r->GUARD OP V)` (or its complement): the edge the guard holds on
    /// is `live_when_true`'s.
    Test {
        root: String,
        formal: bool,
        spec: ProtocolSpec,
        live_when_true: bool,
        identity: u64,
    },
    /// `r->PATH = V;` (`value` is `None` for anything but an integer literal).
    Store {
        root: String,
        formal: bool,
        spec: ProtocolSpec,
        path: String,
        value: Option<i64>,
    },
    /// `close(r->CELL);` (decision Q3: the cell's `#stale`).
    Close {
        root: String,
        formal: bool,
        spec: ProtocolSpec,
        path: String,
    },
}

impl ProtocolStep {
    /// The same test read on a branch whose first edge is the false one.
    #[must_use]
    pub fn flipped(&self) -> Self {
        let mut out = self.clone();
        if let Self::Test { live_when_true, .. } = &mut out {
            *live_when_true = !*live_when_true;
        }
        out
    }
}

/// The object a fact is about.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProtocolRoot {
    /// A caller's object (an unrebound formal).
    Name(String),
    /// A tracked identity.
    Site(usize),
}

/// A record's fact on this path, with the table keys its cells are recorded
/// under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedFact {
    pub fact: rule_38_descriptor_cells::protocol::Fact,
    pub keys: BTreeMap<String, Key>,
}

impl HostedFact {
    fn new(fact: rule_38_descriptor_cells::protocol::Fact) -> Self {
        Self {
            fact,
            keys: BTreeMap::new(),
        }
    }

    fn status(
        &self,
        state: &AbstractState,
    ) -> impl Fn(&str) -> Option<(u64, rule_38_descriptor_cells::Status)> + use<> {
        let statuses = self
            .keys
            .iter()
            .filter_map(|(path, key)| state.cells.get(key).map(|status| (path.clone(), status)))
            .collect::<BTreeMap<_, _>>();
        move |path: &str| statuses.get(path).copied()
    }

    fn holds(&self, state: &AbstractState) -> bool {
        self.fact.holds(self.status(state))
    }

    fn may_hold_live(&self, state: &AbstractState) -> bool {
        self.fact.may_hold_live(self.status(state))
    }
}

/// The object `root` names on this path, when a fact can be about it.
fn protocol_root(state: &AbstractState, root: &str, formal: bool) -> Option<ProtocolRoot> {
    let spelled = if formal {
        RetainedSpelling::Formal(root.to_owned())
    } else {
        RetainedSpelling::Name(root.to_owned())
    };
    match super::retained_by_route::retained_class(state, &spelled) {
        crate::retained_by::Retained::CallerObject(name) => Some(ProtocolRoot::Name(name)),
        crate::retained_by::Retained::Tracked(site) => Some(ProtocolRoot::Site(site)),
        _ => None,
    }
}

/// The record and the member path from it a cell key names.
fn cell_root_path(cell: &CellKey) -> Option<(ProtocolRoot, String)> {
    let CellKey::Object(object, _, path) = cell else {
        return None;
    };
    let (root, mut full) = match object {
        HeldObject::Name(name) => {
            let mut parts = name.split('.').map(str::to_owned);
            let base = parts.next()?;
            (ProtocolRoot::Name(base), parts.collect::<Vec<_>>())
        }
        HeldObject::Site(site) => (ProtocolRoot::Site(*site), Vec::new()),
        HeldObject::SiteInterior(site, fields) => (ProtocolRoot::Site(*site), fields.clone()),
    };
    full.extend(path.iter().cloned());
    Some((root, full.join(".")))
}

/// A cell an operation reads: recorded under its record's fact, and its
/// pending identity adopted into the table.
fn adopt(state: &mut AbstractState, cell: &CellKey) {
    let Some((root, path)) = cell_root_path(cell) else {
        return;
    };
    let key = cell.table();
    let recorded = state.cells.get(&key).is_some();
    let Some(hosted) = state.cell_protocols.get_mut(&root) else {
        return;
    };
    if !hosted.fact.protocol.is_cell(&path) {
        return;
    }
    hosted.keys.insert(path.clone(), key.clone());
    let pending = hosted.fact.adopt(&path);
    if !recorded && let Some((identity, status)) = pending {
        state.cells.adopt(key, identity, status);
    }
}

fn root_text(root: &ProtocolRoot) -> String {
    match root {
        ProtocolRoot::Name(name) => name.clone(),
        ProtocolRoot::Site(_) => "an object this body tracks".to_owned(),
    }
}

fn report_broken(
    function: &str,
    line: usize,
    root: &ProtocolRoot,
    hosted: &HostedFact,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!(
                "`{}` (a `struct {}`) does not keep its declared cell protocol here ({})",
                root_text(root),
                hosted.fact.protocol.record,
                Refusal::ProtocolBroken.key()
            ),
        },
    });
}

/// Hook (rule 38c): a guard test's edge. Read only where the path holds no
/// fact of the record: the live edge introduces the caller's identities,
/// the other edge none.
pub(super) fn refine_protocol(step: &ProtocolStep, edge: usize, state: &mut AbstractState) {
    let ProtocolStep::Test {
        root,
        formal,
        spec,
        live_when_true,
        identity,
    } = step
    else {
        return;
    };
    if !crate::gateway_rules::enabled(rule_38_descriptor_cells::ID) {
        return;
    }
    let Some(root) = protocol_root(state, root, *formal) else {
        return;
    };
    if state.cell_protocols.contains_key(&root) {
        return;
    }
    let live = (edge == 0) == *live_when_true;
    state.cell_protocols.insert(
        root,
        HostedFact::new(rule_38_descriptor_cells::protocol::Fact::introduced(
            &spec.package(),
            live,
            *identity,
        )),
    );
}

fn refuse_step(
    function: &str,
    line: usize,
    what: &str,
    refusal: Refusal,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!(
                "descriptor cell `{what}` is not decided ({})",
                refusal.key()
            ),
        },
    });
    state.unmodeled = true;
}

/// Hook (rule 38c): a store to, or a close of, a protocol member, after the
/// statement's own lowering.
pub(super) fn apply_step(
    function: &str,
    line: usize,
    step: &ProtocolStep,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    if !crate::gateway_rules::enabled(rule_38_descriptor_cells::ID) {
        return;
    }
    match step {
        ProtocolStep::Test { .. } => {}
        ProtocolStep::Store {
            root,
            formal,
            spec,
            path,
            value,
        } => {
            let Some(object) = protocol_root(state, root, *formal) else {
                return;
            };
            let what = format!("{root}->{path}");
            let mut hosted = state.cell_protocols.remove(&object).unwrap_or_else(|| {
                HostedFact::new(rule_38_descriptor_cells::protocol::Fact::reached(
                    &spec.package(),
                ))
            });
            let status = hosted.status(state);
            let outcome = hosted.fact.store(path, *value, status);
            state.cell_protocols.insert(object, hosted);
            if let Err(refusal) = outcome {
                refuse_step(function, line, &what, refusal, state, diagnostics);
            }
        }
        ProtocolStep::Close {
            root,
            formal,
            spec: _,
            path,
        } => {
            // Only a caller's record: its member read needs no live-identity
            // check, and a tracked one keeps the statement's own reading.
            let Some(object @ ProtocolRoot::Name(_)) = protocol_root(state, root, *formal) else {
                return;
            };
            if !state.cell_protocols.contains_key(&object) {
                return;
            }
            let what = format!("{root}->{path}");
            let ProtocolRoot::Name(name) = &object else {
                return;
            };
            let cell = CellKey::Object(
                HeldObject::Name(name.clone()),
                SLOT_VIEW.to_owned(),
                vec![path.clone()],
            );
            adopt(state, &cell);
            let key = cell.table();
            match state.cells.get(&key) {
                Some((identity, rule_38_descriptor_cells::Status::Live))
                    if identity != rule_38_descriptor_cells::cells::JOINED =>
                {
                    let outcome =
                        rule_38_descriptor_cells::cells::release(&mut state.cells, Some(&key));
                    match outcome {
                        Ok(_) => {
                            for hosted in state.cell_protocols.values_mut() {
                                hosted.fact.stale_identity(identity);
                            }
                        }
                        Err(refusal) => {
                            refuse_step(function, line, &what, refusal, state, diagnostics);
                        }
                    }
                }
                Some((_, rule_38_descriptor_cells::Status::Stale)) => refuse_step(
                    function,
                    line,
                    &what,
                    Refusal::DoubleRelease,
                    state,
                    diagnostics,
                ),
                _ => refuse_step(
                    function,
                    line,
                    &what,
                    Refusal::Undecided,
                    state,
                    diagnostics,
                ),
            }
        }
    }
}

/// Hook (rule 38c): a call that may reach a record with a fact forwards it
/// under its protocol when it keeps it (the callee reads the record again
/// from the protocol, and so does this body after the call), and refuses
/// when it does not. A cell-bearing call is rule 38's, never this.
pub(super) fn touch(
    function: &str,
    line: usize,
    may_reach: impl Fn(&ProtocolRoot) -> bool,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    if state.cell_protocols.is_empty() {
        return;
    }
    let roots = state
        .cell_protocols
        .keys()
        .filter(|root| may_reach(root))
        .cloned()
        .collect::<Vec<_>>();
    for root in roots {
        let Some(hosted) = state.cell_protocols.get(&root) else {
            continue;
        };
        if hosted.holds(state) {
            let keys = hosted.keys.values().cloned().collect::<BTreeSet<_>>();
            state.cells.forget(|key| keys.contains(key));
            state.cell_protocols.remove(&root);
        } else {
            let hosted = hosted.clone();
            report_broken(function, line, &root, &hosted, diagnostics);
            state.unmodeled = true;
        }
    }
}

/// Join the arriving path's facts: a record that keeps its protocol on both
/// paths keeps it (untouched); otherwise the guard where both agree, else a
/// conflict, so a touched record that disagrees refuses at its exit.
pub(super) fn join_protocols(current: &mut AbstractState, incoming: &AbstractState) {
    if current.cell_protocols.is_empty() && incoming.cell_protocols.is_empty() {
        return;
    }
    let roots = current
        .cell_protocols
        .keys()
        .chain(incoming.cell_protocols.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut joined = BTreeMap::new();
    for root in roots {
        let left = current.cell_protocols.get(&root);
        let right = incoming.cell_protocols.get(&root);
        let protocol = left.or(right).map(|hosted| hosted.fact.protocol.clone());
        let Some(protocol) = protocol else {
            continue;
        };
        let untouched = || rule_38_descriptor_cells::protocol::Fact {
            protocol: protocol.clone(),
            guard: None,
            touched: false,
            entry: BTreeMap::new(),
        };
        let left_holds = left.is_none_or(|hosted| hosted.holds(current));
        let right_holds = right.is_none_or(|hosted| hosted.holds(incoming));
        let mut keys = BTreeMap::new();
        for hosted in left.into_iter().chain(right) {
            keys.extend(hosted.keys.clone());
        }
        let fact = match (left, right) {
            (Some(l), Some(r)) if l == r => l.fact.clone(),
            _ if left_holds && right_holds => untouched(),
            _ => {
                let l = left.map_or_else(untouched, |hosted| hosted.fact.clone());
                let r = right.map_or_else(untouched, |hosted| hosted.fact.clone());
                l.join(&r)
            }
        };
        joined.insert(root, HostedFact { fact, keys });
    }
    current.cell_protocols = joined;
}

// ----- Parser-side readers (pure, over the statement's words) -----

/// Words with the two-character operators the reader needs rejoined.
fn operator_words(words: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    let mut at = 0;
    while at < words.len() {
        let pair = (words[at], words.get(at + 1).copied());
        let joined = match pair {
            ("-", Some(">")) => Some("->"),
            ("!", Some("=")) => Some("!="),
            ("=", Some("=")) => Some("=="),
            ("<", Some("=")) => Some("<="),
            (">", Some("=")) => Some(">="),
            _ => None,
        };
        if let Some(joined) = joined {
            out.push(joined.to_owned());
            at += 2;
        } else {
            out.push(words[at].to_owned());
            at += 1;
        }
    }
    out
}

fn without_parentheses(mut words: &[String]) -> &[String] {
    while words.len() >= 2
        && words.first().map(String::as_str) == Some("(")
        && words.last().map(String::as_str) == Some(")")
        && matching_close(words) == Some(words.len() - 1)
    {
        words = &words[1..words.len() - 1];
    }
    words
}

fn matching_close(words: &[String]) -> Option<usize> {
    let mut depth = 0usize;
    for (index, word) in words.iter().enumerate() {
        match word.as_str() {
            "(" => depth += 1,
            ")" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// `X -> f([K])? (. g([K])?)*`: the root and the member path.
fn member_place(words: &[String]) -> Option<(String, String)> {
    let words = without_parentheses(words);
    let (root, rest) = words.split_first()?;
    if !is_identifier(root) || rest.first().map(String::as_str) != Some("->") {
        return None;
    }
    let mut rest = &rest[1..];
    let mut segments = Vec::new();
    loop {
        let (name, tail) = rest.split_first()?;
        if !is_identifier(name) {
            return None;
        }
        let mut segment = name.clone();
        rest = tail;
        if let [open, digits, close, tail @ ..] = rest
            && open == "["
            && close == "]"
            && !digits.is_empty()
            && digits.bytes().all(|byte| byte.is_ascii_digit())
            && (digits.len() == 1 || !digits.starts_with('0'))
        {
            segment = format!("{segment}[{digits}]");
            rest = tail;
        }
        segments.push(segment);
        match rest {
            [] => return Some((root.clone(), segments.join("."))),
            [dot, tail @ ..] if dot == "." => rest = tail,
            _ => return None,
        }
    }
}

/// An integer literal `K`, `-K`, possibly parenthesized.
fn integer_literal(words: &[String]) -> Option<i64> {
    match without_parentheses(words) {
        [digits] => digits.parse::<i64>().ok(),
        [minus, digits] if minus == "-" => digits.parse::<i64>().ok().map(|v| -v),
        _ => None,
    }
}

/// The protocol of `root`'s declared record, when it has one.
fn root_protocol<'a>(
    root: &str,
    record_of: &dyn Fn(&str) -> Option<String>,
    specs: &'a BTreeMap<String, ProtocolSpec>,
) -> Option<&'a ProtocolSpec> {
    specs.get(&record_of(root)?)
}

/// A branch condition rule 38c reads: exactly `r->GUARD OP V` or its
/// complement, `r` declared `struct TAG *` for a record with a protocol.
pub(crate) fn test_step(
    condition: &[&str],
    line: usize,
    record_of: &dyn Fn(&str) -> Option<String>,
    formal: &dyn Fn(&str) -> bool,
    specs: &BTreeMap<String, ProtocolSpec>,
) -> Option<ProtocolStep> {
    use qcore_schemas::descriptor_cell::ProtocolGuard;
    if specs.is_empty() {
        return None;
    }
    let words = operator_words(condition);
    let words = without_parentheses(&words);
    let comparisons = words
        .iter()
        .enumerate()
        .filter(|(_, word)| {
            matches!(
                word.as_str(),
                "!=" | "==" | ">" | "<=" | "<" | ">=" | "&&" | "||" | "&" | "|"
            )
        })
        .collect::<Vec<_>>();
    let [(at, op)] = comparisons.as_slice() else {
        return None;
    };
    let (root, path) = member_place(&words[..*at])?;
    let value = integer_literal(&words[at + 1..])?;
    let spec = root_protocol(&root, record_of, specs)?;
    if path != spec.guard {
        return None;
    }
    let live_when_true = match (spec.op, op.as_str(), value) {
        (ProtocolGuard::NotMinusOne, "!=", -1)
        | (ProtocolGuard::Positive, ">", 0)
        | (ProtocolGuard::EqualOne, "==", 1) => true,
        (ProtocolGuard::NotMinusOne, "==", -1)
        | (ProtocolGuard::Positive, "<=", 0)
        | (ProtocolGuard::EqualOne, "!=", 1) => false,
        _ => return None,
    };
    spec.relied();
    Some(ProtocolStep::Test {
        formal: formal(&root),
        root,
        spec: spec.clone(),
        live_when_true,
        identity: (1 << 62) | ((line as u64 & 0xff_ffff) << 8),
    })
}

/// A statement rule 38c reads: `r->PATH = V` (a store to the guard or a
/// cell; `V` any expression) or `close(r->CELL)` for a one-segment cell.
pub(crate) fn statement_step(
    statement: &[&str],
    record_of: &dyn Fn(&str) -> Option<String>,
    formal: &dyn Fn(&str) -> bool,
    specs: &BTreeMap<String, ProtocolSpec>,
) -> Option<ProtocolStep> {
    if specs.is_empty() {
        return None;
    }
    let mut words = operator_words(statement);
    if words.last().map(String::as_str) == Some(";") {
        words.pop();
    }
    if let [callee, open, inner @ .., close] = words.as_slice()
        && callee == "close"
        && open == "("
        && close == ")"
        && let Some((root, path)) = member_place(inner)
        && let Some(spec) = root_protocol(&root, record_of, specs)
        && spec.cells.iter().any(|(cell, _)| *cell == path)
        && !path.contains('.')
    {
        spec.relied();
        return Some(ProtocolStep::Close {
            formal: formal(&root),
            root,
            spec: spec.clone(),
            path,
        });
    }
    let at = words.iter().position(|word| word == "=")?;
    let compound = at > 0
        && matches!(
            words[at - 1].as_str(),
            "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^" | "<" | ">"
        );
    let place = if compound {
        &words[..at - 1]
    } else {
        &words[..at]
    };
    let (root, path) = member_place(place)?;
    let spec = root_protocol(&root, record_of, specs)?;
    if path != spec.guard && !spec.cells.iter().any(|(cell, _)| *cell == path) {
        return None;
    }
    let value = if compound {
        None
    } else {
        integer_literal(&words[at + 1..])
    };
    spec.relied();
    Some(ProtocolStep::Store {
        formal: formal(&root),
        root,
        spec: spec.clone(),
        path,
        value,
    })
}

/// The protocol roots of a cell-bearing call's actuals: the root name of
/// each actual (`&r->…`, `r->…`, `r`), with its record's protocol.
pub(crate) fn operand_protocols(
    actuals: &[Vec<String>],
    record_of: &dyn Fn(&str) -> Option<String>,
    formal: &dyn Fn(&str) -> bool,
    specs: &BTreeMap<String, ProtocolSpec>,
) -> Vec<(String, bool, ProtocolSpec)> {
    let mut out: Vec<(String, bool, ProtocolSpec)> = Vec::new();
    if specs.is_empty() {
        return out;
    }
    for words in actuals {
        let words = words.iter().map(String::as_str).collect::<Vec<_>>();
        let words = operator_words(&words);
        let words = without_parentheses(&words);
        let root = match words {
            [amp, root, ..] if amp == "&" => root,
            [root, ..] => root,
            [] => continue,
        };
        if !is_identifier(root) || out.iter().any(|(seen, ..)| seen == root) {
            continue;
        }
        if let Some(spec) = root_protocol(root, record_of, specs) {
            spec.relied();
            out.push((root.clone(), formal(root), spec.clone()));
        }
    }
    out
}

impl super::Parser<'_> {
    /// The record tag a name of this function is declared a pointer to.
    fn protocol_record_of(&self, name: &str) -> Option<String> {
        match self.typed_types.declared_type(name) {
            Some(crate::typed_field_access::DeclaredType::RecordPointer(tag)) => Some(tag.clone()),
            _ => None,
        }
    }

    /// Rule 38c: the guard test a branch condition is, if any.
    pub(super) fn cell_test_step(
        &self,
        condition: &[super::Token],
        line: usize,
    ) -> Option<Box<ProtocolStep>> {
        if self.cell_protocols.is_empty() {
            return None;
        }
        let words = condition
            .iter()
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>();
        test_step(
            &words,
            line,
            &|name| self.protocol_record_of(name),
            &|name| self.caller_formals.contains(name),
            &self.cell_protocols,
        )
        .map(Box::new)
    }

    /// Rule 38c: the store or close a statement is, if any.
    pub(super) fn cell_statement_step(&self, tokens: &[super::Token]) -> Option<ProtocolStep> {
        if self.cell_protocols.is_empty() {
            return None;
        }
        let words = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
        statement_step(
            &words,
            &|name| self.protocol_record_of(name),
            &|name| self.caller_formals.contains(name),
            &self.cell_protocols,
        )
    }

    /// Rule 38c: the protocol roots of a cell-bearing call's actuals.
    pub(super) fn cell_operand_protocols(
        &self,
        actuals: &[Vec<String>],
    ) -> Vec<(String, bool, ProtocolSpec)> {
        if self.cell_protocols.is_empty() {
            return Vec::new();
        }
        operand_protocols(
            actuals,
            &|name| self.protocol_record_of(name),
            &|name| self.caller_formals.contains(name),
            &self.cell_protocols,
        )
    }
}
