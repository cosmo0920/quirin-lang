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
    /// Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): the callee's
    /// pin-derived result set, for a status-witnessed creation, while it is
    /// in force. Omitted otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_range: Option<qcore_schemas::result_range::ResultRangeFact>,
    /// Card CELL-RESULT-SET-ALIGNMENT (rule 38e as amended): per creation,
    /// the callee's pin-derived written set of its slot while in force.
    /// Omitted when none is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub written_sets: Vec<Option<Vec<(i64, i64)>>>,
}

impl CellOperands {
    /// Card CELL-PROTOCOL-PER-END-GUARDS: every cell the call spells.
    fn spelled(&self) -> Vec<&CellSpelling> {
        let mut out: Vec<&CellSpelling> = Vec::new();
        out.extend(self.creations.iter().map(|(spelled, _)| spelled));
        for (spelled, slot) in &self.aliases {
            out.push(spelled);
            out.push(slot);
        }
        out.extend(&self.releases);
        out.extend(&self.maybe_released);
        if let Some(guard) = &self.guard {
            out.push(&guard.member);
            out.push(&guard.against);
            out.extend(&guard.releases);
        }
        out
    }
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
        result_range: None,
        written_sets: Vec::new(),
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
    // each cell this call reads is adopted from it first. Card
    // CELL-PROTOCOL-PER-END-GUARDS (rule 38f): one fact per guard, and a
    // per-end sentinel gains one only through a cell of its own the call
    // spells (a cell-bearing call's effect is its declared states).
    if !operands.protocols.is_empty() {
        let named = operands
            .spelled()
            .into_iter()
            .filter_map(|spelled| cell_key(state, spelled))
            .filter_map(|cell| cell_root_path(&cell))
            .collect::<Vec<_>>();
        for (root, formal, spec) in &operands.protocols {
            if let Some(root) = protocol_root(state, root, *formal) {
                let package = spec.package();
                if package.per_end()
                    && !named
                        .iter()
                        .any(|(at, path)| *at == root && package.is_cell(path))
                {
                    continue;
                }
                state
                    .cell_protocols
                    .entry((root, spec.guard.clone()))
                    .or_insert_with(|| {
                        HostedFact::new(rule_38_descriptor_cells::protocol::Fact::reached(&package))
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
        for (index, (spelled, _)) in operands.creations.iter().enumerate() {
            let Some(cell) = cell_key(state, spelled) else {
                continue;
            };
            let Some((root, path)) = cell_root_path(&cell) else {
                continue;
            };
            let Some(hosted) = state
                .cell_protocols
                .iter()
                .find(|((at, _), hosted)| *at == root && hosted.fact.protocol.is_cell(&path))
                .map(|(_, hosted)| hosted)
            else {
                continue;
            };
            // Card CELL-RESULT-SET-ALIGNMENT (USER DECISION 2026-11-04, Q3
            // "(α) live_set suffix"): a creation into a cell with a declared
            // live set is admitted only where the creator's derived written
            // set of that slot is in force and lies inside it.
            if let Some(live) = &hosted.fact.protocol.live_set {
                let proved = operands
                    .written_sets
                    .get(index)
                    .and_then(Option::as_ref)
                    .is_some_and(|written| {
                        rule_38_descriptor_cells::protocol::subset(written, live)
                    });
                if !proved {
                    refuse(
                        function,
                        line,
                        callee,
                        &spelled.text(),
                        Refusal::LiveSetUnproved,
                        state,
                        diagnostics,
                    );
                    continue;
                }
            }
            let unknown = state.cells.get(&cell.table()).is_none()
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

/// A witness: the identities it decides, the creation's class, and (card
/// CELL-GUARD-RESULT-RANGE-CHECKED) the callee's result set in force.
pub(super) type CellWitness = (
    Vec<u64>,
    WitnessGuard,
    Option<qcore_schemas::result_range::ResultRangeFact>,
);

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
    state.cell_witnesses.insert(
        destination.to_owned(),
        (identities, guard, operands.result_range.clone()),
    );
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
    /// Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): `v OP K` holds
    /// (`true`) or fails on this edge, read through the witness's result set.
    Compared(qcore_core::result_set::Cmp, i128, bool),
}

/// Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): the class a comparison
/// edge reads of a status witness's creation, through its callee's derived
/// result set; `None` without a set in force (the comparison then reads only
/// what it reads on every `int`, which for a comparison other than a null
/// test is nothing) or when the test does not classify the set exactly.
fn compared_class(
    guard: WitnessGuard,
    range: Option<&qcore_schemas::result_range::ResultRangeFact>,
    cmp: qcore_core::result_set::Cmp,
    literal: i128,
    holds: bool,
) -> Option<bool> {
    use qcore_core::result_set::{ResultSet, comparison_set, edge, test_class};
    let range = range?;
    let universe = (i128::from(range.universe.0), i128::from(range.universe.1));
    let within = ResultSet::from_intervals(
        universe,
        &range
            .intervals
            .iter()
            .map(|(a, b)| (i128::from(*a), i128::from(*b)))
            .collect::<Vec<_>>(),
    );
    let class = match guard {
        WitnessGuard::Zero => ResultSet::interval(universe, 0, 0),
        WitnessGuard::NonZero => ResultSet::interval(universe, 0, 0).complement(),
        WitnessGuard::NotSentinel => ResultSet::interval(universe, -1, -1).complement(),
        WitnessGuard::Sentinel => ResultSet::interval(universe, -1, -1),
    };
    let test = comparison_set(universe, cmp, literal);
    edge(test_class(&within, &test, &class), holds)
}

/// Hook (rule 38): refine the witness `source` on an edge. `true` when the
/// name is a witness (the edge is then read here, whatever it decided).
pub(super) fn refine(source: &str, fact: EdgeFact, state: &mut AbstractState) -> bool {
    let Some((identities, guard, range)) = state.cell_witnesses.get(source).cloned() else {
        return false;
    };
    let holds = match (guard, fact) {
        // Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): only a set in
        // force decides, and only an exact test of it.
        (_, EdgeFact::Compared(cmp, literal, holds)) => {
            compared_class(guard, range.as_ref(), cmp, literal, holds)
        }
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
    current: &mut BTreeMap<String, CellWitness>,
    incoming: &BTreeMap<String, CellWitness>,
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
    for ((root, _), hosted) in &state.cell_protocols {
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
    // Rule 38c: a pending name a protocol test introduced counts too. Card
    // CELL-PROTOCOL-PER-END-GUARDS (rule 38f): the holder's release ends its
    // protocols only where no cell may be live, and a cell a `> 0` dead edge
    // left undecided names the zero reading.
    let protocols = state
        .cell_protocols
        .iter()
        .filter(|((root, _), _)| *root == ProtocolRoot::Site(site))
        .filter_map(|(_, hosted)| hosted.release_holder(state).err())
        .collect::<Vec<_>>();
    let held = state.cells.holds(|key| key.site == Some(site)) || !protocols.is_empty();
    if held {
        let refusal = if protocols.contains(&Refusal::ZeroUndecided) {
            Refusal::ZeroUndecided
        } else if protocols.contains(&Refusal::EdgeUndecided) {
            Refusal::EdgeUndecided
        } else {
            Refusal::OwnerReleased
        };
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: released_description(source, refusal),
            },
        });
    }
    held
}

/// The text of a holder release that refuses.
fn released_description(source: &str, refusal: Refusal) -> String {
    if refusal == Refusal::ZeroUndecided {
        format!(
            "`{source}` is released while a cell of it may hold descriptor 0, which a `> 0` test read as absent ({})",
            refusal.key()
        )
    } else if refusal == Refusal::EdgeUndecided {
        format!(
            "`{source}` is released while a cell of it may hold a live value a test's edge left undecided ({})",
            refusal.key()
        )
    } else {
        format!(
            "`{source}` is released while a cell of it may name a live descriptor ({})",
            refusal.key()
        )
    }
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
// grammar G05i; USER DECISION 2026-11-02, verbatim: "B as recommended"):
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
    /// Card CELL-RESULT-SET-ALIGNMENT: the declared live and absent sets
    /// (omitted when not declared, so a step serializes as before).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_set: Option<Vec<(i64, i64)>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absent_set: Option<Vec<(i64, i64)>>,
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
            live_set: protocol.live_set.clone(),
            absent_set: protocol.absent_set.clone(),
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
            live_set: self.live_set.clone(),
            absent_set: self.absent_set.clone(),
        }
    }

    /// Note that the work relies on the declarations stating it.
    fn relied(&self) {
        for host in &self.hosts {
            crate::contract_dependencies::relied(host);
        }
    }
}

/// Every record's protocols in `contracts`, keyed by record tag: one per
/// guard (card CELL-PROTOCOL-PER-END-GUARDS, rule 38f).
pub(crate) fn protocol_specs(
    contracts: &qcore_schemas::contract::ResourceContracts,
) -> BTreeMap<String, Vec<ProtocolSpec>> {
    contracts
        .cell_protocols()
        .into_iter()
        .map(|(record, (hosts, protocols))| {
            (
                record,
                protocols
                    .iter()
                    .map(|protocol| ProtocolSpec::from_schema(&hosts, protocol))
                    .collect(),
            )
        })
        .collect()
}

/// The key a record's fact is held under on a path: the object and the
/// guard (card CELL-PROTOCOL-PER-END-GUARDS, rule 38f: one fact per guard).
pub type FactKey = (ProtocolRoot, String);

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
        /// Card CELL-PROTOCOL-CLOSE-ACTUAL (rule 38d): the access gate admitted
        /// the actual for this close, so the close is the statement's whole
        /// custody effect and must be read: where rule 38c reads nothing the
        /// gate's refusal stands, and where it does not decide it refuses.
        /// Omitted when false, so a step serializes as before.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        admitted: bool,
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

    fn release_holder(&self, state: &AbstractState) -> Result<(), Refusal> {
        self.fact.release_holder(self.status(state))
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
    let Some(hosted) = state
        .cell_protocols
        .iter_mut()
        .find(|((at, _), hosted)| *at == root && hosted.fact.protocol.is_cell(&path))
        .map(|(_, hosted)| hosted)
    else {
        return;
    };
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
    let key = (root, spec.guard.clone());
    if state.cell_protocols.contains_key(&key) {
        return;
    }
    let live = (edge == 0) == *live_when_true;
    state.cell_protocols.insert(
        key,
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
        // Rule 38d admits nothing with rule 38 off; an admitted close is
        // never read as a silent release.
        if let ProtocolStep::Close { admitted: true, .. } = step {
            refuse_at_gate(function, line, state, diagnostics);
        }
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
            let object = (object, spec.guard.clone());
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
            spec,
            path,
            admitted,
        } => {
            use rule_38_descriptor_cells::close_actual::{Cell, Reading, reading};
            // Only a caller's record with a fact: its member read needs no
            // live-identity check, and a tracked one keeps the statement's own
            // reading.
            let name = match protocol_root(state, root, *formal) {
                Some(ProtocolRoot::Name(name))
                    if state
                        .cell_protocols
                        .contains_key(&(ProtocolRoot::Name(name.clone()), spec.guard.clone())) =>
                {
                    Some(name)
                }
                _ => None,
            };
            let what = format!("{root}->{path}");
            let Some(name) = name else {
                // Rule 38d: an admitted close rule 38c does not read keeps the
                // gate's refusal; any other close is the statement's own.
                if *admitted {
                    refuse_at_gate(function, line, state, diagnostics);
                }
                return;
            };
            let cell = CellKey::Object(
                HeldObject::Name(name),
                SLOT_VIEW.to_owned(),
                vec![path.clone()],
            );
            adopt(state, &cell);
            let key = cell.table();
            let entry = state.cells.get(&key);
            match reading(true, Cell::of(entry)) {
                Reading::Stale => {
                    let identity =
                        entry.map_or(rule_38_descriptor_cells::cells::JOINED, |(i, _)| i);
                    match rule_38_descriptor_cells::cells::release(&mut state.cells, Some(&key)) {
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
                Reading::Refused(refusal) => {
                    refuse_step(function, line, &what, refusal, state, diagnostics);
                }
                Reading::Gate => refuse_at_gate(function, line, state, diagnostics),
            }
        }
    }
}

/// Rule 38d: the access gate's own refusal, where rule 38c reads nothing of
/// an admitted close (the gate admitted it only for rule 38c's reading).
fn refuse_at_gate(
    function: &str,
    line: usize,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: super::call_argument_access::GATE_REFUSAL.to_owned(),
        },
    });
    state.unmodeled = true;
}

/// Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (rule 38g): what a call's
/// callees' frames say of a record type (`standing`), the callee a refusal
/// names, and whether the call is handed only a record's own cell numbers
/// (`cell_numbers_only`, rule 38c's close).
pub(super) struct CallFrames<'a, S, C> {
    pub callee: &'a str,
    pub standing: S,
    pub cell_numbers_only: C,
}

