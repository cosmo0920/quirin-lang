//! Card **SLOT-MOVE-AND-INTERIOR-BORROW** (gateway rules 12x, 12y, 12z, 13a):
//! the route-G adapter of [`crate::slot_move_borrow`]. Thin hooks, each the
//! identity on every function that calls no slot-moving, derived-out or
//! borrowed-result callee:
//!
//! 1. [`Parser::slot_operands`] (inside rule 12t's `retaining_call`): reads
//!    each slot and base actual as spelled; what no state can decide refuses.
//! 2. [`apply`] (after rule 12t's `apply`): the slot move and the derived out
//!    slot against the state.
//! 3. [`before`] and [`after`] (around every transfer): a read of an
//!    invalidated name refuses; a mutation ends the borrows it may reach; a
//!    borrowed result is recorded, copied by an alias and dropped at a
//!    rebinding.
//! 4. [`settle`] (beside rule 12w's): in a unit that defines a declared
//!    member, a declaration its body does not validate is withdrawn.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{DerivedExtent, ResourceContracts, ResourceReturnMode};
use qcore_schemas::retained_by::RetainingContract;

use super::{
    AbstractPointer, AbstractState, Derivation, LegacyCfgNode, LegacyDiagnostic,
    LegacyDiagnosticKind, LegacyInstruction, Parser, Token,
};
use crate::slot_move_borrow::{
    OutSlotBefore, OutSlotOutcome, SlotBinding, SlotOutcome, decide_out, decide_slot,
};

/// A slot actual, as spelled.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SlotActual {
    /// `&L`.
    Local(String),
    /// A formal slot this body names only as whole slot actuals.
    CallerSlot(String),
    /// A null pointer constant.
    Null,
}

/// A derived out slot's base actual.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BaseActual {
    Place(String),
    Untracked,
}

/// One slot clause's operands at a call.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SlotOperand {
    Move {
        text: String,
        actual: SlotActual,
        nullable: bool,
    },
    Out {
        text: String,
        actual: SlotActual,
        base: BaseActual,
        nullable: bool,
        const_dropped: bool,
    },
}

/// Why a name's value is invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Invalid {
    /// Rule 12x: an alias of an object a slot move moved.
    SlotMoved,
    /// Rule 12y: a borrow its parent's release or mutation ended.
    BorrowEnded,
    /// Card **REFCOUNT-RESOURCE-KIND** (rule 13b): a name bound to a
    /// reference family whose possibly-final drop discharged it.
    ReferenceDropped,
    /// Card **REFCOUNT-RESOURCE-KIND** (rule 13b): a name bound to an
    /// acquired family (the object a declared reference-returning call
    /// created at `site`) whose last drop discharged it; a further drop
    /// through it is an over-drop.
    ReferenceFreed { site: usize },
}

/// One borrowed result a name holds (rule 12y): the take that produced it
/// (an alias keeps it) and the parent's root name (empty when the parent is
/// no name of this body).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Borrow {
    pub key: (usize, String),
    pub parent: String,
}

pub(super) const UNDECIDED: &str = "slot-move-undecided";
pub(super) const ALIAS: &str = "slot-move-alias";
pub(super) const ENDED: &str = "interior-borrow-ended";
pub(super) const OUT_UNDECIDED: &str = "out-derived-undecided";

/// The refusal of a slot clause the call site does not decide.
#[must_use]
pub fn undecided_description(function: &str, actual: &str, reason: &str) -> String {
    format!("call to `{function}` with slot `{actual}`: {reason}")
}

fn is_identifier(text: &str) -> bool {
    super::token_shapes::is_identifier(text)
}

fn strip<'a>(mut argument: &'a [&'a str]) -> &'a [&'a str] {
    while argument.len() >= 2
        && argument[0] == "("
        && argument[argument.len() - 1] == ")"
        && close_of(argument, 0) == Some(argument.len() - 1)
    {
        argument = &argument[1..argument.len() - 1];
    }
    argument
}

/// The index of the `)` closing the `(` at `open`.
fn close_of(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
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

fn is_null_constant(words: &[&str]) -> bool {
    let words = strip(words);
    matches!(words, ["NULL"] | ["0"] | ["nullptr"])
        || matches!(words, ["(", "void", "*", ")", rest @ ..] if is_null_constant(rest))
}

/// The callee and argument index of the whole actual at `at` in `body`, when
/// `body[at]` alone is that actual.
fn whole_actual<'a>(body: &[&'a str], at: usize) -> Option<(&'a str, usize)> {
    let previous = at.checked_sub(1).map(|p| body[p])?;
    let next = body.get(at + 1).copied()?;
    if !matches!(previous, "(" | ",") || !matches!(next, "," | ")") {
        return None;
    }
    let mut depth = 0usize;
    let mut commas = 0usize;
    for index in (0..at).rev() {
        match body[index] {
            ")" | "]" | "}" => depth += 1,
            "(" | "[" | "{" if depth > 0 => depth -= 1,
            "(" => {
                let callee = index.checked_sub(1).map(|c| body[c])?;
                return is_identifier(callee).then_some((callee, commas));
            }
            "[" | "{" => return None,
            "," if depth == 0 => commas += 1,
            _ => {}
        }
    }
    None
}

