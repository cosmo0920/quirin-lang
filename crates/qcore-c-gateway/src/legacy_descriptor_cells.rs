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
//!    `struct TAG`, checked against Clang's type of every actual of the call;
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
use rule_38_descriptor_cells::{Equality, Refusal};

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
                // actual of the callee here must be one (Clang's type), so a
                // record whose first member is the event refuses.
                let view = format!("struct {} *", member.tag);
                if pointee(*parameter).as_deref() != Some(view.as_str()) {
                    refused.get_or_insert_with(|| {
                        format!(
                            "the actual `{text}` is not a `{view}` in every call here ({})",
                            Refusal::Undecided.key()
                        )
                    });
                    return CellSpelling::Unresolved(text);
                }
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
    let outcome = rule_38_descriptor_cells::cells::release(&mut state.cells, key.as_ref());
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
    for key in state.cells.escaping() {
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
    let held = state.cells.holds(|key| key.site == Some(site));
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
