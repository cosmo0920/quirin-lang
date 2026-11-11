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

//! Card **OP-CONSTRUCTOR-DECLARATIONS**: a constructor's result state
//! (contract specification §C04.19.4, grammar G05, gateway rules 12o, 13w and
//! 14e), read total-or-reject like every other clause.
//!
//! ```text
//! ensures normal resource_discriminant_set(result, TAG.FIELD, P, FLAGS);
//! ensures normal resource_callback_member_registered(result, TAG.FIELD, F);
//! quant live #result: TAG.FIELD |-> #absent;    // two levels, rule 14e
//! quant live #result;                           // the liveness part alone
//! ensures normal resource_callback_member_registered(result, TAG.FIELD, #absent);
//! ```
//!
//! USER DECISIONS 2026-10-14: A1 (the four-operand discriminant, `FLAGS` a
//! canonical decimal OR-ed into the actual, exact), B1 (the member
//! registration), C (the `quant` state clause in maps-to notation, `|->` the
//! ASCII `↦`, [`qcore_frontend::lexer::MAPS_TO`]; `absent` reserved). The
//! shorthand `quant live #result: TAG.FIELD |-> #absent;` elaborates into the
//! canonical parts `quant live #result;` and the member state `#absent`; the
//! two levels give the same [`ConstructedState`]. Card
//! **SUPPLIED-OWNED-RETURN-CODE-HANDOFF** (USER DECISION 2026-10-15): a value
//! keyword carries `#`, so `#absent` is the one spelling at both levels and a
//! bare `absent` rejects the module naming `#absent`.

use std::collections::BTreeMap;

use qcore_frontend::lexer::{MAPS_TO, Symbol, TokenKind, lex};
use qcore_schemas::callback::CallbackMember;
use qcore_schemas::constructed_state::{
    ABSENT, ABSENT_WORD, ConstructedState, DiscriminantSet, MemberState, MemberValue,
};
use qcore_schemas::contract::{
    ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};

pub(super) const DISCRIMINANT_SET: &str = "resource_discriminant_set";
pub(super) const MEMBER_REGISTERED: &str = "resource_callback_member_registered";
/// The holder word: the object the call returns.
const RESULT: &str = "result";

/// The integer parameter types a discriminant's value may have.
const INTEGER_ABI_TYPES: [&str; 10] = [
    "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize",
];

/// One clause as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Written {
    /// `quant live #result;`.
    Live,
    /// `resource_discriminant_set(result, TAG.FIELD, P, FLAGS)`.
    Discriminant {
        member: CallbackMember,
        parameter: String,
        flags: u64,
    },
    /// A member's state: `resource_callback_member_registered(result,
    /// TAG.FIELD, F | absent)` or the shorthand's member part.
    Member {
        member: CallbackMember,
        /// `None`: `absent`; otherwise a callback parameter (the lower level
        /// only) or, naming no parameter, a function (card
        /// OP-APP-RK-CALLBACKS-VIA-POLL-CB, D1).
        callback: Option<String>,
        /// The clause, for diagnostics.
        text: String,
        /// Written as the `quant live` shorthand, whose value is `absent` or
        /// a function, never a parameter.
        shorthand: bool,
    },
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with(|ch: char| ch.is_ascii_digit())
        && text
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// `TAG.FIELD` with at least one `.`, every segment an identifier.
fn member(text: &str) -> Option<CallbackMember> {
    let (tag, path) = text.split_once('.')?;
    (is_identifier(tag) && path.split('.').all(is_identifier)).then(|| CallbackMember {
        tag: tag.to_owned(),
        path: path.to_owned(),
    })
}

/// A canonical non-negative decimal (C04.5's spelling).
fn canonical(text: &str) -> Option<u64> {
    if text.is_empty()
        || !text.bytes().all(|byte| byte.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return None;
    }
    text.parse().ok()
}

fn arguments<'a>(expression: &'a str, keyword: &str) -> Option<Vec<&'a str>> {
    let rest = expression.trim().strip_prefix(keyword)?;
    let inner = rest.trim_start().strip_prefix('(')?;
    let inner = inner.trim_end().strip_suffix(')')?;
    Some(inner.split(',').map(str::trim).collect())
}

