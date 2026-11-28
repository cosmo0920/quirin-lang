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

//! Card **OUT-SLOT-MEMBER-THEN-FRESH** (gateway rule 96): the read-and-null
//! move-out of a member.
//!
//! USER DECISION 2026-11-28: option (a1)+(b). The pair
//!
//! ```text
//! DESTINATION = BASE->MEMBER;      // DESTINATION a name or `*NAME`
//! BASE->MEMBER = NULL;             // the very next statement
//! ```
//!
//! hands the member's object to `DESTINATION` when the flow decides what the
//! member holds, and is exactly the read's own lowering everywhere else.
//! Two cases decide ([`decide`]):
//!
//! 1. **Linked.** `BASE` resolves to one live aggregate this body holds and
//!    its field link at `MEMBER` (a single name) holds exactly one live
//!    child on every arriving path, which the place binding `BASE->MEMBER`
//!    names alone: `DESTINATION` aliases that child. The next statement's
//!    null store then drops the link and the binding (rule 10v / O04-B08's
//!    own store), so the child is the destination's alone. This is the read
//!    CARRIED-CHILD-TEARDOWN-AFTER-UNLINK asked for in a narrower place: a
//!    bound read of a link that holds (O04-READ-BOUND's premise), taken at a
//!    read followed by the clear, not at a release position.
//! 2. **Stated.** No link at `MEMBER`, and the object's path state carries
//!    an owned member at `MEMBER` (`resource_member_owned` under the decided
//!    discriminant, recorded at the constructor call): `DESTINATION` takes a
//!    fresh possibly-null site, and the state is spent, so a second move-out
//!    on the path reads the fallback. The lowering admits this case only in
//!    a body that writes `MEMBER` nowhere but in the null stores of its own
//!    move-outs and never takes its address ([`stated_admitted`]),
//!    and the flow spends the state of any object handed to a call
//!    ([`spend_at_call`]).
//!
//! `DESTINATION` `*NAME` is admitted only where the slot is sanctioned (an
//! out slot the run reads, as rule 74l's store); anything else, a partial
//! link, a nullable or caller-held base, and every other shape read the
//! fallback, so no verdict another rule gives changes outside the pair.
//!
//! The Lean model is `QCore.MemberMoveOut`.

use qcore_schemas::contract::ResourceContracts;

use super::LIVE;
use super::constructed_route::ObjectState;
use super::transfer::{AbstractPointer, AbstractState, TransferContext};
use super::{LegacyInstruction, Parser, Statement, Token, is_null_constant};

/// The token texts with the stream's split `-` `>` fused into `->` (the
/// stream spells the arrow as one token or as two).
fn fused<'a>(tokens: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    let mut out: Vec<&'a str> = Vec::new();
    for text in tokens {
        if text == ">" && out.last() == Some(&"-") {
            out.pop();
            out.push("->");
        } else {
            out.push(text);
        }
    }
    out
}

/// `[NAME]` or `[*, NAME]`, spelled as the flow names the place.
fn destination(texts: &[&str]) -> Option<String> {
    match texts {
        [name] if crate::typed_field_access::is_name(name) => Some((*name).to_owned()),
        ["*", name] if crate::typed_field_access::is_name(name) => Some(format!("*{name}")),
        _ => None,
    }
}

/// `BASE -> M (. M)*` as `(BASE, "M.M")`.
fn member_place(texts: &[&str]) -> Option<(String, String)> {
    let [base, "->", rest @ ..] = texts else {
        return None;
    };
    if !crate::typed_field_access::is_name(base) || rest.is_empty() {
        return None;
    }
    let mut path = Vec::new();
    for (at, text) in rest.iter().enumerate() {
        if at % 2 == 0 {
            if !crate::typed_field_access::is_name(text) {
                return None;
            }
            path.push(*text);
        } else if *text != "." {
            return None;
        }
    }
    (rest.len() % 2 == 1).then(|| ((*base).to_owned(), path.join(".")))
}

/// The tokens of the statement starting at `at`, to its `;` at depth 0.
fn next_statement(tokens: &[Token], at: usize) -> Option<Vec<&str>> {
    let mut depth = 0usize;
    let mut out = Vec::new();
    for token in tokens.get(at..)? {
        match token.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.checked_sub(1)?,
            ";" if depth == 0 => return Some(out),
            _ => {}
        }
        out.push(token.text.as_str());
    }
    None
}

/// [`next_statement`] with the arrow fused.
fn next_statement_fused(tokens: &[Token], at: usize) -> Option<Vec<&str>> {
    next_statement(tokens, at).map(fused)
}