/// `words` with every `-` `>` pair (the lexer emits each punctuator byte as
/// its own token) read as the one token `->`.
fn arrows<'a>(words: &[&'a str]) -> Vec<&'a str> {
    let mut merged = Vec::with_capacity(words.len());
    let mut at = 0;
    while at < words.len() {
        if words[at] == "-" && words.get(at + 1) == Some(&">") {
            merged.push("->");
            at += 2;
        } else {
            merged.push(words[at]);
            at += 1;
        }
    }
    merged
}

/// Whether `name` is mentioned in `body` only as a whole actual of a call
/// whose callee states a slot clause of the wanted kind at that position.
fn only_slot_mentions(
    body: &[&str],
    name: &str,
    contracts: &ResourceContracts,
    wanted: impl Fn(&RetainingContract, usize) -> bool,
) -> bool {
    let body = &arrows(body)[..];
    body.iter().enumerate().all(|(at, token)| {
        *token != name
            || matches!(at.checked_sub(1).map(|p| body[p]), Some("." | "->"))
            || whole_actual(body, at).is_some_and(|(callee, index)| {
                contracts
                    .retaining_contract(callee)
                    .is_some_and(|declaration| wanted(declaration, index))
            })
    })
}

fn moves_at(declaration: &RetainingContract, index: usize) -> bool {
    declaration
        .slot_moves
        .iter()
        .any(|moved| moved.slot == index)
}

fn outs_at(declaration: &RetainingContract, index: usize) -> bool {
    declaration.derived_outs.iter().any(|out| out.slot == index)
}

impl Parser<'_> {
    /// Rules 12x, 12z: whether `argument` is `&NAME` at a slot position of the
    /// route-G callee `function` (its minted name). Such an actual is the
    /// slot's place, decided by [`apply`] against the state, not an unmodeled
    /// address.
    pub(super) fn slot_address(&self, function: &str, index: usize, argument: &[&str]) -> bool {
        crate::retained_by::retained_callee(function)
            .and_then(|original| self.contracts.retaining_contract(original))
            .is_some_and(|declaration| {
                (moves_at(declaration, index) || outs_at(declaration, index))
                    && matches!(strip(argument), ["&", name] if is_identifier(name))
            })
    }

    /// Hook 1 (rules 12x, 12z): the slot operands of a call to `function`,
    /// or the refusal's description.
    pub(super) fn slot_operands(
        &self,
        function: &str,
        declaration: &RetainingContract,
        actual: &dyn Fn(usize) -> Option<Vec<String>>,
        body: Option<&[&str]>,
    ) -> Result<Vec<SlotOperand>, String> {
        let mut operands = Vec::new();
        let slot = |index: usize,
                    wanted: &dyn Fn(&RetainingContract, usize) -> bool|
         -> Result<(SlotActual, String), String> {
            let spelled = actual(index)
                .ok_or_else(|| undecided_description(function, "?", "the actual is not read"))?;
            let words = spelled.iter().map(String::as_str).collect::<Vec<_>>();
            let text = words.join(" ");
            let words = strip(&words);
            match words {
                ["&", name] if is_identifier(name) => {
                    Ok((SlotActual::Local((*name).to_owned()), text))
                }
                [name]
                    if is_identifier(name)
                        && self.caller_formals.contains(name)
                        && body.is_some_and(|body| {
                            only_slot_mentions(body, name, self.contracts, |d, i| wanted(d, i))
                        }) =>
                {
                    Ok((SlotActual::CallerSlot((*name).to_owned()), text))
                }
                words if is_null_constant(words) => Ok((SlotActual::Null, text)),
                _ => Err(undecided_description(
                    function,
                    &text,
                    "the slot is neither `&NAME`, a caller's slot passed through, nor NULL",
                )),
            }
        };
        for moved in &declaration.slot_moves {
            let (actual, text) = slot(moved.slot, &moves_at)?;
            operands.push(SlotOperand::Move {
                text,
                actual,
                nullable: moved.nullable,
            });
        }
        for out in &declaration.derived_outs {
            let (actual_slot, text) = slot(out.slot, &outs_at)?;
            let base = match &actual_slot {
                // A caller's slot passed through, or a NULL slot, binds
                // nothing here: the base is not read.
                SlotActual::CallerSlot(_) | SlotActual::Null => BaseActual::Untracked,
                SlotActual::Local(_) => {
                    let spelled = actual(out.base).ok_or_else(|| {
                        undecided_description(function, &text, "the base is not read")
                    })?;
                    let words = spelled.iter().map(String::as_str).collect::<Vec<_>>();
                    match super::derived_base(&words) {
                        super::DerivedBase::Place { place, .. } => BaseActual::Place(place),
                        super::DerivedBase::Untracked => BaseActual::Untracked,
                        super::DerivedBase::Unmodeled => {
                            match super::token_shapes::cast_place(&words) {
                                Some(place) => BaseActual::Place(place),
                                None => {
                                    return Err(undecided_description(
                                        function,
                                        &text,
                                        "the base is not a place",
                                    ));
                                }
                            }
                        }
                    }
                }
            };
            operands.push(SlotOperand::Out {
                text,
                actual: actual_slot,
                base,
                nullable: out.nullable,
                const_dropped: out.const_dropped,
            });
        }
        Ok(operands)
    }
}

