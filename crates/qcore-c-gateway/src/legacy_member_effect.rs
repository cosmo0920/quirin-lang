// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Card **MEMBER-EFFECT-SUMMARY** step 2 (gateway rule 38j): the member-release
//! half of a member-effect summary.
//!
//! USER DECISION 2026-11-05 (verbatim): "Option 1 (rec.)"
//! (docs/reports/member-access-projection/README.md section 3).
//!
//! ## The gap
//!
//! Rule 5k admits a member release `R(B->F)` of a formal `B` only when the body
//! also releases `B` afterwards on every path (P4): a body that keeps `B`
//! (`cmt_opts_exit`, `metric_release_storage`) has no summary mode that states
//! what it released, so its caller could release or use the member again.
//!
//! ## The body check ([`BodyReading::read`])
//!
//! Over one body, for each formal `B` (by position), lexically:
//!
//! * **events.** A *release event* on the place `F` is `B->F` as the whole
//!   actual at a `Released { nullable: true }` position of a contract in
//!   force. A *call event* is `B` as the whole actual of a call whose
//!   member-effect summary is in force and names members at that position
//!   (an event on each named member `G`), or `&B->F` there (an event on the
//!   place `F`, releasing `F.G` for each named `G`): the cascade.
//! * **P1, the base is never copied.** Every other occurrence of `B` is a
//!   member access `B->G`, a null test of `B` in an `if` condition, or the
//!   whole single actual of a call whose contract releases its only parameter
//!   (a *base release*). Anything else and `B` contributes nothing.
//! * **P2, one event per place, nothing after it but a null store.** Each
//!   place has exactly one event; every other occurrence of `B->F` is a null
//!   test in an `if` condition before the event, or a store of a null pointer
//!   constant into `B->F` after it (`free(m->b); m->b = NULL;`).
//! * **P3, executed at most once.** No `goto`/`setjmp`/`longjmp` in the body,
//!   and the event is inside no loop (`do … while (0)` excepted).
//! * **The base's disposition.** With a base release, every event must be
//!   followed by one unconditionally, with no `return` between (rule 5k's P4),
//!   and the base release's own body must not touch the place (card
//!   W45-SR5-SOUNDNESS's reading); the member effect is then discharged inside
//!   the body and nothing is published. With no base release the body
//!   **keeps** `B`: each event is admitted and the body's member effect states
//!   it at `B`'s position (`stale`, or `cleared` when a null store follows the
//!   release in its own block on every path).
//!
//! The effect is a *may* effect: on a path that skips an event (an early
//! return, a false null test) the member is untouched. A caller therefore
//! never uses the member after the call, which is what its own reading of
//! the call as an event demands.
//!
//! ## Where the summary lives
//!
//! A body whose effect is not empty gets no ordinary W4 summary: its validated
//! contract goes, with the effect, to the environment's member-effect table
//! (`ResourceContracts::member_effect`), never to the function table, so
//! records, cross-unit imports, callback candidates and indirect calls see an
//! uncontracted function and refuse as before. A function's parser installs
//! each such contract under a reserved minted name ([`minted_name`]) that no
//! source spelling reaches ([`super::reserved_spelling`]), and only the
//! direct-call route ([`super::Parser::member_effect_call`]) classifies a call
//! under it, and only for a call its own body check admitted as an event.
//!
//! ## A member spelled through a pointer typedef
//!
//! Card **MEMBER-EFFECT-COMPONENT-SCOPE**: the body check reads no type, but
//! rule 1's lowering of a release event needs the call-argument recogniser's
//! pointer read of `B->F`, which the lexical declaration reader refuses for a
//! member spelled through a pointer typedef (`cfl_sds_t ns;`, left
//! unresolved). [`Parser::typed_member_release`] is asked there instead, only
//! for such an untyped member at a nullable release of an admitted place: Clang
//! reads the actual as the value of an object-pointer member of the
//! parameter's record, the fact the lexical reader gives a `*`-spelled member.
//!
//! Card **MEMBER-RELEASE-UNTYPED-BASE**: the same consultation where the
//! lexical reader refused the actual's **base** instead (a formal spelled
//! through a typedef of a record, `rd_kafka_toppar_t *rktp`, or of a record
//! pointer), so it never reached the member. Clang must then also read the
//! base: a pointer whose `->` member is a member of a complete `struct` the
//! unit defines (never a `union`), with no `volatile`/`_Atomic`.
//!
//! The Lean models are `QCore.MemberEffectSummary`,
//! `QCore.MemberReleaseTypedMember` and `QCore.MemberReleaseUntypedBase`.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};
use qcore_schemas::member_effect::{MemberEffect, MemberFate};

