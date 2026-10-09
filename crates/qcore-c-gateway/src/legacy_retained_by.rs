//! Card **CALLEE-RETENTION-CLASS1**, stage (b) (gateway rule 12t): the
//! route-G adapter of [`crate::retained_by`]. Five thin hooks, each the
//! identity on every environment that holds no retaining declaration:
//!
//! 1. [`retaining_environment`] (in `build_function_cfg_with`): the parser's
//!    environment, with each retaining declaration the stream names installed
//!    under its minted name ([`crate::retained_by::retained_name`]). The
//!    callee itself stays out of the function map, so every route that knows
//!    nothing of retention sees an uncontracted callee.
//! 2. [`Parser::retaining_call`] (the classifier's uncontracted fallback,
//!    asked before rule 11r's untested refusal): reads each retained pointer
//!    and holder actual as spelled, refuses what no state can decide, and
//!    classifies the call under its custody contract with the operands the
//!    transfer needs.
//! 3. [`apply`] (the transfer of a `ContractCall`, after the container
//!    effect): resets, then each retention decided against the state.
//! 4. [`refuse_release_of_held`] and [`holder_released`] (in
//!    `apply_resource_action`): a release or transfer of a held identity
//!    refuses; a released holder holds nothing.
//! 5. [`report_exit`] (at every exit): a tracked holder still holding
//!    outlives the frame.

use std::collections::BTreeSet;

use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::retained_by::RetentionHolder;

use super::{
    AbstractState, LIVE, LegacyDiagnostic, LegacyDiagnosticKind, LegacyInstruction, Parser, Token,
};
use crate::retained_by::{
    Decision, HeldBy, HeldObject, Holder, Retained, decide, exit_description,
    released_while_held_description, retained_callee, retained_name, undecided_description,
};

/// A retained pointer's actual, as spelled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RetainedSpelling {
    /// A string literal.
    Literal,
    /// An unrebound formal of the calling function.
    Formal(String),
    /// Any other plain name.
    Name(String),
}

/// A holder's actual, as spelled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HolderSpelling {
    /// `&L`, `L` a frame holder of the calling function.
    Frame(String),
    /// A plain name.
    Name(String),
    /// The declaration's `global` holder.
    Global,
    /// Card **REFCOUNT-REFERENCE-RETURNING-DECLARATIONS** (rule 13e): the
    /// object the call returns, decided against the call's destination.
    Result,
}

/// The operands of one retaining call, in clause order.
#[derive(Clone, PartialEq, Eq)]
pub struct Operands {
    /// `(retained, its spelling, holder, its spelling)`.
    pub retentions: Vec<(RetainedSpelling, String, HolderSpelling, String)>,
    pub resets: Vec<HolderSpelling>,
    /// Card **SLOT-MOVE-AND-INTERIOR-BORROW** (rules 12x, 12z): the slot
    /// operands, applied after the retentions ([`super::slot_move_route`]).
    pub slots: Vec<super::slot_move_route::SlotOperand>,
    /// Card **REFCOUNT-RESOURCE-KIND** (rule 13b): the keeps and drops,
    /// applied after the slots ([`super::reference_route`]).
    pub references: Vec<super::reference_route::ReferenceOperand>,
    /// Card **CG22-CALLBACK-REGISTRY-CONTRACT** (rules 13w, 13x): the
    /// registry operands, applied after the references
    /// ([`super::callback_route`]). `None` for every call whose declaration
    /// carries no registry clause.
    pub callbacks: Option<Box<super::callback_route::Operands>>,
}

/// The derived form, with `callbacks` printed only when present, so every
/// retaining call before card CG22-CALLBACK-REGISTRY-CONTRACT prints (in the
/// parity stream) exactly as it did.
impl std::fmt::Debug for Operands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = f.debug_struct("Operands");
        out.field("retentions", &self.retentions)
            .field("resets", &self.resets)
            .field("slots", &self.slots)
            .field("references", &self.references);
        if let Some(callbacks) = &self.callbacks {
            out.field("callbacks", callbacks);
        }
        out.finish()
    }
}