/// Whether a structure other than a name holds `site`.
fn held_elsewhere(state: &AbstractState, site: usize) -> bool {
    state
        .contained
        .iter()
        .any(|(owner, children)| *owner == site || children.contains(&site))
        || state.field_links.iter().any(|(owner, fields)| {
            *owner == site || fields.values().any(|children| children.contains(&site))
        })
        || state.published.contains(&site)
        || state
            .retained
            .iter()
            .any(|(object, _)| object.site() == Some(site))
        || state.collections.contains_key(&site)
        || state
            .conditional_replacements
            .iter()
            .any(|(new, old)| *new == site || old.contains(&site))
        || state
            .status_witnesses
            .values()
            .any(|(held, _)| *held == site)
        || state.nullable_sites.contains(&site)
        // Card REFCOUNT-RESOURCE-KIND (rule 13b): a reference family is held
        // by its count, not by a name: a slot move of it is refused.
        || state.references.contains_key(&site)
}

fn refuse(
    function: &str,
    line: usize,
    description: String,
    pointer: Option<&AbstractPointer>,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported { description },
    });
    state.unmodeled = true;
    if let Some(pointer) = pointer {
        super::mark_unresolved_effect(state, pointer);
    }
}

/// Hook 2 (rules 12x, 12z): each slot operand decided against `state`.
pub(super) fn apply(
    function: &str,
    line: usize,
    called: &str,
    operands: &[SlotOperand],
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for operand in operands {
        match operand {
            SlotOperand::Move {
                text,
                actual,
                nullable,
            } => {
                let (binding, slot, site) = match actual {
                    SlotActual::CallerSlot(_) => (SlotBinding::CallerSlot, None, None),
                    SlotActual::Null => (SlotBinding::Other, None, None),
                    SlotActual::Local(name) => match state.pointers.get(name) {
                        None => (SlotBinding::Other, Some(name), None),
                        Some(pointer)
                            if pointer.targets.is_empty()
                                && !pointer.unknown
                                && pointer.may_be_null =>
                        {
                            (SlotBinding::DefinitelyNull, Some(name), None)
                        }
                        Some(pointer)
                            if pointer.targets.len() == 1
                                && !pointer.unknown
                                && pointer.derivation.is_identity()
                                && (*nullable || !pointer.may_be_null) =>
                        {
                            let site = pointer.targets.first().copied();
                            let live = site.is_some_and(|site| {
                                state.resources.get(&site) == Some(&super::LIVE)
                                    && !state.unresolved_effect_sites.contains(&site)
                            });
                            if live {
                                (
                                    SlotBinding::OneTracked {
                                        held_elsewhere: site
                                            .is_some_and(|site| held_elsewhere(state, site)),
                                    },
                                    Some(name),
                                    site,
                                )
                            } else {
                                (SlotBinding::Other, Some(name), None)
                            }
                        }
                        Some(_) => (SlotBinding::Other, Some(name), None),
                    },
                };
                match decide_slot(binding, *nullable) {
                    SlotOutcome::Nothing => {}
                    SlotOutcome::Move => {
                        let (Some(slot), Some(site)) = (slot, site) else {
                            continue;
                        };
                        let slot = slot.clone();
                        let aliases = state
                            .pointers
                            .iter()
                            .filter(|(name, pointer)| {
                                **name != slot && pointer.targets.contains(&site)
                            })
                            .map(|(name, _)| name.clone())
                            .collect::<Vec<_>>();
                        for alias in aliases {
                            state.pointers.remove(&alias);
                            state.borrows.remove(&alias);
                            state.invalidated.insert(alias, Invalid::SlotMoved);
                        }
                    }
                    SlotOutcome::Refuse => {
                        let pointer = slot.and_then(|slot| state.pointers.get(slot)).cloned();
                        refuse(
                            function,
                            line,
                            undecided_description(
                                called,
                                text,
                                &format!(
                                    "{UNDECIDED}: the slot does not hold exactly one live \
                                     identity this body alone holds"
                                ),
                            ),
                            pointer.as_ref(),
                            state,
                            diagnostics,
                        );
                    }
                }
            }
            SlotOperand::Out {
                text,
                actual,
                base,
                nullable,
                const_dropped,
            } => {
                let before = match actual {
                    SlotActual::CallerSlot(_) => OutSlotBefore::CallerSlot,
                    SlotActual::Null => OutSlotBefore::NullSlot,
                    SlotActual::Local(name) => match state.pointers.get(name) {
                        None => OutSlotBefore::Untracked,
                        Some(pointer) if pointer.targets.is_empty() => OutSlotBefore::Untracked,
                        Some(_) => OutSlotBefore::Tracked,
                    },
                };
                match decide_out(before, *nullable) {
                    OutSlotOutcome::Nothing => {}
                    OutSlotOutcome::Bind => {
                        let SlotActual::Local(name) = actual else {
                            continue;
                        };
                        let base_binding = match base {
                            BaseActual::Place(place) => state.pointers.get(place).cloned(),
                            BaseActual::Untracked => None,
                        };
                        // The slot may keep its old value (an untracked one),
                        // so nothing on it is ever accused: the binding is
                        // read at the `Borrowed` extent, whose release and
                        // use after the base's release are refused, and is
                        // recorded as no borrow, so only the base's release
                        // ends it (§C04.3.1's rules, refused).
                        let mut derived = super::derive_pointer(
                            base_binding.as_ref(),
                            DerivedExtent::Borrowed,
                            true,
                            *const_dropped,
                        );
                        derived.may_be_null = true;
                        state.invalidated.remove(name);
                        state.borrows.remove(name);
                        state.pointers.insert(name.clone(), derived);
                    }
                    OutSlotOutcome::Refuse => {
                        let pointer = match actual {
                            SlotActual::Local(name) => state.pointers.get(name).cloned(),
                            _ => None,
                        };
                        refuse(
                            function,
                            line,
                            undecided_description(
                                called,
                                text,
                                &format!(
                                    "{OUT_UNDECIDED}: the slot may overwrite a tracked identity, \
                                     or is NULL where the declaration says `nonnull`"
                                ),
                            ),
                            pointer.as_ref(),
                            state,
                            diagnostics,
                        );
                    }
                }
            }
        }
    }
}

