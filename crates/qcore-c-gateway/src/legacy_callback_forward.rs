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

//! Card **SERVE-FORWARDED-CALLBACK-FORMALS** (gateway rule 15w): a callback
//! formal handed on, unchanged, to the same clause position of a callee
//! whose declaration is in force.
//!
//! Rule 13y validates `resource_callback_invoked(F, A…)` on a body that names
//! `F` only as the callee of direct calls whose actuals match `A…`; a body
//! that hands `F` on was withdrawn. Rule 15w reads a body whose every mention
//! of `F` is an exact forward instead: the whole actual, at position `p`, of
//! a direct call to a callee `G` whose declaration in force names `p` in
//! exactly one clause, `resource_callback_invoked(p, A'…)`, keeps `p`
//! `resource_borrowed`, and whose `A'…` read through the call's actuals are
//! `A…` position by position (a parameter word: the caller's own formal,
//! unchanged, at that parameter of `G`; `_` and `supplied_owned`: the same
//! word). The callee then calls `F` only as the caller declares, so the
//! caller's declaration stands (rule 13y's check, the forward case), and at
//! the forwarding call the invocation is the caller's own declared one,
//! decided at the caller's callers (rule 13u's check, the forward case).
//!
//! Two hooks, each the identity on every environment without a forward:
//! [`forward_validates`] (from `callback_route::settle`, where the standing
//! set is the least fixed point, so a declaration whose only support would be
//! itself never stands) and [`Parser::forwarded_invocation`] (from
//! `Parser::callback_operands`). Any deviation keeps the refusal it had: a
//! formal reassigned, address-taken, stored, cast, used as an operand,
//! invoked locally beside a forward, forwarded to another position, or to a
//! callee with no declaration in force there.

use qcore_schemas::callback::{CallbackActual, CallbackClause, CallbackHolder};
use qcore_schemas::constructed_state::MemberValue;
use qcore_schemas::contract::ResourceParameterMode;
use qcore_schemas::retained_by::RetainingContract;

use super::Parser;

/// One mention of a callback formal: the whole actual at `position` of a
/// direct call to `callee`, with every actual of that call as spelled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Forward {
    pub callee: String,
    pub position: usize,
    pub arguments: Vec<Vec<String>>,
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// C keywords that precede a parenthesis without calling anything.
const NOT_CALLEES: [&str; 8] = [
    "if", "while", "for", "switch", "return", "sizeof", "_Alignof", "_Generic",
];

/// Every mention of `name` in `body` (a member name after `.` or `->`
/// excepted), each read as a forward: `Some` only when there is at least one
/// mention and every mention is the whole actual of a direct call `G(…)`, `G`
/// an identifier not reached through `.` or `->`; `None` for anything else (a
/// local invocation, a store, an operand, a cast, a parenthesised use).
pub(crate) fn forwards(body: &[&str], name: &str) -> Option<Vec<Forward>> {
    let mut out = Vec::new();
    for (at, word) in body.iter().enumerate() {
        if *word != name || matches!(at.checked_sub(1).map(|b| body[b]), Some("." | "->")) {
            continue;
        }
        // The whole actual: delimited by the call's `(` or `,` and `,` or `)`.
        let previous = at.checked_sub(1).map(|b| body[b]);
        let next = body.get(at + 1).copied();
        if !matches!(previous, Some("(" | ",")) || !matches!(next, Some("," | ")")) {
            return None;
        }
        // The `(` that opens the enclosing argument list.
        let mut depth = 0usize;
        let mut open = None;
        for back in (0..at).rev() {
            match body[back] {
                ")" | "]" => depth += 1,
                "(" | "[" if depth > 0 => depth -= 1,
                "(" => {
                    open = Some(back);
                    break;
                }
                "[" | "{" | "}" | ";" => return None,
                _ => {}
            }
        }
        let open = open?;
        let call = open.checked_sub(1)?;
        let callee = body[call];
        if !is_identifier(callee)
            || NOT_CALLEES.contains(&callee)
            || matches!(call.checked_sub(1).map(|b| body[b]), Some("." | "->"))
        {
            return None;
        }
        let spans = super::call_argument_spans(body, call)?;
        let position = spans
            .iter()
            .position(|&(start, end)| start <= at && at < end)?;
        if spans[position] != (at, at + 1) {
            return None;
        }
        out.push(Forward {
            callee: callee.to_owned(),
            position,
            arguments: spans
                .iter()
                .map(|&(start, end)| body[start..end].iter().map(|w| (*w).to_owned()).collect())
                .collect(),
        });
    }
    (!out.is_empty()).then_some(out)
}