use super::member_release::{
    MemberEffect as CalleeTouch, NON_LOCAL_JUMPS, NOT_CALLS, brace_depths, is_null_test,
    loop_spans, member_effect as callee_touch, releases_its_only_parameter,
};
use super::{
    LegacyInstruction, Parser, Statement, Token, call_argument_spans, is_null_constant, matching,
    strip_parentheses,
};
use crate::typed_field_access::is_name;

/// The prefix a member-effect summary's contract is installed under in a
/// function's parser (reserved: a source spelling it is escaped first).
pub(crate) const MINTED_PREFIX: &str = "qcore_member_effect__";

/// The minted name of `callee`'s member-effect contract.
pub(crate) fn minted_name(callee: &str) -> String {
    format!("{MINTED_PREFIX}{callee}")
}

/// The stable key of a call to a member-effect function its caller's body
/// check did not admit.
pub(super) const CALL_REFUSED_KEY: &str = "member-effect-call-refused";

/// The refusal such a call carries.
pub(super) fn call_refused(callee: &str) -> String {
    format!(
        "call to `{callee}`, whose summary releases members of an argument, outside the \
         caller's member-effect premises ({CALL_REFUSED_KEY})"
    )
}

/// The parser's environment with every member-effect contract the stream
/// names installed under its minted name, or `None` when there is none.
pub(super) fn minted_environment(
    tokens: &[Token],
    contracts: &ResourceContracts,
) -> Option<ResourceContracts> {
    let effects = contracts.member_effects();
    if effects.is_empty() || !tokens.iter().any(|token| effects.contains_key(&token.text)) {
        return None;
    }
    let mut environment = contracts.clone();
    for (name, summary) in effects {
        environment.insert_contract(minted_name(name), summary.contract.clone());
    }
    Some(environment)
}

/// What one body's check admits and states.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct BodyReading {
    /// The `(base, member)` places whose release statement rule 5k's rule 1
    /// admits under this check (the position read `Borrowed`).
    pub(super) places: BTreeSet<(String, String)>,
    /// The `(callee, line)` call events this check admitted.
    pub(super) calls: BTreeSet<(String, usize)>,
    /// The member effect at each kept formal's position.
    pub(super) effect: MemberEffect,
}

/// One event on a place.
#[derive(Clone, Debug)]
struct Event {
    /// The index of the event's base token in the body.
    at: usize,
    /// The callee whose call the event is.
    callee: String,
    /// The callee token's line.
    line: usize,
    /// The paths released below the base, each with its fate as the callee
    /// leaves it (`None` for a release event: decided here).
    paths: Vec<(String, Option<MemberFate>)>,
    /// A release event, as opposed to a call event.
    release: bool,
}

