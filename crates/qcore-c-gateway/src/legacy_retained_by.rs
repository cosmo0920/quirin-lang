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
    Decision, HeldBy, HeldObject, Holder, PathRoot, Retained, decide, end_holder, exit_description,
    poisoned_exit_description, released_while_held_description, removal_key, remove,
    retained_callee, retained_name, undecided_description,
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
    /// Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: `&X->f[->g]` (or
    /// `.`), a pointer to a member (at least one) of the object the plain
    /// name `X` denotes; `true` when `X` is an unrebound formal.
    Interior(String, Vec<String>, bool),
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
    /// Card **FB-EVENT-LOOP-RETENTION-AND-REGISTRATION** (rule 12t, USER
    /// DECISION 2026-10-16, "Tracked holder path"): the object at a member
    /// path below an unrebound formal this body names only through member
    /// accesses off the path (`coll->evl`). Anything else spelled through a
    /// member is not a holder this body can name.
    Path(String, Vec<String>),
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
    /// Card **FB-EVENT-LOOP-RETENTION-AND-REGISTRATION** (rule 12t): each
    /// removal's `(retained, holder)` as spelled, `None` where the actual is
    /// not a spelling a record can be keyed on (the removal drops nothing).
    pub removals: Vec<(Option<RetainedSpelling>, Option<HolderSpelling>)>,
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
        if !self.removals.is_empty() {
            out.field("removals", &self.removals);
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

/// Card **FB-EVENT-LOOP-RETENTION-AND-REGISTRATION** (rule 12t, premise
/// `P-HOLDER-PATH-STABLE`): whether `root` names, throughout `body`, only
/// member accesses (`root->f` / `root.f`) and none of them stores into, takes
/// the address of, or steps a member path that is a prefix of `fields`. The
/// path then denotes one object for the whole body: the formal is never
/// copied or rebound, and no call receives it whole or a view of the path.
fn path_stable(body: &[&str], root: &str, fields: &[String]) -> bool {
    let body = merged_punctuators(body);
    let body = body.iter().map(String::as_str).collect::<Vec<_>>();
    let mut seen = false;
    for (index, token) in body.iter().enumerate() {
        if *token != root {
            continue;
        }
        let previous = index.checked_sub(1).map(|at| body[at]);
        if matches!(previous, Some("." | "->")) {
            continue;
        }
        seen = true;
        // The member chain that follows the mention.
        let mut at = index + 1;
        let mut chain: Vec<&str> = Vec::new();
        while matches!(body.get(at), Some(&"->" | &".")) {
            let Some(name) = body.get(at + 1) else {
                return false;
            };
            chain.push(name);
            at += 2;
        }
        if chain.is_empty() {
            return false;
        }
        let on_path = chain
            .iter()
            .zip(fields)
            .take_while(|(segment, field)| **segment == field.as_str())
            .count();
        // A prefix of the path (or the path itself, or below it) is written,
        // stepped or has its address taken.
        let prefix = on_path == chain.len().min(fields.len()) && on_path > 0;
        let written = matches!(
            body.get(at),
            Some(&"=" | &"+=" | &"-=" | &"++" | &"--" | &"|=" | &"&=" | &"^=")
        ) || matches!(previous, Some("++" | "--"));
        let address = previous == Some("&");
        if prefix && (written || address) {
            return false;
        }
    }
    seen
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
        if self.contracts.lifetime_contract(function).is_some() {
            return Some(LegacyInstruction::Unknown {
                description: format!("lifetime-precondition-undischarged: {function}: scoped condition/protocol requires run-wide discharge"),
                writes: self.refused_writes(texts),
            });
        }
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
                // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: a member
                // path below an unrebound formal of this function.
                _ => this.path_holder(spelled, &[], body.as_deref()),
            }
        };
        let mut operands = Operands {
            retentions: Vec::new(),
            resets: Vec::new(),
            slots: Vec::new(),
            references: Vec::new(),
            callbacks: None,
            removals: Vec::new(),
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
        if !declaration.callbacks.is_empty()
            || declaration.callbacks_withdrawn
            || declaration.discriminant.is_some()
            || declaration.arms_withdrawn
            || !declaration.constructed.is_empty()
            || declaration.handoff.is_some()
            || !declaration.member_paths.is_empty()
        {
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
            let retained = self.retained_spelling(actual(retention.retained));
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
        // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (rule 12t): a
        // retention whose declared holder is a path below a formal: the
        // actual at that formal followed by the path's members.
        for retention in &declaration.path_retentions {
            let retained_text =
                actual(retention.retained).map_or_else(|| "?".to_owned(), |a| a.join(" "));
            let root = actual(retention.root);
            let holder_text = root.map_or_else(
                || "?".to_owned(),
                |a| format!("{}->{}", a.join(" "), retention.fields.join("->")),
            );
            let retained = self.retained_spelling(actual(retention.retained));
            let held =
                root.and_then(|root| self.path_holder(root, &retention.fields, body.as_deref()));
            match (retained, held) {
                (Some(retained), Some((held, _))) => {
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
        // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (rule 12t, USER
        // DECISION 2026-10-16, "R3: quant state"): each removal, never a
        // refusal. Card RETENTION-UNMATCHED-REMOVAL-POISONS: an actual no
        // record can be keyed on (`None`) drops nothing but poisons what it
        // may name ([`apply`]).
        for removal in &declaration.removals {
            let retained = self.retained_spelling(actual(removal.retained));
            let held = holder(self, removal.holder).map(|(held, _)| held);
            operands.removals.push((retained, held));
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
                contract,
                ..
            } => {
                function.clone_into(called);
                // Card CG22-CALLBACK-REGISTRY-CONTRACT (rule 13u): a callback
                // that requires a caller's actual live puts the requirement
                // on the call's own position.
                if let Some(callbacks) = &operands.callbacks {
                    for (&index, &nullable) in &callbacks.owned {
                        if let Some(
                            mode @ qcore_schemas::contract::ResourceParameterMode::Borrowed,
                        ) = contract.parameters.get_mut(index)
                        {
                            *mode =
                                qcore_schemas::contract::ResourceParameterMode::Owned { nullable };
                        }
                    }
                }
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

    /// A retained pointer's actual, as spelled (rule 12t), or `None` for a
    /// spelling no record can be keyed on.
    fn retained_spelling(&self, actual: Option<&[&str]>) -> Option<RetainedSpelling> {
        match actual {
            // The lexer collapses a string literal to this placeholder
            // (premise P-RETAINED-LITERAL: at a pointer position it is a
            // string literal, since a character literal there is an
            // integer-to-pointer conversion Clang rejects by default).
            Some(["__qcore_literal__"]) => Some(RetainedSpelling::Literal),
            // Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS (rule 13e,
            // P-RETAINED-PREDEFINED): `__func__` is a static array (C11
            // 6.4.2.2p1) and `__FUNCTION__`, `__PRETTY_FUNCTION__` are its GNU
            // spellings, which Clang lexes as keywords.
            Some([name]) if crate::retained_by::PREDEFINED_IDENTIFIERS.contains(name) => {
                Some(RetainedSpelling::Literal)
            }
            Some([name]) if is_identifier(name) => Some(if self.caller_formals.contains(name) {
                RetainedSpelling::Formal((*name).to_owned())
            } else {
                RetainedSpelling::Name((*name).to_owned())
            }),
            // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: `&X->f[->g]`.
            Some(["&", root, rest @ ..]) if is_identifier(root) => member_chain(rest)
                .filter(|fields| !fields.is_empty())
                .map(|fields| {
                    RetainedSpelling::Interior(
                        (*root).to_owned(),
                        fields,
                        self.caller_formals.contains(root),
                    )
                }),
            _ => None,
        }
    }

    /// Card **FB-EVENT-LOOP-RETENTION-AND-REGISTRATION** (rule 12t): the
    /// holder `root` (an actual) followed by the declared members `fields`,
    /// when the whole is a member path below an unrebound formal of this
    /// function that the body names only through member accesses off the
    /// path (`path_stable`); `None` otherwise.
    fn path_holder(
        &self,
        root: &[&str],
        fields: &[String],
        body: Option<&[&str]>,
    ) -> Option<(HolderSpelling, String)> {
        let (name, mut chain) = match root {
            [name, rest @ ..] if is_identifier(name) => ((*name).to_owned(), member_chain(rest)?),
            _ => return None,
        };
        chain.extend(fields.iter().cloned());
        if chain.is_empty()
            || !self.caller_formals.contains(name.as_str())
            || !body.is_some_and(|body| path_stable(body, &name, &chain))
        {
            return None;
        }
        let text = format!("{name}->{}", chain.join("->"));
        Some((HolderSpelling::Path(name, chain), text))
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

/// `-> f -> g` (or `.`) as member names, `None` for anything else; empty for
/// no tokens.
fn member_chain(tokens: &[&str]) -> Option<Vec<String>> {
    let tokens = merged_punctuators(tokens);
    let tokens = tokens.iter().map(String::as_str).collect::<Vec<_>>();
    let mut fields = Vec::new();
    let mut rest = tokens.as_slice();
    while let [access, name, tail @ ..] = rest {
        if !matches!(*access, "->" | ".") || !is_identifier(name) {
            return None;
        }
        fields.push((*name).to_owned());
        rest = tail;
    }
    rest.is_empty().then_some(fields)
}

/// `words` with every two-byte punctuator the lexer emits byte by byte
/// (`-` `>`, `=` `=`, `+` `+`, `+` `=`, …) read as one token.
fn merged_punctuators(words: &[&str]) -> Vec<String> {
    const PAIRS: [&str; 15] = [
        "->", "==", "!=", "<=", ">=", "++", "--", "+=", "-=", "*=", "/=", "|=", "&=", "^=", "%=",
    ];
    let mut merged = Vec::with_capacity(words.len());
    let mut at = 0;
    while at < words.len() {
        if let Some(next) = words.get(at + 1) {
            let pair = format!("{}{next}", words[at]);
            if words[at].len() == 1 && next.len() == 1 && PAIRS.contains(&pair.as_str()) {
                merged.push(pair);
                at += 2;
                continue;
            }
        }
        merged.push(words[at].to_owned());
        at += 1;
    }
    merged
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
        // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: an unbound formal
        // (one this body holds no custody of) roots a path holder.
        HolderSpelling::Path(root, fields) => {
            if state.pointers.contains_key(root) {
                Holder::Foreign
            } else {
                Holder::Path(PathRoot::Formal(root.clone()), fields.clone())
            }
        }
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
        // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: an interior of a
        // caller's object is that object's member; of a tracked identity, its
        // member; anything else is undecided.
        RetainedSpelling::Interior(root, fields, formal) => {
            match retained_class(
                state,
                &if *formal {
                    RetainedSpelling::Formal(root.clone())
                } else {
                    RetainedSpelling::Name(root.clone())
                },
            ) {
                Retained::CallerObject(name) => {
                    Retained::CallerObject(format!("{name}.{}", fields.join(".")))
                }
                Retained::Tracked(site) => Retained::TrackedInterior(site, fields.clone()),
                _ => Retained::Other,
            }
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
    // Card RETENTION-UNMATCHED-REMOVAL-POISONS: a reset holder holds
    // nothing any more, poisoned records included.
    for reset in &operands.resets {
        let by = match holder_class(state, reset, destination) {
            Holder::Frame(name) => HeldBy::Frame(name),
            Holder::Tracked(site) => HeldBy::Site(site),
            Holder::Path(root, fields) => HeldBy::Path(root, fields),
            Holder::Global | Holder::Foreign => continue,
        };
        end_holder(&mut state.retained, &mut state.poisoned, &by);
    }
    // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (rule 12t), corrected by
    // card RETENTION-UNMATCHED-REMOVAL-POISONS: a removal drops exactly the
    // unpoisoned record it names, and poisons every other record whose object
    // it may name; an actual no record keys on is an undecided side, which
    // names more, never less.
    for (retained, holder) in &operands.removals {
        let object = retained
            .as_ref()
            .map_or(Retained::Other, |retained| retained_class(state, retained));
        let holder = holder.as_ref().map_or(Holder::Foreign, |holder| {
            holder_class(state, holder, destination)
        });
        let key = removal_key(&object, &holder);
        remove(&mut state.retained, &mut state.poisoned, &key);
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
        .any(|(object, _)| object.site() == Some(site));
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
    // Card RETENTION-UNMATCHED-REMOVAL-POISONS: a released (destroyed) loop
    // ends every record it held, poisoned ones included.
    holder_ended(state, &HeldBy::Site(site));
}

/// Card **RETENTION-UNMATCHED-REMOVAL-POISONS** (rule 12t): a holder that
/// ends (released, destroyed, a registry's drop) holds nothing any more.
pub(super) fn holder_ended(state: &mut AbstractState, by: &HeldBy) {
    if !state.retained.is_empty() {
        end_holder(&mut state.retained, &mut state.poisoned, by);
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
    for record @ (object, by) in &state.retained {
        if matches!(by, HeldBy::Site(_) | HeldBy::Path(..)) {
            let name = match object {
                HeldObject::Name(name) => name.clone(),
                HeldObject::Site(_) | HeldObject::SiteInterior(..) => {
                    "a pointer this body tracks".to_owned()
                }
            };
            // Card RETENTION-UNMATCHED-REMOVAL-POISONS: a poisoned record
            // names why no removal ended it.
            let description = if state.poisoned.contains(record) {
                poisoned_exit_description(&name)
            } else {
                exit_description(&name)
            };
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::Unsupported { description },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{member_chain, path_stable};

    fn words(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    #[test]
    fn a_path_is_stable_only_where_its_formal_is_read_through_members_off_it() {
        let fields = ["evl".to_owned()];
        // Split punctuators, as the lexer emits them.
        let stable = words("{ ev_add ( coll - > evl , & coll - > event ) ; coll - > id = 1 ; }");
        assert!(path_stable(&stable, "coll", &fields));
        for unstable in [
            "{ coll - > evl = other ; }",
            "{ ev_touch ( coll ) ; }",
            "{ p = coll ; }",
            "{ peek ( & coll - > evl ) ; }",
            "{ coll - > evl + + ; }",
            // A write below the path is read cautiously as a write of it.
            "{ coll - > evl - > n = 1 ; }",
            "{ }",
        ] {
            assert!(
                !path_stable(&words(unstable), "coll", &fields),
                "{unstable}"
            );
        }
    }

    #[test]
    fn member_chains_read_both_spellings_of_the_arrow() {
        assert_eq!(
            member_chain(&["-", ">", "a", ".", "b"]),
            Some(vec!["a".to_owned(), "b".to_owned()])
        );
        assert_eq!(member_chain(&["->", "a"]), Some(vec!["a".to_owned()]));
        assert_eq!(member_chain(&["[", "0", "]"]), None);
    }
}