/// The member spelled `M (. M)*` at `texts[at..]` after a `->`, its length.
fn member_at(texts: &[&str], at: usize, path: &[&str]) -> bool {
    let mut cursor = at;
    for (index, segment) in path.iter().enumerate() {
        if index > 0 {
            if texts.get(cursor) != Some(&".") {
                return false;
            }
            cursor += 1;
        }
        if texts.get(cursor) != Some(segment) {
            return false;
        }
        cursor += 1;
    }
    // The member ends there: no deeper selection.
    !matches!(texts.get(cursor), Some(&".") | Some(&"->") | Some(&"["))
}

/// Whether the body writes `-> PATH` only as `B -> PATH = NULL ;` right
/// after a read `D = B -> PATH ;`, and never takes `& … -> PATH`.
fn stated_admitted(tokens: &[Token], path: &str) -> bool {
    let texts = fused(tokens.iter().map(|t| t.text.as_str()));
    let segments = path.split('.').collect::<Vec<_>>();
    let width = segments.len() * 2 - 1;
    for at in 0..texts.len() {
        if texts[at] != "->" || !member_at(&texts, at + 1, &segments) {
            continue;
        }
        let after = at + 1 + width;
        // An address of the member, or of something below it.
        let mut start = at;
        while start > 0 && crate::typed_field_access::is_name(texts[start - 1]) {
            start -= 1;
        }
        if start > 0 && texts[start - 1] == "&" {
            return false;
        }
        let assigned = texts.get(after) == Some(&"=") && texts.get(after + 1) != Some(&"=");
        let compound = matches!(
            texts.get(after).copied(),
            Some("+=" | "-=" | "|=" | "&=" | "++" | "--")
        );
        if compound {
            return false;
        }
        if !assigned {
            continue;
        }
        // A write: it must be `B -> PATH = NULL ;` preceded by `D = B -> PATH ;`.
        let Some(end) = texts[after..].iter().position(|t| *t == ";") else {
            return false;
        };
        if !is_null_constant(&texts[after + 1..after + end]) || at == 0 {
            return false;
        }
        let base = texts[at - 1];
        let statement_start = at - 1;
        if statement_start == 0 || texts[statement_start - 1] != ";" {
            return false;
        }
        let read_end = statement_start - 1;
        let read_start = texts[..read_end]
            .iter()
            .rposition(|t| matches!(*t, ";" | "{" | "}"))
            .map_or(0, |at| at + 1);
        let read = &texts[read_start..read_end];
        let Some(equals) = read.iter().position(|t| *t == "=") else {
            return false;
        };
        if destination(&read[..equals]).is_none()
            || member_place(&read[equals + 1..]) != Some((base.to_owned(), path.to_owned()))
        {
            return false;
        }
    }
    true
}

impl Parser<'_> {
    /// Whether some declaration in force states an owned member at `path`.
    fn states_owned_member(&self, path: &str) -> bool {
        states_owned_member(self.contracts, path)
    }

    /// Rule 96's lowering: `statement`, the read `DESTINATION = BASE->MEMBER;`
    /// the next statement follows with `BASE->MEMBER = NULL;`, as a
    /// [`LegacyInstruction::MoveOut`] over its own lowering; every other
    /// statement unchanged.
    pub(super) fn move_out_statement(
        &mut self,
        tokens: &[Token],
        line: usize,
        statement: Statement,
    ) -> Statement {
        let Statement::Instruction(instruction, at_line) = &statement else {
            return statement;
        };
        if matches!(instruction, LegacyInstruction::MoveOut { .. }) {
            return statement;
        }
        let texts = fused(tokens.iter().map(|t| t.text.as_str()));
        let Some(equals) = texts.iter().position(|t| *t == "=") else {
            return statement;
        };
        let (Some(destination), Some((base, member))) = (
            destination(&texts[..equals]),
            member_place(&texts[equals + 1..]),
        ) else {
            return statement;
        };
        if destination == base || destination == format!("*{base}") {
            return statement;
        }
        let Some(next) = next_statement_fused(self.tokens, self.cursor) else {
            return statement;
        };
        let Some(next_equals) = next.iter().position(|t| *t == "=") else {
            return statement;
        };
        if member_place(&next[..next_equals]) != Some((base.clone(), member.clone()))
            || next.get(next_equals + 1) == Some(&"=")
            || !is_null_constant(&next[next_equals + 1..])
        {
            return statement;
        }
        let stated = self.states_owned_member(&member) && stated_admitted(self.tokens, &member);
        // The linked case needs a body link: a store of a name into the
        // single-name member somewhere before the read.
        let linked = !member.contains('.') && {
            let before = fused(self.tokens[..self.cursor].iter().map(|t| t.text.as_str()));
            before.windows(5).any(|window| {
                window[0] == base
                    && window[1] == "->"
                    && window[2] == member
                    && window[3] == "="
                    && crate::typed_field_access::is_name(window[4])
                    && !is_null_constant(&window[4..5])
            })
        };
        if !stated && !linked {
            return statement;
        }
        let site = self.next_site;
        self.next_site += 1;
        let line = *at_line.max(&line);
        let Statement::Instruction(fallback, _) = statement else {
            unreachable!("matched above")
        };
        Statement::Instruction(
            LegacyInstruction::MoveOut {
                destination,
                base,
                member,
                site,
                stated,
                fallback: Box::new(fallback),
            },
            line,
        )
    }
}