/// The whole actuals of the body's calls: the index of the base token of
/// each actual spelled `B` or `& B -> F`, mapped to (callee index, actual
/// index, the member of an address actual).
fn whole_actuals(texts: &[&str]) -> BTreeMap<usize, (usize, usize, Option<String>)> {
    let mut actuals = BTreeMap::new();
    for at in 0..texts.len() {
        if !is_name(texts[at]) || texts.get(at + 1) != Some(&"(") || NOT_CALLS.contains(&texts[at])
        {
            continue;
        }
        let Some(spans) = call_argument_spans(texts, at) else {
            continue;
        };
        for (index, (start, end)) in spans.into_iter().enumerate() {
            let argument = &texts[start..end];
            let stripped = strip_parentheses(argument);
            let offset = (argument.len() - stripped.len()) / 2;
            match stripped {
                [name] if is_name(name) => {
                    actuals.insert(start + offset, (at, index, None));
                }
                ["&", name, "-", ">", member] if is_name(name) && is_name(member) => {
                    actuals.insert(start + offset + 1, (at, index, Some((*member).to_owned())));
                }
                _ => {}
            }
        }
    }
    actuals
}

/// The member actuals `B -> F` (index of `B` to callee index and actual index).
fn member_actuals(texts: &[&str]) -> BTreeMap<usize, (usize, usize)> {
    let mut actuals = BTreeMap::new();
    for at in 0..texts.len() {
        if !is_name(texts[at]) || texts.get(at + 1) != Some(&"(") || NOT_CALLS.contains(&texts[at])
        {
            continue;
        }
        let Some(spans) = call_argument_spans(texts, at) else {
            continue;
        };
        for (index, (start, end)) in spans.into_iter().enumerate() {
            let argument = &texts[start..end];
            let stripped = strip_parentheses(argument);
            if let [_, "-", ">", _] = stripped {
                let offset = (argument.len() - stripped.len()) / 2;
                actuals.insert(start + offset, (at, index));
            }
        }
    }
    actuals
}

/// Whether the tokens from `at` are a statement `B -> F = N ;` with `N` a null
/// pointer constant; the index of its `;`.
fn null_store(texts: &[&str], at: usize) -> Option<usize> {
    let end = (at..texts.len()).find(|index| texts[*index] == ";")?;
    match &texts[at..end] {
        [_, "-", ">", _, "=", value @ ..] if !value.is_empty() && is_null_constant(value) => {
            Some(end)
        }
        _ => None,
    }
}

/// Whether a token starts a statement (after `;`, `{`, `}` or the body's
/// opening).
fn statement_start(texts: &[&str], at: usize) -> bool {
    at == 0 || matches!(texts[at - 1], ";" | "{" | "}")
}

impl BodyReading {
    /// The check over `function`'s body (module documentation). `tokens` is
    /// the whole unit, `body_start` the index of the body's `{`.
    pub(super) fn read(
        tokens: &[Token],
        function: &str,
        body_start: usize,
        parameters: &[Option<String>],
        contracts: &ResourceContracts,
    ) -> Self {
        let mut reading = Self::default();
        let Some(body_end) = matching(tokens, body_start, "{", "}") else {
            return reading;
        };
        let body = &tokens[body_start + 1..body_end];
        let texts = body
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        if texts.iter().any(|text| NON_LOCAL_JUMPS.contains(text)) {
            return reading;
        }
        let depth = brace_depths(&texts);
        let loops = loop_spans(&texts, &depth);
        let releases = member_actuals(&texts);
        let wholes = whole_actuals(&texts);
        for (position, base) in parameters.iter().enumerate() {
            let Some(base) = base else { continue };
            if let Some(admitted) = read_base(
                tokens, &texts, body, &depth, &loops, &releases, &wholes, base, contracts,
            ) {
                for event in admitted.events {
                    if event.release {
                        reading
                            .places
                            .insert((base.clone(), event.paths[0].0.clone()));
                    } else {
                        reading.calls.insert((event.callee.clone(), event.line));
                    }
                    if admitted.kept {
                        let paths = reading.effect.released.entry(position).or_default();
                        for (path, fate) in event.paths {
                            paths.insert(path, fate.unwrap_or(MemberFate::Stale));
                        }
                    }
                }
            }
        }
        reading.effect.released.retain(|_, paths| !paths.is_empty());
        // A declaration governs what its function does through an argument
        // (P-B08-SHALLOW): a declared body states no member effect and keeps
        // every refusal a kept formal's event would lift.
        if !reading.effect.is_empty()
            && super::descriptor_custody::declared_callees(contracts).contains(function)
        {
            return Self::default();
        }
        reading
    }
}