/// Read an `ensures normal` clause of this card, `None` for any other.
pub(super) fn ensures_clause(expression: &str) -> Result<Option<Written>, String> {
    if let Some(words) = arguments(expression, DISCRIMINANT_SET) {
        let malformed = || {
            format!(
                "malformed `{DISCRIMINANT_SET}` clause `{expression}`: write \
                 `ensures normal {DISCRIMINANT_SET}(result, TAG.FIELD, P, FLAGS);` with `P` an \
                 integer parameter and `FLAGS` a canonical decimal (`0`: none)"
            )
        };
        let [object, field, parameter, flags] = words.as_slice() else {
            return Err(malformed());
        };
        if *object != RESULT || !is_identifier(parameter) {
            return Err(malformed());
        }
        let member = member(field).ok_or_else(malformed)?;
        let flags = canonical(flags).ok_or_else(malformed)?;
        return Ok(Some(Written::Discriminant {
            member,
            parameter: (*parameter).to_owned(),
            flags,
        }));
    }
    if let Some(words) = arguments(expression, MEMBER_REGISTERED) {
        let malformed = || {
            format!(
                "malformed `{MEMBER_REGISTERED}` clause `{expression}`: write \
                 `ensures normal {MEMBER_REGISTERED}(result, TAG.FIELD, F);` with `F` a callback \
                 parameter or `{ABSENT}`"
            )
        };
        let [object, field, value] = words.as_slice() else {
            return Err(malformed());
        };
        // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (Q2): a parameter holder is a
        // setter's registration, read with the callback clauses.
        if *object != RESULT {
            return Ok(None);
        }
        // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: the value keyword is
        // `#absent`; the bare word rejects naming it.
        if *value == ABSENT_WORD {
            return Err(bare_absent(expression));
        }
        if *value != ABSENT && !is_identifier(value) {
            return Err(malformed());
        }
        let member = member(field).ok_or_else(malformed)?;
        return Ok(Some(Written::Member {
            member,
            callback: (*value != ABSENT).then(|| (*value).to_owned()),
            text: expression.trim().to_owned(),
            shorthand: false,
        }));
    }
    Ok(None)
}

/// The rejection of a bare `absent` (card SUPPLIED-OWNED-RETURN-CODE-HANDOFF).
fn bare_absent(text: &str) -> String {
    format!(
        "quant-live-absent: `{text}`: `{ABSENT_WORD}` is the value keyword `{ABSENT}` (a value \
         keyword carries `#`)"
    )
}

/// Read the text after `quant live` in a resource v1 declaration: `#result`
/// (the liveness part) or `#result: TAG.FIELD |-> #absent` (the shorthand).
/// Every other shape rejects the module.
pub(super) fn quant_live_clause(source: &str) -> Result<Vec<Written>, String> {
    let shape = || {
        format!(
            "quant-live-result-shape: a resource v1 liveness clause is `quant live #result;` or \
             `quant live #result: TAG.FIELD {MAPS_TO} {ABSENT};`, not `quant live {source};`"
        )
    };
    let tokens = lex(source)
        .map_err(|error| format!("quant-live-result-shape: {error}"))?
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect::<Vec<_>>();
    let word = |at: usize| match tokens.get(at) {
        Some(TokenKind::Word(word)) => Some(word.as_str()),
        _ => None,
    };
    let symbol = |at: usize, expected: Symbol| tokens.get(at) == Some(&TokenKind::Symbol(expected));
    if !symbol(0, Symbol::Hash) || word(1) != Some(RESULT) {
        return Err(shape());
    }
    if tokens.len() == 2 {
        return Ok(vec![Written::Live]);
    }
    if !symbol(2, Symbol::Colon) {
        return Err(shape());
    }
    // The left of the maps-to: `TAG.FIELD`, words joined by dots.
    let mut at = 3;
    let mut segments = Vec::new();
    while let Some(segment) = word(at) {
        segments.push(segment);
        at += 1;
        if symbol(at, Symbol::Dot) {
            at += 1;
        } else {
            break;
        }
    }
    let left = segments.join(".");
    if symbol(at, Symbol::Arrow) {
        return Err(format!(
            "quant-live-maps-to: `->` is C member access, not maps-to: write \
             `quant live #result: TAG.FIELD {MAPS_TO} {ABSENT};` (`{MAPS_TO}` is the ASCII `↦`)"
        ));
    }
    let Some(member) = member(&left) else {
        return Err(format!(
            "quant-live-member: the left of `{MAPS_TO}` is a dotted `TAG.FIELD` (`struct TAG`'s \
             member), not `{}`",
            source
                .split_once(':')
                .map_or(source, |(_, rest)| rest)
                .split(MAPS_TO)
                .next()
                .unwrap_or("")
                .trim()
        ));
    };
    if !symbol(at, Symbol::MapsTo) {
        return Err(shape());
    }
    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (USER DECISION 2026-10-14, D1):
    // the right of `|->` is `#absent` or a function designator, one
    // identifier; anything else rejects the module. Whether the designator
    // names a parameter is checked with the declaration's names. Card
    // SUPPLIED-OWNED-RETURN-CODE-HANDOFF: `#absent`, the value keyword; the
    // bare `absent` rejects naming it.
    if symbol(at + 1, Symbol::Hash) {
        return match (word(at + 2), tokens.len() == at + 3) {
            (Some(ABSENT_WORD), true) => Ok(vec![
                Written::Live,
                Written::Member {
                    member,
                    callback: None,
                    text: format!("quant live {source}"),
                    shorthand: true,
                },
            ]),
            _ => Err(format!(
                "quant-live-value: a member maps to `{ABSENT}` (no callback) or a function \
                 designator; `quant live {source};` names another value"
            )),
        };
    }
    let callback = match (word(at + 1), tokens.len() == at + 2) {
        (Some(ABSENT_WORD), true) => return Err(bare_absent(&format!("quant live {source}"))),
        // C's null pointer is a value, not a function: `#absent` states it.
        (Some(name), true)
            if is_identifier(name) && !matches!(name, RESULT | "NULL" | "nullptr") =>
        {
            Some(name.to_owned())
        }
        (_, _) => {
            return Err(format!(
                "quant-live-value: a member maps to `{ABSENT}` (no callback) or a function \
                 designator; `quant live {source};` names another value"
            ));
        }
    };
    Ok(vec![
        Written::Live,
        Written::Member {
            member,
            callback,
            text: format!("quant live {source}"),
            shorthand: true,
        },
    ])
}