/// Hook (rule 38c): a call that may reach a record with a fact forwards it
/// under its protocol when it keeps it (the callee reads the record again
/// from the protocol, and so does this body after the call), and refuses
/// when it does not. A cell-bearing call is rule 38's, never this.
///
/// Card CELL-PROTOCOL-PER-END-GUARDS (rule 38f): a call that releases the
/// record (`releases`) ends its protocols instead. Where a cell may still be
/// live a caller's record refuses (`descriptor-cell-zero-undecided` when a
/// `> 0` dead edge left it undecided, else `descriptor-cell-owner-released`;
/// a tracked one refused at its release already); either way the facts end.
///
/// Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (rule 38g; USER DECISION
/// 2026-11-04, verbatim: "(a) Callee frames"): where a record does not keep
/// its protocol, `frames` (the callee and its frame's standing over a record
/// type) decides instead of the refusal: a checked or accepted frame passes
/// the fact through the call unchanged, a declared one passes it and holds
/// the call for review (`Pending`), a refuted one refuses
/// (`descriptor-cell-frame-refuted`), and none refuses as before.
pub(super) fn touch(
    function: &str,
    line: usize,
    may_reach: impl Fn(&ProtocolRoot) -> bool,
    releases: impl Fn(&ProtocolRoot) -> bool,
    frames: CallFrames<
        '_,
        impl Fn(&str) -> rule_38_descriptor_cells::frame::Standing,
        impl Fn(&ProtocolRoot, &str) -> bool,
    >,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    if state.cell_protocols.is_empty() {
        return;
    }
    // Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (rule 38g): a record that
    // does not keep its protocol is in its release window, where a call
    // reaches it through any pointer it is handed (`container_of`, a back
    // pointer), not only through an actual naming it: option (b)'s narrowing
    // is not read, so only the callee's frame admits the call. A call handed
    // only the record's own cell numbers (rule 38c's close) is the one
    // exception.
    let keys = state
        .cell_protocols
        .iter()
        .filter(|((root, _), hosted)| {
            may_reach(root)
                || releases(root)
                || (!hosted.holds(state)
                    && !(frames.cell_numbers_only)(root, &hosted.fact.protocol.record))
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    // The holder's release: every fact of a released record ends, its cells
    // with it; one refusal per record where a cell may still be live.
    let released = keys
        .iter()
        .filter(|(root, _)| releases(root))
        .cloned()
        .collect::<Vec<_>>();
    let mut refusals: BTreeMap<ProtocolRoot, Refusal> = BTreeMap::new();
    for key in &released {
        let Some(hosted) = state.cell_protocols.get(key) else {
            continue;
        };
        if let Err(refusal) = hosted.release_holder(state) {
            let entry = refusals.entry(key.0.clone()).or_insert(refusal);
            if refusal == Refusal::ZeroUndecided
                || (refusal == Refusal::EdgeUndecided && *entry == Refusal::OwnerReleased)
            {
                *entry = refusal;
            }
        }
        let ended = hosted.keys.values().cloned().collect::<BTreeSet<_>>();
        state.cells.forget(|cell| ended.contains(cell));
        state.cell_protocols.remove(key);
    }
    for (root, refusal) in refusals {
        // A tracked holder's release refused already (`refuse_release_of_holder`).
        if let ProtocolRoot::Name(name) = &root {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: released_description(name, refusal),
                },
            });
            state.unmodeled = true;
        }
    }
    for key in keys {
        if released.contains(&key) {
            continue;
        }
        let Some(hosted) = state.cell_protocols.get(&key) else {
            continue;
        };
        if hosted.holds(state) {
            let keys = hosted.keys.values().cloned().collect::<BTreeSet<_>>();
            state.cells.forget(|key| keys.contains(key));
            state.cell_protocols.remove(&key);
            continue;
        }
        use rule_38_descriptor_cells::frame::{Reading, window};
        let hosted = hosted.clone();
        let record = hosted.fact.protocol.record.clone();
        match window((frames.standing)(&record)) {
            // The callee touches no protocol member of any `struct TAG`: the
            // fact is the same after the call.
            Reading::Pass => {}
            Reading::Held => {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::Unsupported {
                        description: super::cell_frames_route::frame_hold_description(
                            frames.callee,
                            &record,
                        ),
                    },
                });
            }
            Reading::Refused(Refusal::FrameRefuted) => {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::Unsupported {
                        description: format!(
                            "`{}` (a `struct {record}`) does not keep its declared cell protocol here, and a body this unit analyses refutes the declared frame of `{}` over it ({})",
                            root_text(&key.0),
                            frames.callee,
                            Refusal::FrameRefuted.key()
                        ),
                    },
                });
                state.unmodeled = true;
            }
            Reading::Refused(_) => {
                report_broken(function, line, &key.0, &hosted, diagnostics);
                state.unmodeled = true;
            }
        }
    }
}