/// The admitted events of one base, and whether the body keeps it.
struct Admitted {
    events: Vec<Event>,
    kept: bool,
}

#[allow(clippy::too_many_arguments)]
fn read_base(
    tokens: &[Token],
    texts: &[&str],
    body: &[Token],
    depth: &[usize],
    loops: &[(usize, usize)],
    releases: &BTreeMap<usize, (usize, usize)>,
    wholes: &BTreeMap<usize, (usize, usize, Option<String>)>,
    base: &str,
    contracts: &ResourceContracts,
) -> Option<Admitted> {
    let callee_effect = |callee: &str, index: usize| {
        contracts
            .member_effect(callee)
            .and_then(|summary| summary.effect.at(index))
            .cloned()
    };
    // Occurrences of each place other than its events, and the events.
    let mut others = BTreeMap::<String, Vec<usize>>::new();
    let mut events = BTreeMap::<String, Vec<Event>>::new();
    let mut base_releases = Vec::new();
    for at in 0..texts.len() {
        if texts[at] != base {
            continue;
        }
        let previous = at.checked_sub(1).map(|p| texts[p]);
        if previous == Some(".") || (previous == Some(">") && at >= 2 && texts[at - 2] == "-") {
            continue; // a member name that spells the formal's name
        }
        if let (Some("-"), Some(">"), Some(field)) = (
            texts.get(at + 1).copied(),
            texts.get(at + 2).copied(),
            texts.get(at + 3).copied(),
        ) && is_name(field)
        {
            // `& B -> F` as a whole actual of a member-effect callee.
            if previous == Some("&")
                && let Some((call, index, Some(member))) = wholes.get(&at)
                && member == field
                && let Some(paths) = callee_effect(texts[*call], *index)
            {
                events.entry(field.to_owned()).or_default().push(Event {
                    at,
                    callee: texts[*call].to_owned(),
                    line: body[*call].line,
                    paths: paths
                        .into_iter()
                        .map(|(path, fate)| (format!("{field}.{path}"), Some(fate)))
                        .collect(),
                    release: false,
                });
                continue;
            }
            // `B -> F` as a whole actual at a nullable release position.
            if let Some((call, index)) = releases.get(&at)
                && matches!(
                    contracts
                        .contract(texts[*call])
                        .and_then(|contract| contract.parameters.get(*index)),
                    Some(ResourceParameterMode::Released { nullable: true })
                )
            {
                events.entry(field.to_owned()).or_default().push(Event {
                    at,
                    callee: texts[*call].to_owned(),
                    line: body[*call].line,
                    paths: vec![(field.to_owned(), None)],
                    release: true,
                });
                continue;
            }
            others.entry(field.to_owned()).or_default().push(at);
            continue;
        }
        if is_null_test(texts, at, 1) {
            continue;
        }
        // `R ( B )` with `R` releasing its only parameter.
        if previous == Some("(")
            && texts.get(at + 1) == Some(&")")
            && at >= 2
            && is_name(texts[at - 2])
            && releases_its_only_parameter(contracts.contract(texts[at - 2]))
        {
            base_releases.push(at - 2);
            continue;
        }
        // `B` as a whole actual of a member-effect callee: an event on each
        // member it names.
        if let Some((call, index, None)) = wholes.get(&at)
            && let Some(paths) = callee_effect(texts[*call], *index)
        {
            for (path, fate) in paths {
                let place = path.split('.').next().unwrap_or(&path).to_owned();
                events.entry(place).or_default().push(Event {
                    at,
                    callee: texts[*call].to_owned(),
                    line: body[*call].line,
                    paths: vec![(path, Some(fate))],
                    release: false,
                });
            }
            continue;
        }
        return None; // P1: any other use of the base
    }
    if events.is_empty() {
        return None;
    }
    // A call event's callee reads its formal like a base release's does
    // (card W45-SR5-SOUNDNESS's reading): its body must be characterised,
    // and a whole-base call may touch no other place this body has an event
    // on (its own events' places are its body check's). An address actual
    // reaches only its member's sub-object, and a characterised body does no
    // arithmetic on its formal.
    let mut calls = BTreeMap::<(usize, &str), BTreeSet<String>>::new();
    for (place, place_events) in &events {
        for event in place_events.iter().filter(|event| !event.release) {
            calls
                .entry((event.at, event.callee.as_str()))
                .or_default()
                .insert(place.clone());
        }
    }
    for ((at, callee), own) in &calls {
        match callee_touch(tokens, callee, contracts, &mut BTreeSet::new()) {
            Some(CalleeTouch::Characterised { touched, .. }) => {
                let whole = texts.get(at.wrapping_sub(1)) != Some(&"&");
                if whole
                    && events
                        .keys()
                        .any(|place| !own.contains(place) && touched.contains(place))
                {
                    return None;
                }
            }
            None | Some(CalleeTouch::Unknown) => return None,
        }
    }
    let kept = base_releases.is_empty();
    let mut admitted = Vec::new();
    for (place, place_events) in events {
        let [event] = place_events.as_slice() else {
            return None; // P2: two events on one place
        };
        let mut fate = None;
        for at in others.get(&place).into_iter().flatten().copied() {
            if at < event.at && is_null_test(texts, at, 4) {
                continue;
            }
            if at > event.at && statement_start(texts, at) && null_store(texts, at).is_some() {
                // A null store after the release: `cleared` when it runs on
                // every path after the event, in its block.
                let event_end = (event.at..texts.len()).find(|index| texts[*index] == ";");
                let same_block = event_end.is_some_and(|end| {
                    depth[at] == depth[event.at]
                        && texts[end..at]
                            .iter()
                            .all(|text| !matches!(*text, "return" | "break" | "continue"))
                        && depth[end..at].iter().all(|d| *d >= depth[event.at])
                });
                if event.release && same_block && fate.is_none() {
                    fate = Some(MemberFate::Cleared);
                }
                continue;
            }
            return None; // P2: another use of the place
        }
        if loops
            .iter()
            .any(|(start, end)| *start < event.at && event.at < *end)
        {
            return None; // P3
        }
        if !kept {
            // Rule 5k's P4: a top-level `R(B);` after the event, no `return`
            // between; and no base release's body touches the place.
            let unconditionally_after = |call: usize| {
                call > event.at
                    && depth[call] == 0
                    && statement_start(texts, call)
                    && texts.get(call + 4) == Some(&";")
                    && !texts[event.at..call].contains(&"return")
            };
            if !base_releases
                .iter()
                .any(|call| unconditionally_after(*call))
            {
                return None;
            }
            for call in &base_releases {
                match callee_touch(tokens, texts[*call], contracts, &mut BTreeSet::new()) {
                    None => {}
                    Some(CalleeTouch::Characterised { touched, .. }) => {
                        if touched.contains(&place) {
                            return None;
                        }
                    }
                    Some(CalleeTouch::Unknown) => return None,
                }
            }
        }
        let mut event = event.clone();
        if event.release {
            event.paths[0].1 = Some(fate.unwrap_or(MemberFate::Stale));
        }
        admitted.push(event);
    }
    Some(Admitted {
        events: admitted,
        kept,
    })
}