/// How many times `clause` names parameter `position` (as its callback,
/// table, holder or element, or as a parameter word among its actuals).
fn names_position(clause: &CallbackClause, position: usize) -> usize {
    let in_actuals = |actuals: &[CallbackActual]| {
        actuals
            .iter()
            .filter(|actual| matches!(actual, CallbackActual::Parameter { index } if *index == position))
            .count()
    };
    match clause {
        CallbackClause::Invoked { callback, actuals } => {
            usize::from(*callback == position) + in_actuals(actuals)
        }
        CallbackClause::Table { table, actuals } => {
            usize::from(*table == position) + in_actuals(actuals)
        }
        CallbackClause::Registered { callback, holder } => {
            usize::from(*callback == position)
                + usize::from(
                    matches!(holder, CallbackHolder::Parameter { index } if *index == position),
                )
        }
        CallbackClause::RegistryInvoked { holder, actuals } => {
            usize::from(*holder == position) + in_actuals(actuals)
        }
        CallbackClause::ElementAdded { element, holder } => {
            usize::from(*element == position) + usize::from(*holder == position)
        }
        CallbackClause::Unregistered { holder }
        | CallbackClause::ElementsDropped { holder }
        | CallbackClause::ReleasedIfConstructed { holder } => usize::from(*holder == position),
        CallbackClause::MemberInvoked { actuals, .. } => in_actuals(actuals),
        // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (Q2): a setter names its holder
        // and its callback.
        CallbackClause::MemberRegistered {
            object, callback, ..
        } => usize::from(*object == position) + usize::from(*callback == position),
    }
}

/// The one invocation `callee` states of its parameter `position`: `Some`
/// only when that parameter is `resource_borrowed`, is named by exactly one
/// clause, `resource_callback_invoked(position, A'…)`, is never retained, and
/// is registered in no constructed result.
pub(crate) fn stated_invocation(
    callee: &RetainingContract,
    position: usize,
) -> Option<&[CallbackActual]> {
    if callee.callbacks_withdrawn
        || callee.contract.parameters.get(position) != Some(&ResourceParameterMode::Borrowed)
        || callee
            .retentions
            .iter()
            .any(|retention| retention.retained == position)
        || callee.constructed.members.iter().any(|state| {
            matches!(state.value, MemberValue::Callback { callback } if callback == position)
        })
    {
        return None;
    }
    let naming = callee
        .callbacks
        .iter()
        .map(|clause| names_position(clause, position))
        .sum::<usize>();
    if naming != 1 {
        return None;
    }
    callee.callbacks.iter().find_map(|clause| match clause {
        CallbackClause::Invoked { callback, actuals } if *callback == position => {
            Some(actuals.as_slice())
        }
        _ => None,
    })
}

/// Whether the callee's actuals `callee`, read through the call's
/// `arguments`, are the caller's clause actuals `caller` position by
/// position: a caller parameter word `k` against a callee parameter word `k'`
/// whose actual is exactly the caller's formal `k`, `unchanged`; `_` against
/// `_`; `supplied_owned` against `supplied_owned`. Anything else deviates.
pub(crate) fn corresponds(
    caller_parameters: &[String],
    caller: &[CallbackActual],
    callee: &[CallbackActual],
    arguments: &[Vec<String>],
    unchanged: &dyn Fn(&str) -> bool,
) -> bool {
    caller.len() == callee.len()
        && caller.iter().zip(callee).all(|pair| match pair {
            (CallbackActual::Parameter { index }, CallbackActual::Parameter { index: theirs }) => {
                caller_parameters.get(*index).is_some_and(|formal| {
                    !formal.is_empty()
                        && arguments
                            .get(*theirs)
                            .is_some_and(|argument| argument.as_slice() == [formal.as_str()])
                        && unchanged(formal)
                })
            }
            (CallbackActual::Supplied, CallbackActual::Supplied)
            | (CallbackActual::SuppliedOwned, CallbackActual::SuppliedOwned) => true,
            _ => false,
        })
}