/// Join the arriving path's facts and cell tables: a record that keeps its
/// protocol on both paths keeps it (untouched); otherwise the guard where both
/// agree, else a conflict, so a touched record that disagrees refuses at its
/// exit. Card CELL-PROTOCOL-PER-END-GUARDS (rule 38f): per guard; each
/// path's pending names whose cell either path recorded are put in its own
/// table first, and a join never drops a name that may be live.
pub(super) fn join_cells(current: &mut AbstractState, incoming: &AbstractState) {
    if current.cell_protocols.is_empty() && incoming.cell_protocols.is_empty() {
        if !current.cells.is_empty() || !incoming.cells.is_empty() {
            current.cells.join(&incoming.cells);
        }
        return;
    }
    let mut incoming_cells = incoming.cells.clone();
    let mut incoming_facts = incoming.cell_protocols.clone();
    let roots = current
        .cell_protocols
        .keys()
        .chain(incoming.cell_protocols.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for root in &roots {
        let mut keys = BTreeMap::new();
        for hosted in current
            .cell_protocols
            .get(root)
            .into_iter()
            .chain(incoming_facts.get(root))
        {
            keys.extend(hosted.keys.clone());
        }
        if let Some(hosted) = current.cell_protocols.get_mut(root) {
            hosted.fact.normalize(&mut current.cells, &keys);
            hosted.keys.extend(keys.clone());
        }
        if let Some(hosted) = incoming_facts.get_mut(root) {
            hosted.fact.normalize(&mut incoming_cells, &keys);
            hosted.keys.extend(keys);
        }
    }
    let mut incoming_state = AbstractState {
        cells: incoming_cells,
        ..AbstractState::default()
    };
    incoming_state.cell_protocols = incoming_facts;
    let mut joined = BTreeMap::new();
    for root in roots {
        let left = current.cell_protocols.get(&root);
        let right = incoming_state.cell_protocols.get(&root);
        let protocol = left.or(right).map(|hosted| hosted.fact.protocol.clone());
        let Some(protocol) = protocol else {
            continue;
        };
        let untouched = || rule_38_descriptor_cells::protocol::Fact {
            protocol: protocol.clone(),
            guard: None,
            touched: false,
            entry: BTreeMap::new(),
            zero: false,
        };
        let left_holds = left.is_none_or(|hosted| hosted.holds(current));
        let right_holds = right.is_none_or(|hosted| hosted.holds(&incoming_state));
        let mut keys = BTreeMap::new();
        for hosted in left.into_iter().chain(right) {
            keys.extend(hosted.keys.clone());
        }
        let fact = match (left, right) {
            (Some(l), Some(r)) if l == r => l.fact.clone(),
            _ => {
                let l = left.map_or_else(untouched, |hosted| hosted.fact.clone());
                let r = right.map_or_else(untouched, |hosted| hosted.fact.clone());
                let mut fact = l.join(&r);
                if left_holds && right_holds {
                    fact.guard = None;
                    fact.touched = false;
                }
                fact
            }
        };
        joined.insert(root, HostedFact { fact, keys });
    }
    // A protocol cell both paths left stale names no live descriptor,
    // whatever identities the paths name (the table's own join reads two
    // different stale identities as undecided).
    let protocol_keys = joined
        .values()
        .flat_map(|hosted| hosted.keys.values())
        .cloned()
        .collect::<BTreeSet<_>>();
    current.cell_protocols = joined;
    current
        .cells
        .join_keeping_stale(&incoming_state.cells, |key| protocol_keys.contains(key));
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

/// The protocols of `root`'s declared record, one per guard (card
/// CELL-PROTOCOL-PER-END-GUARDS, rule 38f), when it has any.
fn root_protocols<'a>(
    root: &str,
    record_of: &dyn Fn(&str) -> Option<String>,
    specs: &'a BTreeMap<String, Vec<ProtocolSpec>>,
) -> Option<&'a [ProtocolSpec]> {
    specs.get(&record_of(root)?).map(Vec::as_slice)
}