/// The roots of every quoted string in `text` (a `Debug` rendering).
fn quoted_roots(text: &str) -> Vec<String> {
    let mut roots = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '"' {
            continue;
        }
        let mut value = String::new();
        while let Some(inner) = chars.next() {
            match inner {
                '\\' => {
                    if let Some(escaped) = chars.next() {
                        value.push(escaped);
                    }
                }
                '"' => break,
                other => value.push(other),
            }
        }
        let value = value.trim_start_matches('*').trim_start_matches('&');
        if let Some(root) = crate::may_alias::place_root(value)
            && is_identifier(root)
        {
            roots.push(root.to_owned());
        }
    }
    roots
}

/// The names `instruction` reads: every place it mentions, less one mention
/// of each plain name it only writes and of each callee name.
fn reads(instruction: &LegacyInstruction) -> BTreeSet<String> {
    let mut roots = quoted_roots(&format!("{instruction:?}"));
    let mut drop_one = |name: &str| {
        if let Some(at) = roots.iter().position(|root| root == name) {
            roots.remove(at);
        }
    };
    for name in super::rebound_names(instruction) {
        drop_one(name);
    }
    if let LegacyInstruction::ContractCall {
        function, nested, ..
    } = instruction
    {
        drop_one(function);
        for call in nested {
            drop_one(&call.function);
        }
    }
    roots.into_iter().collect()
}

/// What an instruction may mutate, for the borrows (rule 12y).
enum Mutation {
    /// Nothing a borrow's parent could be.
    None,
    /// Exactly these roots.
    Names(BTreeSet<String>),
    /// Something this reader does not see: every borrow ends.
    Unseen,
}

fn root_of(place: &str) -> Option<String> {
    crate::may_alias::place_root(place.trim_start_matches('*')).map(str::to_owned)
}

fn is_plain(place: &str) -> bool {
    crate::may_alias::place_root(place) == Some(place)
}

fn mutation(instruction: &LegacyInstruction) -> Mutation {
    match instruction {
        LegacyInstruction::ContractCall {
            arguments, nested, ..
        } => {
            let mut names = BTreeSet::new();
            for argument in arguments
                .iter()
                .chain(nested.iter().flat_map(|call| call.arguments.iter()))
            {
                match argument.as_deref().and_then(root_of) {
                    Some(root) => {
                        names.insert(root);
                    }
                    None => return Mutation::Unseen,
                }
            }
            Mutation::Names(names)
        }
        LegacyInstruction::ScalarMemberStore { base, .. } => {
            Mutation::Names(root_of(base).into_iter().collect())
        }
        LegacyInstruction::IndirectScalarAccess { bases, .. } => {
            Mutation::Names(bases.iter().filter_map(|base| root_of(base)).collect())
        }
        LegacyInstruction::Release { source } => {
            Mutation::Names(root_of(source).into_iter().collect())
        }
        LegacyInstruction::SelfInitCall { base, .. }
        | LegacyInstruction::TrackedSelfInit { base, .. } => {
            Mutation::Names(root_of(base).into_iter().collect())
        }
        LegacyInstruction::Alias { destination, .. }
        | LegacyInstruction::Clear { destination }
        | LegacyInstruction::OpaqueValue { destination, .. } => {
            if is_plain(destination) {
                Mutation::None
            } else {
                root_of(destination).map_or(Mutation::Unseen, |root| {
                    Mutation::Names(BTreeSet::from([root]))
                })
            }
        }
        LegacyInstruction::Escape { .. }
        | LegacyInstruction::EscapeIfOwned { .. }
        | LegacyInstruction::ExactFormalReturn { .. }
        | LegacyInstruction::BranchOnNull { .. }
        | LegacyInstruction::BranchOnValidity { .. }
        | LegacyInstruction::BranchOnResult { .. }
        | LegacyInstruction::BranchOnDistinct { .. }
        | LegacyInstruction::ForgetScalar { .. }
        | LegacyInstruction::VaArgPointer { .. }
        | LegacyInstruction::PointerStep { .. }
        | LegacyInstruction::CountedLoopHeader { .. }
        | LegacyInstruction::CountedLoopStep { .. }
        | LegacyInstruction::NoOp => Mutation::None,
        _ => Mutation::Unseen,
    }
}