/// The environment the parser reads: `contracts` plus the custody contract of
/// every retaining declaration the stream names, under its minted name.
/// `None` when there is nothing to add.
pub(super) fn retaining_environment(
    tokens: &[Token],
    contracts: &ResourceContracts,
) -> Option<ResourceContracts> {
    let retaining = contracts.retaining();
    if retaining.is_empty()
        || !tokens
            .iter()
            .any(|token| retaining.contains_key(&token.text))
    {
        return None;
    }
    let mut environment = contracts.clone();
    for (name, declaration) in retaining {
        // Card REFCOUNT-RESOURCE-KIND: a keep's alias result is the route's.
        environment.insert_contract(
            retained_name(name),
            super::reference_route::custody_contract(declaration),
        );
    }
    Some(environment)
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// The index of the `)` closing the `(` at `open`.
fn close_of(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
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

/// Whether `name` is mentioned in `body` only as the declaration of the
/// object and as whole `&name` actuals of calls to retaining callees.
fn only_holder_mentions(body: &[&str], name: &str, contracts: &ResourceContracts) -> bool {
    let mut declarations = 0usize;
    for (index, token) in body.iter().enumerate() {
        if *token != name {
            continue;
        }
        let previous = index.checked_sub(1).map(|at| body[at]);
        let next = body.get(index + 1).copied();
        if matches!(previous, Some("." | "->")) {
            continue;
        }
        if previous == Some("&") && matches!(next, Some("," | ")")) {
            // The enclosing call's callee: the identifier before the nearest
            // unmatched `(` to the left.
            let mut depth = 0usize;
            let mut callee = None;
            for at in (0..index.saturating_sub(1)).rev() {
                match body[at] {
                    ")" => depth += 1,
                    "(" if depth == 0 => {
                        callee = at.checked_sub(1).map(|c| body[c]);
                        break;
                    }
                    "(" => depth -= 1,
                    _ => {}
                }
            }
            // Card SLOT-MOVE-AND-INTERIOR-BORROW: only a callee that retains
            // or resets is a holder position, not one with slot clauses alone.
            if callee.is_some_and(|callee| {
                contracts
                    .retaining_contract(callee)
                    .is_some_and(qcore_schemas::retained_by::RetainingContract::retains)
            }) {
                continue;
            }
            return false;
        }
        // The declarator itself: `T name ;` or `T name = …;`.
        if matches!(next, Some(";" | "="))
            && previous.is_some_and(|previous| is_identifier(previous) || previous == "}")
        {
            declarations += 1;
            continue;
        }
        return false;
    }
    declarations == 1
}

impl Parser<'_> {
    /// Hook (rule 12t): classify a call to a retaining callee, or `None` when
    /// `function` has no retaining declaration.
    pub(super) fn retaining_call(
        &mut self,
        tokens: &[Token],
        texts: &[&str],
        call_index: usize,
        function: &str,
    ) -> Option<LegacyInstruction> {
        let declaration = self.contracts.retaining_contract(function)?.clone();
        let refuse = |this: &Self, description: String| LegacyInstruction::Unknown {
            description,
            writes: this.refused_writes(texts),
        };
        let Some(spans) = super::call_argument_spans(texts, call_index) else {
            return Some(refuse(self, undecided_description(function, "?", "?")));
        };
        let actual = |index: usize| -> Option<&[&str]> {
            spans
                .get(index)
                .map(|&(start, end)| strip(&texts[start..end]))
        };
        let body = self.function_body_texts();
        let holder = |this: &Self, index: usize| -> Option<(HolderSpelling, String)> {
            let spelled = actual(index)?;
            let text = spelled.join(" ");
            match spelled {
                ["&", name] if is_identifier(name) => {
                    let frame = body
                        .as_deref()
                        .is_some_and(|body| only_holder_mentions(body, name, this.contracts))
                        && this.typed_positions_index().is_some_and(|index| {
                            super::in_place_custody::one_automatic_object(&index.function, name)
                        });
                    frame.then(|| (HolderSpelling::Frame((*name).to_owned()), text))
                }
                [name] if is_identifier(name) => {
                    Some((HolderSpelling::Name((*name).to_owned()), text))
                }
                _ => None,
            }
        };
        let mut operands = Operands {
            retentions: Vec::new(),
            resets: Vec::new(),
            slots: Vec::new(),
            references: Vec::new(),
            callbacks: None,
        };
        // Card REFCOUNT-RESOURCE-KIND (rules 13b, 13c): the reference
        // operands, or the refusal of a declaration withdrawn in this unit.
        // Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS (rule 13d): a
        // returned reference rule 13d withdrew carries no clause, but refuses.
        if !declaration.references.is_empty() || declaration.references_withdrawn {
            let owned = |index: usize| -> Option<Vec<String>> {
                actual(index).map(|words| words.iter().map(|word| (*word).to_owned()).collect())
            };
            match self.reference_operands(function, &declaration, &owned) {
                Ok(references) => operands.references = references,
                Err(description) => return Some(refuse(self, description)),
            }
        }
        // Card CG22-CALLBACK-REGISTRY-CONTRACT (rules 13u-13x): the
        // synchronous invocations and table dispatches, decided here, and the
        // registry operands, or the call's refusal.
        if !declaration.callbacks.is_empty() || declaration.callbacks_withdrawn {
            let owned = |index: usize| -> Option<Vec<String>> {
                actual(index).map(|words| words.iter().map(|word| (*word).to_owned()).collect())
            };
            let line = tokens.get(call_index).map_or(0, |token| token.line);
            let frame = |name: &str| self.frame_registry_holder(name);
            match self.callback_operands(function, &declaration, line, &owned, &frame) {
                Ok(callbacks) => operands.callbacks = callbacks.map(Box::new),
                Err(description) => return Some(refuse(self, description)),
            }
        }
        for retention in &declaration.retentions {
            // Card W43-ABI-ENUM-AND-CONST-PP (rule 12w): a guarded clause a
            // decided actual fails is not in force at this call; a decided
            // success and an undecided actual keep it (the cautious reading).
            if let Some(guard) = retention.guard {
                let decision = super::retention_guard::decide(
                    self.tokens,
                    &self.typed_path_function,
                    actual(guard.parameter),
                    guard,
                );
                if !qcore_schemas::retained_by::RetentionGuard::in_force(decision) {
                    continue;
                }
            }
            let retained_text = actual(retention.retained).map(|a| a.join(" "));
            let retained = match actual(retention.retained) {
                // The lexer collapses a string literal to this placeholder
                // (premise P-RETAINED-LITERAL: at a pointer position it is a
                // string literal, since a character literal there is an
                // integer-to-pointer conversion Clang rejects by default).
                Some(["__qcore_literal__"]) => Some(RetainedSpelling::Literal),
                // Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS (rule 13e,
                // P-RETAINED-PREDEFINED): `__func__` is a static array (C11
                // 6.4.2.2p1) and `__FUNCTION__`, `__PRETTY_FUNCTION__` are
                // its GNU spellings, which Clang lexes as keywords.
                Some([name]) if crate::retained_by::PREDEFINED_IDENTIFIERS.contains(name) => {
                    Some(RetainedSpelling::Literal)
                }
                Some([name]) if is_identifier(name) => {
                    Some(if self.caller_formals.contains(name) {
                        RetainedSpelling::Formal((*name).to_owned())
                    } else {
                        RetainedSpelling::Name((*name).to_owned())
                    })
                }
                _ => None,
            };
            let held = match retention.holder {
                RetentionHolder::Global => Some((HolderSpelling::Global, "global".to_owned())),
                RetentionHolder::Result => Some((HolderSpelling::Result, "result".to_owned())),
                RetentionHolder::Parameter { index } => holder(self, index),
            };
            let holder_text = match retention.holder {
                RetentionHolder::Parameter { index } => actual(index).map(|a| a.join(" ")),
                RetentionHolder::Global => Some("global".to_owned()),
                RetentionHolder::Result => Some("the returned object".to_owned()),
            }
            .unwrap_or_else(|| "?".to_owned());
            let retained_text = retained_text.unwrap_or_else(|| "?".to_owned());
            match (retained, held) {
                // A static `P` outlives every holder; a non-static one in the
                // program-lifetime holder is undecided whatever the state says.
                (Some(retained), Some((held, _)))
                    if held != HolderSpelling::Global || retained == RetainedSpelling::Literal =>
                {
                    operands
                        .retentions
                        .push((retained, retained_text, held, holder_text));
                }
                _ => {
                    return Some(refuse(
                        self,
                        undecided_description(function, &retained_text, &holder_text),
                    ));
                }
            }
        }
        for &reset in &declaration.resets {
            // A reset of a holder this body cannot name forgets nothing it
            // recorded, which is the identity: it keeps the borrowed reading.
            if let Some((held, _)) = holder(self, reset) {
                operands.resets.push(held);
            }
        }
        // Card SLOT-MOVE-AND-INTERIOR-BORROW (rules 12x, 12z): the slot
        // operands, or the call's refusal.
        if !declaration.slot_moves.is_empty() || !declaration.derived_outs.is_empty() {
            let owned = |index: usize| -> Option<Vec<String>> {
                actual(index).map(|words| words.iter().map(|word| (*word).to_owned()).collect())
            };
            match self.slot_operands(function, &declaration, &owned, body.as_deref()) {
                Ok(slots) => operands.slots = slots,
                Err(description) => return Some(refuse(self, description)),
            }
        }
        // Classify the call under its custody contract, then name the callee.
        let mut rewritten = tokens.to_vec();
        rewritten[call_index].text = retained_name(function);
        let mut instruction = self.classify_simple(&rewritten);
        match &mut instruction {
            LegacyInstruction::ContractCall {
                function: called,
                retention,
                ..
            } => {
                function.clone_into(called);
                *retention = Some(Box::new(operands));
            }
            LegacyInstruction::Unknown { description, .. } => {
                *description = description.replace(&retained_name(function), function);
            }
            _ => {}
        }
        // Card REFCOUNT-RESOURCE-KIND (rule 13b): the site a keep may mint.
        self.reserve_reference_site(&mut instruction);
        Some(instruction)
    }

    /// The calling function's body, as texts, between its braces.
    fn function_body_texts(&self) -> Option<Vec<&str>> {
        let (start, _) = super::find_function_body(self.tokens, &self.typed_path_function)?;
        let end = super::matching(self.tokens, start, "{", "}")?;
        Some(
            self.tokens[start + 1..end]
                .iter()
                .map(|token| token.text.as_str())
                .collect(),
        )
    }
}

/// The one known live identity `name` denotes, if it denotes exactly one.
pub(super) fn tracked(state: &AbstractState, name: &str) -> Option<usize> {
    let pointer = state.pointers.get(name)?;
    if pointer.unknown || pointer.may_be_null || !pointer.derivation.is_identity() {
        return None;
    }
    let [site] = pointer.targets.iter().copied().collect::<Vec<_>>()[..] else {
        return None;
    };
    (state.resources.get(&site).copied().unwrap_or(0) & LIVE != 0
        && !state.unresolved_effect_sites.contains(&site))
    .then_some(site)
}

fn holder_class(
    state: &AbstractState,
    holder: &HolderSpelling,
    destination: Option<&str>,
) -> Holder {
    match holder {
        HolderSpelling::Frame(name) => Holder::Frame(name.clone()),
        HolderSpelling::Global => Holder::Global,
        HolderSpelling::Name(name) => tracked(state, name).map_or(Holder::Foreign, Holder::Tracked),
        // Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS (rule 13e): the
        // returned object is the one live identity the call bound its plain
        // destination to; any other destination is undecided.
        HolderSpelling::Result => destination
            .filter(|place| crate::may_alias::place_root(place) == Some(*place))
            .and_then(|place| tracked(state, place))
            .map_or(Holder::Foreign, Holder::Tracked),
    }
}

pub(super) fn retained_class(state: &AbstractState, retained: &RetainedSpelling) -> Retained {
    match retained {
        RetainedSpelling::Literal => Retained::Static,
        RetainedSpelling::Formal(name) => match state.pointers.get(name) {
            // Bound: a formal the body was handed custody of (a W4 entry) is
            // tracked like any identity.
            Some(_) => tracked(state, name).map_or(Retained::Other, Retained::Tracked),
            None => Retained::CallerObject(name.clone()),
        },
        RetainedSpelling::Name(name) => {
            tracked(state, name).map_or(Retained::Other, Retained::Tracked)
        }
    }
}

/// Hook (rule 12t): the call's resets, then each retention decided against
/// `state`.
pub(super) fn apply(
    function: &str,
    line: usize,
    called: &str,
    operands: &Operands,
    destination: Option<&str>,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for reset in &operands.resets {
        match holder_class(state, reset, destination) {
            Holder::Frame(name) => state
                .retained
                .retain(|(_, by)| *by != HeldBy::Frame(name.clone())),
            Holder::Tracked(site) => state.retained.retain(|(_, by)| *by != HeldBy::Site(site)),
            Holder::Global | Holder::Foreign => {}
        }
    }
    for (retained, retained_text, holder, holder_text) in &operands.retentions {
        match decide(
            &retained_class(state, retained),
            &holder_class(state, holder, destination),
        ) {
            Decision::Outlives => {}
            Decision::Record(object, by) => {
                state.retained.insert((object, by));
            }
            Decision::Undecided => {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::Unsupported {
                        description: undecided_description(
                            retained_callee(called).unwrap_or(called),
                            retained_text,
                            holder_text,
                        ),
                    },
                });
                state.unmodeled = true;
            }
        }
    }
    // Card SLOT-MOVE-AND-INTERIOR-BORROW (rules 12x, 12z).
    if !operands.slots.is_empty() {
        super::slot_move_route::apply(
            function,
            line,
            retained_callee(called).unwrap_or(called),
            &operands.slots,
            state,
            diagnostics,
        );
    }
}

/// Hook (rule 12t): a release or transfer of `site` while a holder retains
/// it refuses. `true` when it refused.
pub(super) fn refuse_release_of_held(
    function: &str,
    line: usize,
    source: &str,
    site: usize,
    state: &AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> bool {
    let held = state
        .retained
        .iter()
        .any(|(object, _)| *object == HeldObject::Site(site));
    if held {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: released_while_held_description(source),
            },
        });
    }
    held
}

/// Hook (rule 12t): a released holder holds nothing any more.
pub(super) fn holder_released(state: &mut AbstractState, site: usize) {
    if !state.retained.is_empty() {
        state.retained.retain(|(_, by)| *by != HeldBy::Site(site));
    }
}

/// Hook (rule 12t): at an exit, a tracked holder still holding outlives the
/// frame; a frame holder dies here.
pub(super) fn report_exit(
    function: &str,
    line: usize,
    state: &AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (object, by) in &state.retained {
        if matches!(by, HeldBy::Site(_)) {
            let name = match object {
                HeldObject::Name(name) => name.clone(),
                HeldObject::Site(_) => "a pointer this body tracks".to_owned(),
            };
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: exit_description(&name),
                },
            });
        }
    }
}