/// Validate one declaration's clauses of this card against its loaded
/// custody contract, giving the canonical [`ConstructedState`]: the
/// shorthand and its decomposition give the same state, and the same part
/// written twice (or by both levels) is merged.
pub(super) fn declaration(
    written: &[Written],
    contract: &ResourceFunctionContract,
    parameter_types: &[String],
    names: &BTreeMap<&str, usize>,
) -> Result<ConstructedState, String> {
    let mut state = ConstructedState::default();
    if written.is_empty() {
        return Ok(state);
    }
    if names.contains_key(RESULT) {
        return Err(format!(
            "a result state reads `{RESULT}` as the returned object: rename the parameter \
             `{RESULT}`"
        ));
    }
    // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: every reserved keyword (the
    // loader also checks it for any declaration carrying a `#` form).
    if let Some(name) = qcore_schemas::handoff::RESERVED_KEYWORDS
        .iter()
        .find(|name| names.contains_key(**name))
    {
        return Err(format!(
            "`#{name}` is reserved in a `#` form: rename the parameter `{name}`"
        ));
    }
    if contract.result != (ResourceReturnMode::Owned { nullable: false }) {
        return Err(
            "quant-live-unbacked: a result state needs `resource_return_owned(nonnull)`: the \
             caller owns the live object the state describes"
                .into(),
        );
    }
    let index = |name: &str| {
        names
            .get(name)
            .copied()
            .ok_or_else(|| format!("unknown parameter {name}"))
    };
    for clause in written {
        match clause {
            Written::Live => state.live = true,
            Written::Discriminant {
                member,
                parameter,
                flags,
            } => {
                let at = index(parameter)?;
                if !INTEGER_ABI_TYPES.contains(&parameter_types[at].as_str()) {
                    return Err(format!(
                        "a discriminant's value names an integer parameter, not `{parameter}`"
                    ));
                }
                let set = DiscriminantSet {
                    member: member.clone(),
                    parameter: at,
                    flags: *flags,
                };
                if state
                    .discriminant
                    .as_ref()
                    .is_some_and(|earlier| *earlier != set)
                {
                    return Err(format!(
                        "a declaration states one discriminant of its result (`{DISCRIMINANT_SET}`)"
                    ));
                }
                state.discriminant = Some(set);
            }
            Written::Member {
                member,
                callback,
                text,
                shorthand,
            } => {
                let value = match callback {
                    None => MemberValue::Absent,
                    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (D1): a name that
                    // is no parameter is a function designator, the stated
                    // default; the shorthand never names a parameter.
                    Some(name) if !names.contains_key(name.as_str()) => MemberValue::Function {
                        function: name.clone(),
                    },
                    Some(name) if *shorthand => {
                        return Err(format!(
                            "quant-live-value: `{text}`: a member maps to `{ABSENT}` or a function \
                             designator, not the parameter `{name}`"
                        ));
                    }
                    Some(name) => {
                        let at = index(name)?;
                        if parameter_types[at] != "fnptr"
                            || contract.parameters[at] != ResourceParameterMode::Borrowed
                        {
                            return Err(format!(
                                "`{text}`: the registered callback names a `fnptr` parameter \
                                 whose mode is `resource_borrowed`"
                            ));
                        }
                        MemberValue::Callback { callback: at }
                    }
                };
                let stated = MemberState {
                    member: member.clone(),
                    value,
                };
                match state
                    .members
                    .iter()
                    .find(|earlier| earlier.member == *member)
                {
                    Some(earlier) if *earlier == stated => {}
                    Some(_) => {
                        return Err(format!(
                            "`{text}`: the member `{}` is given two states",
                            member.spelling()
                        ));
                    }
                    None => state.members.push(stated),
                }
            }
        }
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shorthand_reads_as_its_two_parts() {
        let shorthand = quant_live_clause(&format!(
            "#result: rd_kafka_op_s.rko_op_cb {MAPS_TO} #absent"
        ))
        .expect("reads");
        assert_eq!(shorthand.len(), 2);
        assert_eq!(shorthand[0], Written::Live);
        assert!(matches!(
            &shorthand[1],
            Written::Member { member, callback: None, .. } if member.spelling() == "rd_kafka_op_s.rko_op_cb"
        ));
        assert_eq!(
            quant_live_clause("#result").expect("reads"),
            vec![Written::Live]
        );
    }

    #[test]
    fn every_other_shape_rejects_with_a_named_diagnostic() {
        let arrow = quant_live_clause("#result: rd_kafka_op_s.rko_op_cb -> #absent")
            .expect_err("a bare arrow rejects");
        assert!(arrow.starts_with("quant-live-maps-to:"), "{arrow}");
        assert!(arrow.contains(MAPS_TO), "{arrow}");
        let c_style = quant_live_clause(&format!("#result: result->rko_op_cb {MAPS_TO} #absent"))
            .expect_err("C member access rejects");
        assert!(c_style.starts_with("quant-live-maps-to:"), "{c_style}");
        let undotted = quant_live_clause(&format!("#result: rko_op_cb {MAPS_TO} #absent"))
            .expect_err("an undotted member rejects");
        assert!(undotted.starts_with("quant-live-member:"), "{undotted}");
        let value = quant_live_clause(&format!("#result: rd_kafka_op_s.rko_op_cb {MAPS_TO} NULL"))
            .expect_err("absent or a function, not C's null");
        assert!(value.starts_with("quant-live-value:"), "{value}");
        // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (D1): a function designator is
        // admitted; a number or an expression is not.
        let function = quant_live_clause(&format!(
            "#result: rd_kafka_conf_s.log_cb {MAPS_TO} rd_kafka_log_print"
        ))
        .expect("a function designator");
        assert!(matches!(
            &function[1],
            Written::Member { callback: Some(name), shorthand: true, .. } if name == "rd_kafka_log_print"
        ));
        for other in ["0", "f(x)", "&f", "a.b"] {
            let error = quant_live_clause(&format!(
                "#result: rd_kafka_conf_s.log_cb {MAPS_TO} {other}"
            ))
            .expect_err("not a designator");
            assert!(
                error.starts_with("quant-live-value:")
                    || error.starts_with("quant-live-result-shape:"),
                "{other}: {error}"
            );
        }
        let parameter = quant_live_clause("#rko").expect_err("only the result");
        assert!(
            parameter.starts_with("quant-live-result-shape:"),
            "{parameter}"
        );
        let trailing = quant_live_clause(&format!(
            "#result: rd_kafka_op_s.rko_op_cb {MAPS_TO} #absent #absent"
        ))
        .expect_err("one value");
        assert!(trailing.starts_with("quant-live-value:"), "{trailing}");
        // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: the bare word rejects,
        // naming the keyword; so does another `#` word.
        let bare = quant_live_clause(&format!(
            "#result: rd_kafka_op_s.rko_op_cb {MAPS_TO} absent"
        ))
        .expect_err("a bare absent rejects");
        assert!(
            bare.starts_with("quant-live-absent:") && bare.contains("#absent"),
            "{bare}"
        );
        let other = quant_live_clause(&format!(
            "#result: rd_kafka_op_s.rko_op_cb {MAPS_TO} #owner"
        ))
        .expect_err("only #absent");
        assert!(other.starts_with("quant-live-value:"), "{other}");
        let lower = ensures_clause(
            "resource_callback_member_registered(result, rd_kafka_op_s.rko_op_cb, absent)",
        )
        .expect_err("a bare absent rejects at the lower level");
        assert!(lower.contains("#absent"), "{lower}");
        assert!(matches!(
            ensures_clause(
                "resource_callback_member_registered(result, rd_kafka_op_s.rko_op_cb, #absent)"
            ),
            Ok(Some(Written::Member { callback: None, .. }))
        ));
    }
}