/// Whether `name` is in the family of the borrow `record`: a borrow of the
/// same take, or one taken (transitively) from such a borrow.
fn in_family(borrows: &BTreeMap<String, BTreeSet<Borrow>>, record: &Borrow, name: &str) -> bool {
    let mut frontier = vec![name.to_owned()];
    let mut seen = BTreeSet::new();
    while let Some(current) = frontier.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        let Some(records) = borrows.get(&current) else {
            return false;
        };
        if records.is_empty() {
            return false;
        }
        for other in records {
            if other.key != record.key {
                if other.parent.is_empty() {
                    return false;
                }
                frontier.push(other.parent.clone());
            }
        }
    }
    true
}

fn end(state: &mut AbstractState, name: &str) {
    state.borrows.remove(name);
    state.pointers.remove(name);
    state
        .invalidated
        .insert(name.to_owned(), Invalid::BorrowEnded);
}

/// Hook 3a: before `node`'s transfer, a read of an invalidated name refuses,
/// and every borrow the instruction may mutate ends.
pub(super) fn before(
    function: &str,
    node: &LegacyCfgNode,
    mut state: AbstractState,
    scalars: &BTreeSet<String>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> AbstractState {
    if state.invalidated.is_empty() && state.borrows.is_empty() {
        return state;
    }
    if !state.invalidated.is_empty() {
        for name in reads(&node.instruction) {
            // Card REFCOUNT-RESOURCE-KIND (rule 13b): a drop's own actual
            // past an acquired family's last drop is the rule's to decide.
            if matches!(
                state.invalidated.get(&name),
                Some(Invalid::ReferenceFreed { .. })
            ) && super::reference_route::decides_read(&node.instruction, &name)
            {
                continue;
            }
            if let Some(reason) = state.invalidated.get(&name) {
                let description = match reason {
                    Invalid::SlotMoved => format!(
                        "use of `{name}` after its object moved or its parent changed ({ALIAS})"
                    ),
                    Invalid::BorrowEnded => format!(
                        "use of `{name}` after its object moved or its parent changed ({ENDED})"
                    ),
                    Invalid::ReferenceDropped | Invalid::ReferenceFreed { .. } => format!(
                        "use of `{name}` after a possibly-final drop of its reference ({})",
                        crate::reference_count::DROPPED
                    ),
                };
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line: node.line,
                    kind: LegacyDiagnosticKind::Unsupported { description },
                });
                state.unmodeled = true;
            }
        }
    }
    if state.borrows.is_empty() {
        return state;
    }
    let mutated = if node.lookup.is_some() {
        Mutation::None
    } else {
        mutation(&node.instruction)
    };
    let ended = state
        .borrows
        .iter()
        .filter(|(_, records)| match &mutated {
            Mutation::None => false,
            Mutation::Unseen => true,
            Mutation::Names(names) => names.iter().any(|name| {
                !scalars.contains(name)
                    && !records
                        .iter()
                        .all(|record| in_family(&state.borrows, record, name))
            }),
        })
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    for name in ended {
        end(&mut state, &name);
    }
    state
}

