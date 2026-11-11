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

//! Card **OP-CALLBACK-MEMBER-PATH-RELEASE**: the state a call leaves at a
//! member path of a formal's object (contract specification §C04.19.6,
//! grammar G05d, gateway rule 30), read total-or-reject like every other
//! clause.
//!
//! ```text
//! quant live #rko: rd_kafka_op_s.rko_u.leaders.eonce |-> #released when #result != #KEEP;
//! quant live #rko: rd_kafka_op_s.rko_u.leaders.partitions |-> {#moved, #kept} when #result != #KEEP;
//! quant live #reply: rd_kafka_op_s.rko_u.leaders.opaque |-> #stale;
//! ```
//!
//! USER DECISION 2026-10-15 (verbatim in the card's report): "B: state +
//! set" and "Build it now". The value keywords are `#released`, `#moved`,
//! `#stale` and `#kept`; the set form is exactly `{V, #kept}` with `V` one of
//! the first three (any other set rejects); the guard is absent or the
//! complement of the declaration's hand-off guard; a path the declaration
//! does not name is kept; no move destination is stated.

use std::collections::BTreeMap;

use qcore_frontend::lexer::{MAPS_TO, Symbol, TokenKind, lex};
use qcore_schemas::callback::CallbackMember;
use qcore_schemas::contract::{ResourceFunctionContract, ResourceParameterMode};
use qcore_schemas::handoff::{GuardOp, HandOff, HandOffGuard, RESERVED_KEYWORDS};
use qcore_schemas::member_path::{
    Fate, GuardConstant, KEPT, MemberPathState, ParameterGuard, ParameterOp, PathState, complements,
};

/// The guarded value's binder.
const RESULT: &str = "result";
/// The hand-off's owner keyword (a `#P: #owner …` clause is a hand-off).
const OWNER: &str = "owner";

/// One member-path clause as written, its guard's constant canonical (an
/// alias resolved to its `@` name).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Written {
    /// The binder's formal, without its `#`.
    pub parameter: String,
    pub member: CallbackMember,
    pub state: PathState,
    pub guard: Option<HandOffGuard>,
    /// Card FB-TEARDOWN-DESTROY-DECLARATIONS (decision 2): `when #PARAM OP
    /// CONST`, the parameter by name, before the declaration is known.
    pub parameter_guard: Option<WrittenParameterGuard>,
    /// The clause, for diagnostics.
    pub text: String,
}

/// `when #PARAM OP CONST` as written, its alias resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WrittenParameterGuard {
    pub name: String,
    pub op: ParameterOp,
    pub constant: GuardConstant,
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with(|ch: char| ch.is_ascii_digit())
        && text
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn tokens(source: &str) -> Option<Vec<TokenKind>> {
    Some(
        lex(source)
            .ok()?
            .into_iter()
            .map(|token| token.kind)
            .filter(|kind| *kind != TokenKind::Eof)
            .collect(),
    )
}

/// Whether the text after `quant live` binds a formal's member path
/// (`#P: WORD…`), as opposed to a hand-off (`#P: #owner …`).
pub(super) fn is_member_path(source: &str) -> bool {
    let Some(tokens) = tokens(source) else {
        return false;
    };
    let binds = matches!(
        (tokens.first(), tokens.get(1), tokens.get(2)),
        (
            Some(TokenKind::Symbol(Symbol::Hash)),
            Some(TokenKind::Word(binder)),
            Some(TokenKind::Symbol(Symbol::Colon))
        ) if binder != RESULT
    );
    let hand_off = matches!(
        (tokens.get(3), tokens.get(4)),
        (Some(TokenKind::Symbol(Symbol::Hash)), Some(TokenKind::Word(word))) if word == OWNER
    );
    binds && !hand_off
}