/// Whether some declaration in `contracts` states an owned member at `path`.
pub(crate) fn states_owned_member(contracts: &ResourceContracts, path: &str) -> bool {
    contracts.retaining().values().any(|declared| {
        declared
            .constructed
            .owned_members
            .iter()
            .any(|owned| owned.member.path == path)
    })
}

/// The site of a constructor's possibly-null result named by the plain
/// `destination`, for the states a declaration with an owned member states
/// (C04.19.4 as (a1) amends it): one live identity, possibly null.
pub(super) fn nullable_result_site(
    state: &AbstractState,
    destination: Option<&str>,
    constructed: &super::constructed_route::ConstructedOperands,
) -> Option<usize> {
    if constructed.keys.is_empty() {
        return None;
    }
    let place = destination.filter(|place| crate::typed_field_access::is_name(place))?;
    let pointer = state.pointers.get(place)?;
    if pointer.unknown || !pointer.derivation.is_identity() {
        return None;
    }
    let [site] = pointer.targets.iter().copied().collect::<Vec<_>>()[..] else {
        return None;
    };
    (state.resources.get(&site).copied().unwrap_or(0) & LIVE != 0
        && state.nullable_sites.contains(&site))
    .then_some(site)
}

/// What a decided move-out takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Taken {
    /// The child the body linked at the member.
    Linked(AbstractPointer),
    /// The stated owned member of the object at this site, under this key.
    Stated(usize, String),
}

fn path_of(key: &str) -> &str {
    key.split_once('.').map_or(key, |(_, path)| path)
}

/// The move-out's decision on `state`, `None` for the fallback.
pub(super) fn decide(
    state: &AbstractState,
    destination: &str,
    base: &str,
    member: &str,
    stated: bool,
    context: &TransferContext<'_>,
) -> Option<Taken> {
    // An unsanctioned slot reads the fallback (`reading`'s `sanctioned`).
    if destination.starts_with('*') && !context.sanctioned.contains(destination) {
        return None;
    }
    let owner = super::transfer::resolved_aggregate_owner(state, base, &context.caller_sites)?;
    // The link at a single-name member, classified as the vectors spell it.
    let place = format!("{base}->{member}");
    let link = match (!member.contains('.'))
        .then(|| {
            state
                .field_links
                .get(&owner)
                .and_then(|fields| fields.get(member))
        })
        .flatten()
    {
        None => "none",
        Some(children) => {
            let partial = state
                .partial_links
                .get(&owner)
                .and_then(|fields| fields.get(member))
                .is_some_and(|children| !children.is_empty());
            let one_live = children.len() == 1
                && state.pointers.get(&place).is_some_and(|pointer| {
                    !pointer.unknown
                        && pointer.derivation.is_identity()
                        && pointer.targets == *children
                })
                && children
                    .iter()
                    .all(|child| state.resources.get(child) == Some(&LIVE));
            match (partial, one_live) {
                (true, _) => "on-some-paths",
                (false, true) => "one-live",
                (false, false) => "several",
            }
        }
    };
    let owned = state.constructed.iter().find(|((site, key), value)| {
        *site == owner && path_of(key) == member && **value == ObjectState::OwnedMember
    });
    match reading(link, owned.is_some(), true, stated) {
        "alias" => Some(Taken::Linked(state.pointers.get(&place)?.clone())),
        "fresh-nullable" => owned.map(|((site, key), _)| Taken::Stated(*site, key.clone())),
        _ => None,
    }
}

/// Apply a decided move-out.
pub(super) fn apply(
    mut state: AbstractState,
    destination: &str,
    taken: Taken,
    site: usize,
) -> AbstractState {
    match taken {
        Taken::Linked(pointer) => {
            state.pointers.insert(destination.to_owned(), pointer);
        }
        Taken::Stated(owner, key) => {
            state.constructed.remove(&(owner, key));
            super::transfer::create_resource(&mut state, destination, site, true);
        }
    }
    state
}