/// Hook 3b: after `node`'s transfer, the bookkeeping of rebound names, alias
/// copies and new borrowed results.
pub(super) fn after(node: &LegacyCfgNode, state: &mut AbstractState) {
    let rebound = super::rebound_names(&node.instruction);
    let borrowed_call = match &node.instruction {
        LegacyInstruction::ContractCall {
            destination: Some(destination),
            arguments,
            contract,
            ..
        } if is_plain(destination) => match contract.result {
            ResourceReturnMode::Derived {
                parameter,
                extent: DerivedExtent::Borrowed,
                ..
            } => Some((
                destination.clone(),
                arguments
                    .get(parameter)
                    .and_then(Option::as_deref)
                    .and_then(root_of)
                    .unwrap_or_default(),
            )),
            _ => None,
        },
        _ => None,
    };
    if rebound.is_empty() && borrowed_call.is_none() {
        return;
    }
    let copied = match &node.instruction {
        LegacyInstruction::Alias {
            destination,
            source,
        }
        | LegacyInstruction::PointerStep {
            destination,
            source,
            ..
        } if is_plain(destination) && is_plain(source) => state.borrows.get(source).cloned(),
        _ => None,
    };
    for name in &rebound {
        state.invalidated.remove(*name);
        state.borrows.remove(*name);
    }
    // A rebound parent no longer names the object its borrows were taken
    // from: they end.
    if !state.borrows.is_empty() {
        let orphaned = state
            .borrows
            .iter()
            .filter(|(_, records)| {
                records
                    .iter()
                    .any(|record| rebound.contains(&record.parent.as_str()))
            })
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        for name in orphaned {
            end(state, &name);
        }
    }
    if let Some(records) = copied
        && let Some(destination) = rebound.first()
    {
        state.borrows.insert((*destination).to_owned(), records);
    }
    if let Some((destination, parent)) = borrowed_call {
        // A borrow whose binding the call did not keep (a refused call)
        // records nothing.
        if state.pointers.get(&destination).is_some_and(|pointer| {
            matches!(
                pointer.derivation,
                Derivation::Derived {
                    extent: DerivedExtent::Borrowed,
                    ..
                }
            )
        }) {
            state.borrows.insert(
                destination.clone(),
                BTreeSet::from([Borrow {
                    key: (node.line, destination),
                    parent,
                }]),
            );
        }
    }
}

/// Join two paths' records: by union (an invalidation on one path stands).
pub(super) fn join(state: &mut AbstractState, incoming: &AbstractState) {
    for (name, reason) in &incoming.invalidated {
        state
            .invalidated
            .entry(name.clone())
            .and_modify(|current| *current = (*current).min(*reason))
            .or_insert(*reason);
    }
    for (name, records) in &incoming.borrows {
        state
            .borrows
            .entry(name.clone())
            .or_default()
            .extend(records.iter().cloned());
    }
}

/// The parameter names and body words of `function`'s definition in `tokens`.
pub(super) fn defined_body<'a>(
    tokens: &'a [Token],
    function: &str,
) -> Option<(Vec<String>, Vec<&'a str>)> {
    let (parameters, open) = super::retention_guard::definition(tokens, function)?;
    let close = super::matching(tokens, open, "{", "}")?;
    let words = tokens[open..=close]
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    Some((parameters, arrows(&words)))
}

/// The end (exclusive) of the expression starting at `start` in `words`: the
/// first `;`, or `,` or closing bracket at its own depth.
fn expression_end(words: &[&str], start: usize) -> usize {
    let mut depth = 0usize;
    for (index, word) in words.iter().enumerate().skip(start) {
        match *word {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                if depth == 0 {
                    return index;
                }
                depth -= 1;
            }
            ";" => return index,
            "," if depth == 0 => return index,
            _ => {}
        }
    }
    words.len()
}

/// Whether `words` is a type group: identifiers, `*` and keywords, the first
/// an identifier.
fn is_type_group(words: &[&str]) -> bool {
    words.first().is_some_and(|word| is_identifier(word))
        && words.iter().all(|word| *word == "*" || is_identifier(word))
}

/// Whether the expression `words` is NULL or a pointer reached from a name in
/// `valid` by member reads, an address of such a member, or an offset of one.
fn reached(words: &[&str], valid: &BTreeSet<String>) -> bool {
    let mut words = strip(words);
    // Casts.
    while words.first() == Some(&"(")
        && let Some(close) = close_of(words, 0)
        && close + 1 < words.len()
        && is_type_group(&words[1..close])
        // `( name ) -> f` is a parenthesised operand, never a cast.
        && !matches!(words[close + 1], "->" | "." | "[")
    {
        words = strip(&words[close + 1..]);
    }
    if is_null_constant(words) {
        return true;
    }
    if let ["&", rest @ ..] = words {
        return rest.len() > 1 && chain(rest, valid);
    }
    if chain(words, valid) {
        return true;
    }
    // An offset of a reached pointer stays inside its object.
    if let Some(at) = words.iter().rposition(|word| matches!(*word, "+" | "-"))
        && at > 0
        && words[at + 1..]
            .iter()
            .all(|word| is_identifier(word) || word.bytes().all(|b| b.is_ascii_digit()))
        && words.len() > at + 1
    {
        return reached(&words[..at], valid);
    }
    false
}

/// `NAME (-> F | . F)*` with `NAME` in `valid`.
fn chain(words: &[&str], valid: &BTreeSet<String>) -> bool {
    // A parenthesised head, `( name ) -> f` (the expansion of `TAILQ_NEXT`).
    let (first, rest) = match words {
        ["(", name, ")", rest @ ..] => (name, rest),
        [first, rest @ ..] => (first, rest),
        [] => return false,
    };
    if !valid.contains(*first) || rest.len() % 2 != 0 {
        return false;
    }
    rest.chunks(2)
        .all(|pair| matches!(pair[0], "->" | ".") && is_identifier(pair[1]))
}