/// Read the text after `quant live` of a member-path state:
/// `#P: TAG.PATH |-> V [when #result (== | !=) (@NAME | #ALIAS)]` with `V`
/// one of `#released`, `#moved`, `#stale`, `#kept` or a set `{V, #kept}`.
/// Every other shape rejects the module.
pub(super) fn quant_live_clause(
    source: &str,
    aliases: &BTreeMap<String, String>,
) -> Result<Written, String> {
    let shape = || {
        format!(
            "quant-live-path-shape: a member-path state is `quant live #P: TAG.PATH {MAPS_TO} \
             #released;` (or `#moved`, `#stale`, `#kept`, or `{{V, #kept}}`), optionally `when \
             #result OP @NAME`, not `quant live {source};`"
        )
    };
    let tokens = tokens(source).ok_or_else(shape)?;
    let word = |at: usize| match tokens.get(at) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    };
    let symbol = |at: usize, expected: Symbol| tokens.get(at) == Some(&TokenKind::Symbol(expected));
    if !symbol(0, Symbol::Hash) || !symbol(2, Symbol::Colon) {
        return Err(shape());
    }
    let parameter = word(1)
        .filter(|name| is_identifier(name))
        .ok_or_else(shape)?;
    if RESERVED_KEYWORDS.contains(&parameter) {
        return Err(format!(
            "quant-live-path-binder: `#{parameter}` is a reserved keyword, not a formal's binder"
        ));
    }
    // `TAG . SEG (. SEG)*`
    let mut at = 3;
    let mut segments = Vec::new();
    loop {
        let segment = word(at)
            .filter(|segment| is_identifier(segment))
            .ok_or_else(|| {
                format!(
                    "quant-live-path-member: the left of `{MAPS_TO}` is a dotted `TAG.FIELD`: {}",
                    shape()
                )
            })?;
        segments.push(segment.to_owned());
        at += 1;
        if symbol(at, Symbol::Dot) {
            at += 1;
            continue;
        }
        break;
    }
    if segments.len() < 2 {
        return Err(format!(
            "quant-live-path-member: the left of `{MAPS_TO}` is a dotted `TAG.FIELD`: {}",
            shape()
        ));
    }
    if symbol(at, Symbol::Arrow) {
        return Err(format!(
            "quant-live-maps-to: `->` is C member access, not maps-to: write `TAG.PATH {MAPS_TO} \
             #released` (`{MAPS_TO}` is the ASCII `↦`)"
        ));
    }
    if !symbol(at, Symbol::MapsTo) {
        return Err(shape());
    }
    at += 1;
    let value_error = |found: &str| {
        format!(
            "quant-live-path-value: a member path maps to `#released`, `#moved`, `#stale` or \
             `#{KEPT}` (or `{{V, #{KEPT}}}`), not `{found}`"
        )
    };
    let keyword = |at: usize| -> Result<Fate, String> {
        if !symbol(at, Symbol::Hash) {
            return Err(value_error(word(at).unwrap_or("?")));
        }
        let name = word(at + 1).ok_or_else(|| value_error("#?"))?;
        Fate::from_word(name).ok_or_else(|| value_error(&format!("#{name}")))
    };
    let state = if symbol(at, Symbol::LeftBrace) {
        let set_error = || {
            format!(
                "quant-live-path-set: a set is exactly `{{V, #{KEPT}}}` with `V` one of \
                 `#released`, `#moved`, `#stale`: {}",
                shape()
            )
        };
        let first = keyword(at + 1).map_err(|_| set_error())?;
        if !symbol(at + 3, Symbol::Comma) {
            return Err(set_error());
        }
        let second = keyword(at + 4).map_err(|_| set_error())?;
        if !symbol(at + 6, Symbol::RightBrace) || first == Fate::Kept || second != Fate::Kept {
            return Err(set_error());
        }
        at += 7;
        PathState {
            fate: first,
            maybe: true,
        }
    } else {
        let fate = keyword(at)?;
        at += 2;
        PathState { fate, maybe: false }
    };
    let guard_shape = || {
        format!(
            "quant-live-path-guard: a member-path state is guarded by `when #{RESULT} OP @NAME` \
             beside a hand-off, by `when #PARAM OP CONST` on an integer parameter, or not at \
             all: {}",
            shape()
        )
    };
    let mut guard = None;
    let mut parameter_guard = None;
    if at != tokens.len() {
        // `when # WORD OP CONSTANT`, then the end.
        if word(at) != Some("when") || !symbol(at + 1, Symbol::Hash) {
            return Err(guard_shape());
        }
        let binder = word(at + 2)
            .filter(|name| is_identifier(name))
            .ok_or_else(guard_shape)?;
        let op = match tokens.get(at + 3) {
            Some(TokenKind::Symbol(Symbol::EqualEqual)) => ParameterOp::Eq,
            Some(TokenKind::Symbol(Symbol::NotEqual)) => ParameterOp::Ne,
            Some(TokenKind::Symbol(Symbol::Less)) => ParameterOp::Lt,
            Some(TokenKind::Symbol(Symbol::Greater)) => ParameterOp::Gt,
            Some(TokenKind::Symbol(Symbol::LessEqual)) => ParameterOp::Le,
            Some(TokenKind::Symbol(Symbol::GreaterEqual)) => ParameterOp::Ge,
            _ => return Err(guard_shape()),
        };
        if binder == RESULT {
            // The hand-off complement: `#result (== | !=) (@NAME | #ALIAS)`.
            let op = match op {
                ParameterOp::Eq => GuardOp::Eq,
                ParameterOp::Ne => GuardOp::Ne,
                _ => {
                    return Err(format!(
                        "quant-live-path-guard: the guard compares with `==` or `!=` only: {}",
                        shape()
                    ));
                }
            };
            if !(symbol(at + 4, Symbol::At) || symbol(at + 4, Symbol::Hash))
                || tokens.len() != at + 6
            {
                return Err(format!(
                    "quant-live-path-guard: the guard's constant is `@NAME` or a module alias \
                     `#NAME`: {}",
                    shape()
                ));
            }
            let name = word(at + 5)
                .filter(|name| is_identifier(name))
                .ok_or_else(shape)?;
            let constant = if symbol(at + 4, Symbol::At) {
                name.to_owned()
            } else {
                aliases.get(name).cloned().ok_or_else(|| {
                    format!(
                        "quant-live-handoff-alias: `#{name}` is not an alias this module declares"
                    )
                })?
            };
            guard = Some(HandOffGuard { op, constant });
        } else {
            if RESERVED_KEYWORDS.contains(&binder) {
                return Err(format!(
                    "quant-live-path-guard: `#{binder}` is a reserved keyword, not a parameter"
                ));
            }
            // Card FB-TEARDOWN-DESTROY-DECLARATIONS (decision 2): `#PARAM OP
            // CONST`, the constant a canonical decimal (an optional `-`), an
            // `@` enumerator or a module alias.
            let constant_error = || {
                format!(
                    "quant-live-path-guard-constant: a parameter guard's constant is a canonical \
                     decimal, `@NAME` or a module alias `#NAME`: {}",
                    shape()
                )
            };
            let (constant, end) = match (tokens.get(at + 4), tokens.get(at + 5)) {
                (Some(TokenKind::Symbol(Symbol::At)), Some(TokenKind::Word(name)))
                    if is_identifier(name) =>
                {
                    (GuardConstant::Name(name.clone()), at + 6)
                }
                (Some(TokenKind::Symbol(Symbol::Hash)), Some(TokenKind::Word(name)))
                    if is_identifier(name) =>
                {
                    let target = aliases.get(name.as_str()).cloned().ok_or_else(|| {
                        format!(
                            "quant-live-handoff-alias: `#{name}` is not an alias this module \
                             declares"
                        )
                    })?;
                    (GuardConstant::Name(target), at + 6)
                }
                (Some(TokenKind::Integer(digits)), _) => (
                    GuardConstant::Value(canonical(digits, false).ok_or_else(constant_error)?),
                    at + 5,
                ),
                (Some(TokenKind::Symbol(Symbol::Minus)), Some(TokenKind::Integer(digits))) => (
                    GuardConstant::Value(canonical(digits, true).ok_or_else(constant_error)?),
                    at + 6,
                ),
                _ => return Err(constant_error()),
            };
            if tokens.len() != end {
                return Err(guard_shape());
            }
            parameter_guard = Some(WrittenParameterGuard {
                name: binder.to_owned(),
                op,
                constant,
            });
        }
    }
    let tag = segments.remove(0);
    Ok(Written {
        parameter: parameter.to_owned(),
        member: CallbackMember {
            tag,
            path: segments.join("."),
        },
        state,
        guard,
        parameter_guard,
        text: format!("quant live {source}"),
    })
}