/// Rule 15w at the body check: the clause `resource_callback_invoked(F, A…)`
/// (`F` the parameter at `callback`) stands on a body whose every mention of
/// `F` is an exact forward to a callee `in_force` answers for, `F` and every
/// forwarded formal never rebound.
pub(crate) fn forward_validates<'a>(
    body: &[&str],
    parameters: &[String],
    callback: usize,
    actuals: &[CallbackActual],
    in_force: &dyn Fn(&str) -> Option<&'a RetainingContract>,
) -> bool {
    let Some(name) = parameters.get(callback).filter(|name| !name.is_empty()) else {
        return false;
    };
    let unchanged = |formal: &str| !super::callback_route::rebound(body, formal);
    if !unchanged(name) {
        return false;
    }
    let Some(forwards) = forwards(body, name) else {
        return false;
    };
    forwards.iter().all(|forward| {
        in_force(&forward.callee)
            .and_then(|callee| stated_invocation(callee, forward.position))
            .is_some_and(|theirs| {
                corresponds(parameters, actuals, theirs, &forward.arguments, &unchanged)
            })
    })
}

impl Parser<'_> {
    /// Rule 15w at a call (rule 13u's forward case): the clause
    /// `resource_callback_invoked(position, theirs…)` of `callee` at a call
    /// whose actual there is the calling function's own callback formal `F`,
    /// handed on exactly as its declaration in force here states `F`'s
    /// invocation, is that declared invocation: decided at the calling
    /// function's own callers, nothing to decide here.
    pub(super) fn forwarded_invocation(
        &self,
        callee: &RetainingContract,
        position: usize,
        actual: &dyn Fn(usize) -> Option<Vec<String>>,
    ) -> bool {
        let Some(theirs) = stated_invocation(callee, position) else {
            return false;
        };
        let Some(words) = actual(position) else {
            return false;
        };
        let [formal] = words.as_slice() else {
            return false;
        };
        let function = self.typed_path_function.as_str();
        let Some(own) = self.contracts.retaining_contract(function) else {
            return false;
        };
        if own.callbacks_withdrawn {
            return false;
        }
        let Some((_, body)) = super::slot_move_route::defined_body(self.tokens, function) else {
            return false;
        };
        let Some(parameters) = super::callback_route::parameter_names(self.tokens, function) else {
            return false;
        };
        let Some(index) = parameters.iter().position(|name| name == formal) else {
            return false;
        };
        let unchanged = |name: &str| !super::callback_route::rebound(&body, name);
        if !unchanged(formal) {
            return false;
        }
        let mut arguments = Vec::new();
        let mut at = 0usize;
        while let Some(words) = actual(at) {
            arguments.push(words);
            at += 1;
        }
        // Every clause of the calling function's declaration on `F` is the
        // forwarded one, and there is at least one.
        let mut own_clauses = own
            .callbacks
            .iter()
            .filter(|clause| names_position(clause, index) > 0)
            .peekable();
        own_clauses.peek().is_some()
            && own_clauses.all(|clause| match clause {
                CallbackClause::Invoked { callback, actuals } if *callback == index => {
                    corresponds(&parameters, actuals, theirs, &arguments, &unchanged)
                }
                _ => false,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    fn formals(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_whole_actual_of_a_direct_call_is_a_forward() {
        let body = words(
            "{ return rd_kafka_q_serve0 ( rkq , timeout_ms , max_cnt , cb_type , callback , \
             opaque , rd_false ) ; }",
        );
        let found = forwards(&body, "callback").expect("one forward");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].callee, "rd_kafka_q_serve0");
        assert_eq!(found[0].position, 4);
        assert_eq!(found[0].arguments[3], vec!["cb_type".to_owned()]);
        assert_eq!(found[0].arguments.len(), 7);
    }

    #[test]
    fn any_other_mention_is_no_forward() {
        for body in [
            "{ cb ( r , 0 , c ) ; return g ( r , cb , c ) ; }",
            "{ return g ( r , ( void * ) cb , c ) ; }",
            "{ x = cb ; return g ( r , x , c ) ; }",
            "{ return g ( r , cb == 0 , c ) ; }",
            "{ return s . g ( r , cb , c ) ; }",
            "{ if ( cb ) return 0 ; return g ( r , cb , c ) ; }",
            "{ return t [ cb ] ; }",
            "{ return 0 ; }",
        ] {
            assert_eq!(forwards(&words(body), "cb"), None, "{body}");
        }
    }

    #[test]
    fn correspondence_is_position_by_position() {
        let parameters = formals(&["r", "cb", "c"]);
        let arguments = vec![
            vec!["r".to_owned()],
            vec!["cb".to_owned()],
            vec!["c".to_owned()],
        ];
        let unchanged = |_: &str| true;
        let caller = [
            CallbackActual::Parameter { index: 0 },
            CallbackActual::Supplied,
            CallbackActual::Parameter { index: 2 },
        ];
        assert!(corresponds(
            &parameters,
            &caller,
            &caller,
            &arguments,
            &unchanged
        ));
        // A caller word against a callee `_`, or the other way round.
        let supplied = [
            CallbackActual::Parameter { index: 0 },
            CallbackActual::Supplied,
            CallbackActual::Supplied,
        ];
        assert!(!corresponds(
            &parameters,
            &caller,
            &supplied,
            &arguments,
            &unchanged
        ));
        assert!(!corresponds(
            &parameters,
            &supplied,
            &caller,
            &arguments,
            &unchanged
        ));
        // `supplied_owned` against `_`.
        let owned = [
            CallbackActual::Parameter { index: 0 },
            CallbackActual::SuppliedOwned,
            CallbackActual::Parameter { index: 2 },
        ];
        assert!(!corresponds(
            &parameters,
            &owned,
            &caller,
            &arguments,
            &unchanged
        ));
        // The callee's word reads another actual than the caller's formal.
        let swapped = vec![
            vec!["c".to_owned()],
            vec!["cb".to_owned()],
            vec!["r".to_owned()],
        ];
        assert!(!corresponds(
            &parameters,
            &caller,
            &caller,
            &swapped,
            &unchanged
        ));
        // A changed formal.
        assert!(!corresponds(
            &parameters,
            &caller,
            &caller,
            &arguments,
            &|_| false
        ));
        // Different arity.
        assert!(!corresponds(
            &parameters,
            &caller,
            &caller[..2],
            &arguments,
            &unchanged
        ));
    }

    const DECLARATIONS: &str = "module contract;
extern fn g(reader: ptr<mut, u8>, cb: fnptr, context: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(reader);
    ensures normal resource_borrowed(cb);
    ensures normal resource_borrowed(context);
    ensures normal resource_callback_invoked(cb, reader, _, context);
    ensures normal resource_return_none();
};
extern fn keep(holder: ptr<mut, u8>, cb: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(holder);
    ensures normal resource_borrowed(cb);
    ensures normal resource_callback_registered(cb, holder);
    ensures normal resource_return_none();
};
";

    /// `FORWARD_VECTORS` of `QCore.CallbackForward` (`forward_vectors_hold`),
    /// each a body of a caller declared `resource_callback_invoked(cb, r, _,
    /// c)` over `(r, cb, c)`, with whether the callee `g` is in force and
    /// whether the clause stands.
    const FORWARD_VECTORS: [(&str, bool, bool); 8] = [
        ("{ return g ( r , cb , c ) ; }", true, true),
        ("{ return g ( r , cb , c ) ; }", false, false),
        ("{ cb = 0 ; return g ( r , cb , c ) ; }", true, false),
        ("{ return g ( r , c , cb ) ; }", true, false),
        ("{ return h ( r , cb , c ) ; }", true, false),
        (
            "{ cb ( r , 0 , c ) ; return g ( r , cb , c ) ; }",
            true,
            false,
        ),
        ("{ return g ( c , cb , r ) ; }", true, false),
        ("{ keep ( r , cb ) ; return 0 ; }", true, false),
    ];

    #[test]
    fn forward_vectors_hold() {
        let contracts = qcore_contract::contract_loader::load_resource_contract_sources(
            &std::collections::BTreeMap::from([("forward.qv".to_owned(), DECLARATIONS.to_owned())]),
        )
        .expect("the declarations load");
        let parameters = formals(&["r", "cb", "c"]);
        let caller = [
            CallbackActual::Parameter { index: 0 },
            CallbackActual::Supplied,
            CallbackActual::Parameter { index: 2 },
        ];
        for (body, g_in_force, stands) in FORWARD_VECTORS {
            let in_force = |name: &str| {
                (name != "g" || g_in_force)
                    .then(|| contracts.retaining_contract(name))
                    .flatten()
            };
            assert_eq!(
                forward_validates(&words(body), &parameters, 1, &caller, &in_force),
                stands,
                "{body} (g in force: {g_in_force})"
            );
        }
    }
}