/// A branch condition rule 38c reads: exactly `r->GUARD OP V` or its
/// complement, `r` declared `struct TAG *` for a record with a protocol.
pub(crate) fn test_step(
    condition: &[&str],
    line: usize,
    record_of: &dyn Fn(&str) -> Option<String>,
    formal: &dyn Fn(&str) -> bool,
    specs: &BTreeMap<String, Vec<ProtocolSpec>>,
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
    let spec = root_protocols(&root, record_of, specs)?
        .iter()
        .find(|spec| spec.guard == path)?;
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
    specs: &BTreeMap<String, Vec<ProtocolSpec>>,
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
        && let Some(spec) = root_protocols(&root, record_of, specs)
            .into_iter()
            .flatten()
            .find(|spec| spec.cells.iter().any(|(cell, _)| *cell == path))
        && !path.contains('.')
    {
        spec.relied();
        return Some(ProtocolStep::Close {
            formal: formal(&root),
            root,
            spec: spec.clone(),
            path,
            admitted: false,
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
    let spec = root_protocols(&root, record_of, specs)?
        .iter()
        .find(|spec| path == spec.guard || spec.cells.iter().any(|(cell, _)| *cell == path))?;
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
    specs: &BTreeMap<String, Vec<ProtocolSpec>>,
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
        for spec in root_protocols(root, record_of, specs).into_iter().flatten() {
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