/// Every `NAME = EXPR` of `body` (a declaration's initialiser included), by
/// name; a store through `*`, `.` or `->` is not one.
fn assignments<'a>(body: &[&'a str]) -> BTreeMap<&'a str, Vec<(usize, usize)>> {
    let mut found: BTreeMap<&str, Vec<(usize, usize)>> = BTreeMap::new();
    for at in 1..body.len().saturating_sub(1) {
        if body[at] != "=" || matches!(body[at + 1], "=") {
            continue;
        }
        let name = body[at - 1];
        // A store through `.`, `->` or a leading `*` is not an assignment of
        // the name; `T *name = …` (a declarator's `*` after a type) is.
        let before = at.checked_sub(2).map(|p| body[p]);
        let store_star = before == Some("*")
            && at
                .checked_sub(3)
                .is_none_or(|p| matches!(body[p], ";" | "{" | "}" | ")" | "(" | ","));
        if !is_identifier(name) || matches!(before, Some("." | "->")) || store_star {
            continue;
        }
        let end = expression_end(body, at + 1);
        found.entry(name).or_default().push((at + 1, end));
    }
    found
}

/// Whether `body` stores `name` into a member of a name in `valid`
/// (`P->F = name;`).
fn stored_into(body: &[&str], name: &str, valid: &BTreeSet<String>) -> bool {
    (2..body.len().saturating_sub(1)).any(|at| {
        body[at] == "="
            && body[at + 1] == name
            && matches!(body.get(at + 2).copied(), Some(";"))
            && {
                // The left-hand side, back to the statement's start.
                let start = (0..at)
                    .rev()
                    .find(|index| matches!(body[*index], ";" | "{" | "}" | ")"))
                    .map_or(0, |index| index + 1);
                let left = &body[start..at];
                left.len() >= 3 && chain(left, valid)
            }
    })
}

/// The names of `body` whose every assignment is reached from `parent` (the
/// greatest fixpoint), `parent` included, or `None` when `parent` is itself
/// assigned. `created` admits an assignment from a call whose result the
/// body stores into a member of a reached name (a lazily created child).
fn valid_locals(body: &[&str], parent: &str, created: bool) -> Option<BTreeSet<String>> {
    let assigned = assignments(body);
    // `&name` alone (not `&name->f`, the address of a member).
    let whole_address = |at: usize| {
        at > 0
            && body[at - 1] == "&"
            && !matches!(body.get(at + 1).copied(), Some("->" | "." | "["))
    };
    let address_taken = |name: &str| {
        body.iter()
            .enumerate()
            .any(|(at, word)| *word == name && whole_address(at))
    };
    if assigned.contains_key(parent) || address_taken(parent) {
        return None;
    }
    let mut valid = assigned
        .keys()
        .map(|name| (*name).to_owned())
        .chain(std::iter::once(parent.to_owned()))
        .collect::<BTreeSet<_>>();
    // A local whose address is taken may be written through it: not reached.
    for (at, word) in body.iter().enumerate() {
        if whole_address(at) && *word != parent {
            valid.remove(*word);
        }
    }
    loop {
        let before = valid.len();
        for (name, sites) in &assigned {
            let ok = sites.iter().all(|&(start, end)| {
                let expression = &body[start..end];
                reached(expression, &valid)
                    || (created
                        && matches!(expression, [callee, "(", .., ")"] if is_identifier(callee)
                            && close_of(expression, 1) == Some(expression.len() - 1))
                        && stored_into(body, name, &valid))
            });
            if !ok {
                valid.remove(*name);
            }
        }
        if valid.len() == before {
            return Some(valid);
        }
    }
}

/// Rule 13a for a borrowed result: every `return` is NULL or reached from the
/// parent (through locals so assigned, or a child created and stored into a
/// member of the parent).
fn borrowed_validated(body: &[&str], parent: &str) -> bool {
    let Some(valid) = valid_locals(body, parent, true) else {
        return false;
    };
    let mut returns = 0usize;
    for at in 0..body.len() {
        if body[at] != "return" {
            continue;
        }
        returns += 1;
        let end = expression_end(body, at + 1);
        if !reached(&body[at + 1..end], &valid) {
            return false;
        }
    }
    returns > 0
}

/// Rule 13a for a slot move: every mention of the slot is a whole actual at a
/// slot-moving position.
fn slot_validated(body: &[&str], slot: &str, contracts: &ResourceContracts) -> bool {
    only_slot_mentions(body, slot, contracts, moves_at)
}

/// Rule 13a for a derived out slot: every mention is `*slot = EXPR;` with
/// `EXPR` reached from the base, or a whole actual at a derived-out position.
fn out_validated(body: &[&str], slot: &str, base: &str, contracts: &ResourceContracts) -> bool {
    let Some(valid) = valid_locals(body, base, false) else {
        return false;
    };
    body.iter().enumerate().all(|(at, token)| {
        if *token != slot || matches!(at.checked_sub(1).map(|p| body[p]), Some("." | "->")) {
            return true;
        }
        let store = at >= 1
            && body[at - 1] == "*"
            && at
                .checked_sub(2)
                .is_none_or(|p| matches!(body[p], ";" | "{" | "}" | ")"))
            && body.get(at + 1) == Some(&"=")
            && body.get(at + 2) != Some(&"=");
        if store {
            let end = expression_end(body, at + 2);
            return reached(&body[at + 2..end], &valid);
        }
        whole_actual(body, at).is_some_and(|(callee, index)| {
            contracts
                .retaining_contract(callee)
                .is_some_and(|declaration| outs_at(declaration, index))
        })
    })
}