/// The member effect `function`'s body states, read with `contracts` (the
/// derivation's environment at the time its summary is validated); `None`
/// when it states none.
pub(crate) fn published_effect(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Option<MemberEffect> {
    let (body_start, _) = super::find_function_body(tokens, function)?;
    let parameters = super::function_parameters(tokens, function).unwrap_or_default();
    let reading = BodyReading::read(tokens, function, body_start, &parameters, contracts);
    (!reading.effect.is_empty()).then_some(reading.effect)
}

/// Whether a node's type spelling is free of the qualifiers no member read
/// here may carry.
fn plain(spelling: &str) -> bool {
    !spelling.contains("volatile") && !spelling.contains("_Atomic")
}

impl Parser<'_> {
    /// Card MEMBER-EFFECT-COMPONENT-SCOPE (rule 38j amended): whether Clang
    /// reads the actual `B -> F` (`argument`, at `index` of the one call of
    /// `callee` with `arity` actuals on `line`) as the **value** of a pointer
    /// member `F` of the object the formal `B` points to.
    ///
    /// Asked by the call-argument rule only where the lexical recogniser
    /// refused the actual as an untyped member (a member spelled through a
    /// pointer typedef, `cfl_sds_t ns;`, which the declaration reader leaves
    /// unresolved), at a nullable release position, for a place rule 5k's or
    /// rule 38j's lexical check admitted. `true` is exactly the fact the
    /// lexical reader's `pointer_member` gives a `*`-spelled member: the
    /// actual is one pointer value read from a member of `*B`. The value read
    /// is required (`LValueToRValue`, never `ArrayToPointerDecay`, which would
    /// hand an interior address to the release), the member's type an object
    /// pointer with no `volatile`/`_Atomic`, and the base a plain parameter
    /// reference that is not a pointer to a `union`.
    ///
    /// Card MEMBER-RELEASE-UNTYPED-BASE (rule 38j amended again): with
    /// `untyped_base`, asked where the lexical recogniser refused the actual's
    /// base instead (a formal spelled through a typedef of a record or of a
    /// record pointer, which the declaration reader does not type, so it never
    /// classified the member either). Every condition above holds, and Clang
    /// must also read the base: its type (through any typedef chain, which
    /// Clang has already resolved for `->`) a pointer with no
    /// `volatile`/`_Atomic` and no function type, and the member Clang resolved
    /// (`referencedMemberDecl`) a member named `F` of a complete `struct` the
    /// unit defines, which no `union` member and no incomplete record is.
    pub(super) fn typed_member_release(
        &self,
        callee: &str,
        index: usize,
        arity: usize,
        argument: &[&str],
        line: usize,
        untyped_base: bool,
    ) -> bool {
        use crate::typed_access_path::qual;
        use crate::typed_c_ast::{children, kind};
        let [base, "-", ">", field] = strip_parentheses(argument) else {
            return false;
        };
        let Some(statements) = self.typed_positions_index() else {
            return false;
        };
        let Some(mut node) = statements.argument_node(callee, index, arity, argument, line) else {
            return false;
        };
        let mut value = false;
        loop {
            match kind(node) {
                "ParenExpr" => {}
                "ImplicitCastExpr" if node["castKind"].as_str() == Some("LValueToRValue") => {
                    value = true;
                }
                // Card MEMBER-RELEASE-UNTYPED-BASE: the implicit conversion
                // of the value read to the callee's parameter type (`char *`
                // to `void *` for `free`, `rd_free`, `mk_mem_free`): a
                // conversion between object pointers that keeps the value,
                // never an explicit cast, and only outside the read.
                "ImplicitCastExpr"
                    if !value
                        && matches!(node["castKind"].as_str(), Some("BitCast" | "NoOp"))
                        && qual(node).is_some_and(|ty| {
                            let ty = ty.trim_end();
                            ty.ends_with('*') && !ty.contains('(') && plain(ty)
                        }) => {}
                _ => break,
            }
            let Some(inner) = children(node).first() else {
                return false;
            };
            node = inner;
        }
        if !value
            || kind(node) != "MemberExpr"
            || node["isArrow"].as_bool() != Some(true)
            || node["name"].as_str() != Some(*field)
        {
            return false;
        }
        let Some(member) = qual(node) else {
            return false;
        };
        let member = member.trim_end();
        if !member.ends_with('*') || member.contains('(') || !plain(member) {
            return false;
        }
        let Some(mut reference) = children(node).first() else {
            return false;
        };
        let Some(pointer) = qual(reference) else {
            return false;
        };
        if pointer.contains("union ") || !plain(pointer) {
            return false;
        }
        if untyped_base {
            let pointer = pointer.trim_end();
            let in_struct = node["referencedMemberDecl"]
                .as_str()
                .and_then(|id| statements.records.struct_members.get(id))
                .is_some_and(|(_, name)| name == field);
            if !pointer.ends_with('*') || pointer.contains('(') || !in_struct {
                return false;
            }
        }
        loop {
            match kind(reference) {
                "ParenExpr" => {}
                "ImplicitCastExpr"
                    if matches!(
                        reference["castKind"].as_str(),
                        Some("LValueToRValue" | "NoOp")
                    ) => {}
                _ => break,
            }
            let Some(inner) = children(reference).first() else {
                return false;
            };
            reference = inner;
        }
        // Card MEMBER-RELEASE-UNTYPED-BASE: the rvalue read above drops the
        // parameter's own qualifiers, so they are read on its reference (a
        // `volatile` or `_Atomic` formal is never this rule's base).
        kind(reference) == "DeclRefExpr"
            && reference["referencedDecl"]["kind"].as_str() == Some("ParmVarDecl")
            && reference["referencedDecl"]["name"].as_str() == Some(*base)
            && [qual(reference), reference["type"]["qualType"].as_str()]
                .into_iter()
                .all(|ty| ty.is_some_and(plain))
    }

    /// Rule 38j's direct-call route: `instruction`, the lowering of a
    /// whole-statement call to a function whose member-effect summary is in
    /// force, refused as an uncontracted call. `Some` is the call lowered under
    /// the summary's contract when this body's check admitted it as an event;
    /// otherwise the refusal under [`call_refused`].
    pub(super) fn member_effect_call(
        &mut self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        let LegacyInstruction::Unknown {
            description,
            writes,
        } = instruction
        else {
            return None;
        };
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let [callee, "(", .., ")"] = texts.as_slice() else {
            return None;
        };
        if *description != super::uncontracted_access::uncontracted_refusal(callee)
            || self.contracts.member_effect(callee).is_none()
        {
            return None;
        }
        let refusal = || {
            Statement::Instruction(
                LegacyInstruction::Unknown {
                    description: call_refused(callee),
                    writes: writes.clone(),
                },
                line,
            )
        };
        let callee = (*callee).to_owned();
        if !self
            .member_effect_reading
            .calls
            .contains(&(callee.clone(), tokens[0].line))
        {
            return Some(refusal());
        }
        let mut synthetic = tokens.to_vec();
        synthetic[0].text = minted_name(&callee);
        let saved = self.probe_state();
        let classified = self.classify_simple(&synthetic);
        let sequence = match classified {
            LegacyInstruction::ContractCall { .. } => vec![classified],
            LegacyInstruction::Unknown {
                ref description, ..
            } if description == super::call_argument_access::GATE_REFUSAL => {
                match self.call_argument_access(&synthetic, &classified) {
                    Some(sequence)
                        if matches!(
                            sequence.last(),
                            Some(LegacyInstruction::ContractCall { .. })
                        ) =>
                    {
                        sequence
                    }
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        };
        if sequence.is_empty() {
            self.restore_probe_state(&saved);
            return Some(refusal());
        }
        let mut statements = Vec::with_capacity(sequence.len());
        for mut instruction in sequence {
            if let LegacyInstruction::ContractCall { function, .. } = &mut instruction
                && *function == minted_name(&callee)
            {
                function.clone_from(&callee);
            }
            statements.push(Statement::Instruction(instruction, line));
        }
        Some(match statements.len() {
            1 => statements.pop().expect("one statement"),
            _ => Statement::Block(statements),
        })
    }
}