/// The flow's guard on the stated case: an object handed to a call may have
/// its member rewritten there, so its owned-member states are spent.
pub(super) fn spend_at_call(state: &mut AbstractState, instruction: &LegacyInstruction) {
    if !state
        .constructed
        .values()
        .any(|value| *value == ObjectState::OwnedMember)
    {
        return;
    }
    let LegacyInstruction::ContractCall { arguments, .. } = instruction else {
        return;
    };
    let sites = arguments
        .iter()
        .flatten()
        .filter_map(|argument| state.pointers.get(argument))
        .flat_map(|pointer| pointer.targets.iter().copied())
        .collect::<std::collections::BTreeSet<_>>();
    state
        .constructed
        .retain(|(site, _), value| *value != ObjectState::OwnedMember || !sites.contains(site));
}

/// The refusal of a store into a member still holding its stated owned
/// object: the store would lose it.
pub(crate) fn owned_member_overwrite(place: &str) -> String {
    format!(
        "store into `{place}`, which still holds an owned member the store would lose \
         (resource_member_owned)"
    )
}

/// The flow's guard on a stated owned member: a store into the member while
/// its state is unspent (no move-out took the object first) loses an object
/// the declaration says the owner holds, so it is refused and the state
/// spent. Every other instruction is unchanged.
pub(super) fn guard_owned_overwrite(
    function: &str,
    line: usize,
    state: &mut AbstractState,
    instruction: &LegacyInstruction,
    diagnostics: &mut std::collections::BTreeSet<super::LegacyDiagnostic>,
) {
    if !state
        .constructed
        .values()
        .any(|value| *value == ObjectState::OwnedMember)
    {
        return;
    }
    let place = match instruction {
        LegacyInstruction::AggregateStore { place }
        | LegacyInstruction::Alias {
            destination: place, ..
        }
        | LegacyInstruction::Clear { destination: place }
        | LegacyInstruction::ContractCall {
            destination: Some(place),
            ..
        } => place.as_str(),
        _ => return,
    };
    let Some((base, member)) = place.split_once("->") else {
        return;
    };
    let Some(pointer) = state.pointers.get(base) else {
        return;
    };
    let sites = pointer.targets.clone();
    let held = state
        .constructed
        .iter()
        .filter(|((site, key), value)| {
            sites.contains(site) && path_of(key) == member && **value == ObjectState::OwnedMember
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    if held.is_empty() {
        return;
    }
    for key in held {
        state.constructed.remove(&key);
    }
    diagnostics.insert(super::LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: super::LegacyDiagnosticKind::Unsupported {
            description: owned_member_overwrite(place),
        },
    });
    state.unmodeled = true;
}

/// The decision table shared with `proof/QCore/MemberMoveOut.lean`
/// (`moveOutVectors`): `(link, stated state, sanctioned destination, admitted
/// body) -> reading`, where `link` is `"none"`, `"one-live"`, `"on-some-paths"` or
/// `"several"`, and the reading is `"alias"`, `"fresh-nullable"` or
/// `"fallback"`.
#[cfg(test)]
pub(crate) const MOVE_OUT_VECTORS: &[(&str, bool, bool, bool, &str)] = &[
    ("one-live", false, true, false, "alias"),
    ("one-live", true, true, true, "alias"),
    ("on-some-paths", true, true, true, "fallback"),
    ("several", false, true, false, "fallback"),
    ("none", true, true, true, "fresh-nullable"),
    ("none", true, true, false, "fallback"),
    ("none", false, true, true, "fallback"),
    ("none", true, false, true, "fallback"),
    ("one-live", false, false, false, "fallback"),
];

/// The reading [`decide`] takes, over the vector alphabet: `link` the
/// member's link, `state` whether an owned-member state is held, `sanctioned`
/// whether the destination may receive custody, `admitted` whether the body
/// admits the stated case.
pub(crate) fn reading(link: &str, state: bool, sanctioned: bool, admitted: bool) -> &'static str {
    if !sanctioned {
        return "fallback";
    }
    match link {
        "one-live" => "alias",
        "none" if state && admitted => "fresh-nullable",
        _ => "fallback",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vectors_hold() {
        for (link, state, sanctioned, admitted, expected) in MOVE_OUT_VECTORS {
            assert_eq!(
                reading(link, *state, *sanctioned, *admitted),
                *expected,
                "{link} {state} {sanctioned} {admitted}"
            );
        }
    }

    #[test]
    fn the_shapes_read() {
        assert_eq!(destination(&["p"]), Some("p".into()));
        assert_eq!(destination(&["*", "out"]), Some("*out".into()));
        assert_eq!(destination(&["a", "b"]), None);
        assert_eq!(
            member_place(&["rko", "->", "u", ".", "a"]),
            Some(("rko".into(), "u.a".into()))
        );
        assert_eq!(member_place(&["rko", "->", "u", "."]), None);
        assert_eq!(member_place(&["rko", "->", "u", "->", "a"]), None);
    }
}