/// Hook 4 (rule 13a): the unit's environment with every slot-moving,
/// derived-out or borrowed-result declaration whose body this unit defines
/// and does not validate withdrawn. `None` when nothing changes.
pub(crate) fn settle(tokens: &[Token], contracts: &ResourceContracts) -> Option<ResourceContracts> {
    let mut withdrawn = Vec::new();
    for (name, declaration) in contracts.retaining() {
        if declaration.slot_moves.is_empty() && declaration.derived_outs.is_empty() {
            continue;
        }
        let Some((parameters, body)) = defined_body(tokens, name) else {
            continue;
        };
        let named = |index: usize| parameters.get(index).map(String::as_str).unwrap_or("");
        let valid = declaration
            .slot_moves
            .iter()
            .all(|moved| slot_validated(&body, named(moved.slot), contracts))
            && declaration
                .derived_outs
                .iter()
                .all(|out| out_validated(&body, named(out.slot), named(out.base), contracts));
        if !valid {
            withdrawn.push(name.clone());
        } else {
            // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13g).
            crate::contract_dependencies::validated(name, "13a");
        }
    }
    for (name, contract) in contracts.functions() {
        let ResourceReturnMode::Derived {
            parameter,
            extent: DerivedExtent::Borrowed,
            ..
        } = contract.result
        else {
            continue;
        };
        let Some((parameters, body)) = defined_body(tokens, name) else {
            continue;
        };
        if !parameters
            .get(parameter)
            .is_some_and(|parent| !parent.is_empty() && borrowed_validated(&body, parent))
        {
            withdrawn.push(name.clone());
        } else {
            // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13g).
            crate::contract_dependencies::validated(name, "13a");
        }
    }
    if withdrawn.is_empty() {
        return None;
    }
    let mut settled = contracts.clone();
    for name in withdrawn {
        crate::contract_dependencies::withdrawn(&name, "13a", true);
        settled.withdraw_declaration(&name);
    }
    Some(settled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    #[test]
    fn the_measured_borrowed_bodies_validate() {
        let ret = words(
            "{ rd_kafka_op_type_t reqtype = rko -> rko_u . admin_result . reqtype & ~ 1 ; \
             * cntp = rd_list_cnt ( & rko -> rko_u . admin_result . results ) ; \
             return ( const rd_kafka_group_result_t * * ) rko -> rko_u . admin_result . results . rl_elems ; }",
        );
        assert!(borrowed_validated(&ret, "rko"));
        let item = words(
            "{ cJSON * current_element = NULL ; if ( ( object == NULL ) || ( name == NULL ) ) \
             { return NULL ; } current_element = object -> child ; \
             while ( ( current_element != NULL ) ) { current_element = current_element -> next ; } \
             if ( ( current_element == NULL ) ) { return NULL ; } return current_element ; }",
        );
        assert!(borrowed_validated(&item, "object"));
        let private = words(
            "{ rd_kafka_topic_partition_private_t * parpriv ; \
             if ( ! ( parpriv = rktpar -> _private ) ) { parpriv = rd_kafka_topic_partition_private_new ( ) ; \
             rktpar -> _private = parpriv ; } return parpriv ; }",
        );
        assert!(borrowed_validated(&private, "rktpar"));
    }

    #[test]
    fn a_body_returning_anything_else_does_not_validate() {
        // A fresh allocation not linked into the parent.
        let fresh = words("{ char * q ; q = make ( ) ; return q ; }");
        assert!(!borrowed_validated(&fresh, "p"));
        // Another formal.
        let other = words("{ return other -> child ; }");
        assert!(!borrowed_validated(&other, "p"));
        // The parent rebound.
        let rebound = words("{ p = p -> next ; return p ; }");
        assert!(!borrowed_validated(&rebound, "p"));
        // A local reassigned from a call.
        let called = words("{ T * c = p -> child ; c = lookup ( c ) ; return c ; }");
        assert!(!borrowed_validated(&called, "p"));
    }

    #[test]
    fn the_measured_out_slot_body_validates() {
        let reader = words(
            "{ size_t rof = slice -> rof ; size_t rlen ; const rd_segment_t * seg ; \
             for ( seg = slice -> seg ; seg && seg -> seg_absof + rof < slice -> end ; \
             seg = ( ( seg ) -> seg_link . tqe_next ) ) rof = 0 ; \
             if ( ! seg ) return 0 ; \
             * p = ( const void * ) ( seg -> seg_p + rof ) ; return rlen ; }",
        );
        let contracts = ResourceContracts::default();
        assert!(out_validated(&reader, "p", "slice", &contracts));
        let foreign = words("{ * p = other -> data ; return 0 ; }");
        assert!(!out_validated(&foreign, "p", "slice", &contracts));
    }
}