/// A canonical decimal (no leading zero, no suffix, no `-0`), negated when
/// `negative`.
fn canonical(digits: &str, negative: bool) -> Option<i128> {
    if digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
        || (negative && digits == "0")
    {
        return None;
    }
    let value: i128 = digits.parse().ok()?;
    Some(if negative { -value } else { value })
}

/// Validate one declaration's member-path clauses against its loaded custody
/// contract and hand-off: each binder a depth-one pointer formal stated
/// `resource_borrowed` or `resource_released` with no kind, or the formal the
/// hand-off hands (`resource_owned(P, nonnull)`); one state per path and no
/// path below another of the same formal; a result guard only beside a
/// hand-off, its exact complement; a parameter guard (card
/// FB-TEARDOWN-DESTROY-DECLARATIONS) on a plain integer parameter other than
/// the binder, a decimal constant in its range that partitions it.
///
/// # Errors
/// The module's rejection.
pub(super) fn declaration(
    written: &[Written],
    contract: &ResourceFunctionContract,
    parameter_types: &[String],
    names: &BTreeMap<&str, usize>,
    handoff: Option<&HandOff>,
) -> Result<Vec<MemberPathState>, String> {
    let mut states: Vec<MemberPathState> = Vec::new();
    for clause in written {
        let at = names
            .get(clause.parameter.as_str())
            .copied()
            .ok_or_else(|| {
                format!(
                    "quant-live-path-binder: `{}`: `#{}` names no parameter",
                    clause.text, clause.parameter
                )
            })?;
        if !matches!(
            parameter_types[at].as_str(),
            "ptr<mut, u8>" | "ptr<const, u8>"
        ) {
            return Err(format!(
                "quant-live-path-binder: `{}`: the formal is a depth-one pointer",
                clause.text
            ));
        }
        let handed = handoff.is_some_and(|handoff| handoff.parameter == at);
        // Card FB-TEARDOWN-DESTROY-DECLARATIONS: a destroyer's own object
        // (`resource_released`) is a binder too; its states are what the
        // release does to the objects its members held.
        let mode_ok = match contract.parameters[at] {
            ResourceParameterMode::Borrowed | ResourceParameterMode::Released { .. } => true,
            ResourceParameterMode::Owned { nullable: false } => handed,
            _ => false,
        };
        if !mode_ok || contract.parameter_kinds.contains_key(&at) {
            return Err(format!(
                "quant-live-path-binder: `{}`: the formal is `resource_borrowed` or \
                 `resource_released` with no kind, or the formal a hand-off hands \
                 (`resource_owned({}, nonnull)`)",
                clause.text, clause.parameter
            ));
        }
        let parameter_guard = match &clause.parameter_guard {
            None => None,
            Some(written) => {
                let tested = names.get(written.name.as_str()).copied().ok_or_else(|| {
                    format!(
                        "quant-live-path-guard: `{}`: `#{}` names no parameter",
                        clause.text, written.name
                    )
                })?;
                let (low, high) = super::retained_by::guard_range(&parameter_types[tested])
                    .filter(|_| tested != at)
                    .ok_or_else(|| {
                        format!(
                            "quant-live-path-guard: `{}`: `#{}` is not a plain integer parameter \
                             other than the binder (`i8`-`i64`, `isize`, `u8`-`u64`, `usize`)",
                            clause.text, written.name
                        )
                    })?;
                if let GuardConstant::Value(value) = written.constant
                    && !written.op.partitions(value, low, high)
                {
                    return Err(format!(
                        "quant-live-path-guard: `{}`: the constant lies outside `#{}`'s range or \
                         the guard does not partition it",
                        clause.text, written.name
                    ));
                }
                Some(ParameterGuard {
                    parameter: tested,
                    op: written.op,
                    constant: written.constant.clone(),
                    low,
                    high,
                })
            }
        };
        if let Some(guard) = &clause.guard {
            let Some(handoff) = handoff.filter(|_| handed) else {
                return Err(format!(
                    "quant-live-path-guard: `{}`: a member-path state is guarded only beside a \
                     hand-off of the same formal, by the complement of its guard",
                    clause.text
                ));
            };
            if !complements(guard, &handoff.guard) {
                return Err(format!(
                    "quant-live-path-guard: `{}`: the guard is the complement of the hand-off's \
                     `{}`",
                    clause.text,
                    handoff.guard.spelling()
                ));
            }
        }
        let path = clause.member.path.as_str();
        if let Some(other) = states.iter().find(|state| {
            state.parameter == at
                && (state.member.path == path
                    || state.member.path.starts_with(&format!("{path}."))
                    || path.starts_with(&format!("{}.", state.member.path)))
        }) {
            return Err(format!(
                "quant-live-path-duplicate: `{}`: `{}` is already stated by `{}`",
                clause.text,
                clause.member.spelling(),
                other.member.spelling()
            ));
        }
        states.push(MemberPathState {
            parameter: at,
            member: clause.member.clone(),
            state: clause.state,
            guard: clause.guard.clone(),
            parameter_guard,
        });
    }
    Ok(states)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aliases() -> BTreeMap<String, String> {
        BTreeMap::from([("KEEP".to_owned(), "RD_KAFKA_OP_RES_KEEP".to_owned())])
    }

    #[test]
    fn the_four_values_and_the_set_read() {
        for (value, fate, maybe) in [
            ("#released", Fate::Released, false),
            ("#moved", Fate::Moved, false),
            ("#stale", Fate::Stale, false),
            ("#kept", Fate::Kept, false),
            ("{#moved, #kept}", Fate::Moved, true),
            ("{#released, #kept}", Fate::Released, true),
            ("{#stale, #kept}", Fate::Stale, true),
        ] {
            let text = format!("#rko: rd_kafka_op_s.rko_u.leaders.partitions {MAPS_TO} {value}");
            assert!(is_member_path(&text), "{text}");
            let read = quant_live_clause(&text, &aliases()).expect(&text);
            assert_eq!(read.state, PathState { fate, maybe }, "{text}");
            assert_eq!(read.member.tag, "rd_kafka_op_s");
            assert_eq!(read.member.path, "rko_u.leaders.partitions");
            assert_eq!(read.guard, None);
        }
        let guarded = quant_live_clause(
            &format!("#rko: t.a.b {MAPS_TO} #released when #result != #KEEP"),
            &aliases(),
        )
        .expect("reads");
        let direct = quant_live_clause(
            &format!("#rko: t.a.b {MAPS_TO} #released when #result != @RD_KAFKA_OP_RES_KEEP"),
            &aliases(),
        )
        .expect("reads");
        assert_eq!(guarded.guard, direct.guard);
        assert_eq!(guarded.guard.map(|g| g.op), Some(GuardOp::Ne));
    }

    #[test]
    fn every_other_shape_rejects_with_a_named_diagnostic() {
        let cases = [
            ("#rko: t.a {M} {#kept, #moved}", "quant-live-path-set:"),
            ("#rko: t.a {M} {#kept, #kept}", "quant-live-path-set:"),
            ("#rko: t.a {M} {#moved, #stale}", "quant-live-path-set:"),
            ("#rko: t.a {M} {#moved}", "quant-live-path-set:"),
            (
                "#rko: t.a {M} {#moved, #kept, #kept}",
                "quant-live-path-set:",
            ),
            ("#rko: t.a {M} #absent", "quant-live-path-value:"),
            ("#rko: t.a {M} #dropped", "quant-live-path-value:"),
            ("#rko: t.a {M} released", "quant-live-path-value:"),
            ("#rko: t {M} #moved", "quant-live-path-member:"),
            ("#rko: t.a -> #moved", "quant-live-maps-to:"),
            (
                "#rko: t.a {M} #moved when #result < @K",
                "quant-live-path-guard:",
            ),
            (
                "#rko: t.a {M} #moved when result != @K",
                "quant-live-path-guard:",
            ),
            (
                "#rko: t.a {M} #moved when #result != 0",
                "quant-live-path-guard:",
            ),
            (
                "#rko: t.a {M} #moved when #result != #NOPE",
                "quant-live-handoff-alias:",
            ),
            ("#moved: t.a {M} #moved", "quant-live-path-binder:"),
        ];
        for (text, diagnostic) in cases {
            let text = text.replace("{M}", MAPS_TO);
            let error = quant_live_clause(&text, &aliases()).expect_err(&text);
            assert!(error.starts_with(diagnostic), "{text}: {error}");
        }
    }

    #[test]
    fn a_parameter_guard_reads_every_operator_and_constant_form() {
        let mut aliases = aliases();
        aliases.insert("ON".to_owned(), "FLAG_ON".to_owned());
        for (guard, op, constant) in [
            ("#del != 0", ParameterOp::Ne, GuardConstant::Value(0)),
            ("#del == 1", ParameterOp::Eq, GuardConstant::Value(1)),
            ("#del < -2", ParameterOp::Lt, GuardConstant::Value(-2)),
            ("#del >= 3", ParameterOp::Ge, GuardConstant::Value(3)),
            ("#del <= 3", ParameterOp::Le, GuardConstant::Value(3)),
            ("#del > 3", ParameterOp::Gt, GuardConstant::Value(3)),
            (
                "#del == @FLAG_ON",
                ParameterOp::Eq,
                GuardConstant::Name("FLAG_ON".into()),
            ),
            (
                "#del != #ON",
                ParameterOp::Ne,
                GuardConstant::Name("FLAG_ON".into()),
            ),
        ] {
            let text = format!("#task: flb_task.ic {MAPS_TO} #released when {guard}");
            let read = quant_live_clause(&text, &aliases).expect(&text);
            assert_eq!(read.guard, None, "{text}");
            assert_eq!(
                read.parameter_guard,
                Some(WrittenParameterGuard {
                    name: "del".into(),
                    op,
                    constant
                }),
                "{text}"
            );
        }
        for (guard, diagnostic) in [
            ("#del != 00", "quant-live-path-guard-constant:"),
            ("#del != -0", "quant-live-path-guard-constant:"),
            ("#del != del", "quant-live-path-guard-constant:"),
            ("#del != 1 2", "quant-live-path-guard:"),
            ("#del ! 1", "quant-live-path-guard:"),
            ("#moved != 1", "quant-live-path-guard:"),
            ("#del != #NOPE", "quant-live-handoff-alias:"),
        ] {
            let text = format!("#task: flb_task.ic {MAPS_TO} #released when {guard}");
            let error = quant_live_clause(&text, &aliases).expect_err(&text);
            assert!(error.starts_with(diagnostic), "{text}: {error}");
        }
    }

    #[test]
    fn a_member_path_is_told_from_a_hand_off_and_a_result_state() {
        assert!(is_member_path("#rko: t.a |-> #moved"));
        assert!(!is_member_path(
            "#rko: #owner |-> #callback when #result != @K"
        ));
        assert!(!is_member_path("#result: t.a |-> #absent"));
        assert!(!is_member_path("#rko"));
    }
}
