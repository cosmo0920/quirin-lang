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

//! Card **OP-CALLBACK-MEMBER-PATH-RELEASE** (gateway rule 30; contract spec
//! §C04.19.6): the body check of a declaration's member-path states, in a
//! unit that defines the declared function.
//!
//! Over the Clang body:
//!
//! * the stated formal's pointee (through the unit's typedefs) is `struct
//!   TAG` for every stated tag; an unread pointee leaves an assumption;
//! * every store of a pointer value into a member path of a stated formal is
//!   a null pointer constant into a path stated `#released`, `#moved`, or a
//!   set of one of them; a store of a record holding a stated path, any other
//!   pointer store, and a store into or the address of a path stated `#stale`
//!   or `#kept` are **contradictions** (the declaration is withdrawn in that
//!   unit).
//!
//! Card **MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS** (rule 30, amended):
//! validation is the bounded path reading of
//! [`crate::member_path_reading`]: `goto`/labels and the other statements
//! are read as a control-flow graph, a static helper of the unit (a header
//! `static inline` included) that receives the formal or a member path's
//! address is followed with its parameters bound (a wrapper's `return f(…)`
//! keeps the callee's outcome per returned value), and every exit where a
//! state's guard may hold must establish it. A call that is not followed
//! leaves what it reaches unknown, so the clause stays a declared assumption
//! (rules 13i–13k); a contradiction found in the declaration's own body
//! still withdraws it. The check establishes the member's reading after the
//! call; that the object it held was released or handed on stays a declared
//! assumption, as for the arms (rule 12o).
//!
//! Card **SCHEDULER-DESTROY-MEMBER-STATES** (gateway rule 50):
//! [`validates_composing`] also reads the declaration's own unlink's link
//! members (a store there inside a followed helper is that clause's) and,
//! given the unit's helper declarations, composes a direct call of a unit
//! `static` helper that receives a `#stale` path's value: where every exit
//! takes the path's object exactly once through such a helper, the take is
//! established ([`Taken::Established`]) instead of assumed.

use qcore_schemas::member_path::{Fate, MemberPathState};
use serde_json::Value;

use std::collections::BTreeMap;

pub use crate::member_path_reading::{Helper, HelperLookup, Taken};
use crate::member_path_reading::{ORIG, Reader, taken_exits, validated_exits};
use crate::typed_c_ast::{children, kind};

fn strip(mut node: &Value) -> &Value {
    loop {
        match kind(node) {
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" | "ConstantExpr"
            | "ExprWithCleanups" => match children(node).first() {
                Some(inner) => node = inner,
                None => return node,
            },
            _ => return node,
        }
    }
}

/// The member path of `node` below the parameter whose declaration id is
/// `parameter` (`P->a.b.c` is `a.b.c`), `None` for anything else.
fn path_of(node: &Value, parameter: &str) -> Option<String> {
    let mut segments = Vec::new();
    let mut node = strip(node);
    loop {
        if kind(node) != "MemberExpr" {
            return None;
        }
        segments.push(node["name"].as_str()?.to_owned());
        let base = strip(children(node).first()?);
        if node["isArrow"].as_bool() == Some(true) {
            if kind(base) == "DeclRefExpr"
                && base["referencedDecl"]["id"].as_str() == Some(parameter)
            {
                segments.reverse();
                return Some(segments.join("."));
            }
            return None;
        }
        node = base;
    }
}

fn is_pointer(node: &Value) -> bool {
    let ty = &node["type"];
    let spelled = ty["desugaredQualType"]
        .as_str()
        .or_else(|| ty["qualType"].as_str())
        .unwrap_or("");
    spelled.trim_end().ends_with('*')
}

fn walk<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    out.push(node);
    for child in children(node) {
        walk(child, out);
    }
}

/// What rule 30's body check establishes for a declaration's member-path
/// states in a unit that defines it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Every exit where a state's guard may hold establishes it.
    Validated,
    /// A declared assumption, with the reading's reason.
    Assumption(String),
}

/// Rule 30's body check: `Ok(Standing::Validated)`, `Ok(Standing::Assumption)`
/// with the reading's reason, `Err` a contradiction. `callee` gives the unit's
/// unique body for a function name (the helpers the reading may follow).
///
/// # Errors
/// A contradiction, naming it.
pub fn validates<'a>(
    function: &'a Value,
    states: &[MemberPathState],
    pointee: &dyn Fn(&Value) -> Option<String>,
    callee: &dyn Fn(&str) -> Option<&'a Value>,
) -> Result<Standing, String> {
    validates_composing(function, states, pointee, callee, &BTreeMap::new(), None)
        .map(|checked| checked.standing)
}

/// What [`validates_composing`] establishes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    /// Rule 30's standing of the states.
    pub standing: Standing,
    /// Rule 50's take of the `#stale` paths' objects.
    pub taken: Taken,
    /// What the composed helpers state at the taken objects.
    pub composed: Vec<String>,
}

/// Rule 30's body check with rule 50's additions: `links` the link member
/// the declaration's own unlink names per formal, `helper` the unit's helper
/// declarations (no composition when `None`; [`Taken::NotStated`] then
/// stands for "not read").
///
/// # Errors
/// A contradiction, naming it.
pub fn validates_composing<'a>(
    function: &'a Value,
    states: &[MemberPathState],
    pointee: &dyn Fn(&Value) -> Option<String>,
    callee: &dyn Fn(&str) -> Option<&'a Value>,
    links: &BTreeMap<usize, String>,
    helper: Option<HelperLookup<'_>>,
) -> Result<Checked, String> {
    let mut taken = Taken::NotStated;
    let mut composed = std::collections::BTreeSet::new();
    let parameters = children(function)
        .iter()
        .filter(|child| kind(child) == "ParmVarDecl")
        .collect::<Vec<_>>();
    let body = children(function)
        .iter()
        .rev()
        .find(|child| kind(child) == "CompoundStmt")
        .ok_or("the body is not read")?;
    let mut nodes = Vec::new();
    walk(body, &mut nodes);
    let mut assumption: Option<String> = None;
    let formals = states
        .iter()
        .map(|state| state.parameter)
        .collect::<std::collections::BTreeSet<_>>();
    for formal in formals {
        let parameter = parameters
            .get(formal)
            .ok_or_else(|| format!("parameter {formal} is not read"))?;
        let id = parameter["id"]
            .as_str()
            .ok_or_else(|| format!("parameter {formal} has no id"))?;
        let stated = states
            .iter()
            .filter(|state| state.parameter == formal)
            .collect::<Vec<_>>();
        // The formal's pointee record, through the unit's typedefs (Clang's
        // JSON gives no desugared spelling for a pointer to a typedef): a
        // different record contradicts the stated tag; an unread one leaves
        // the clause a declared assumption.
        match pointee(parameter) {
            Some(record) => {
                if let Some(state) = stated.iter().find(|state| state.member.tag != record) {
                    return Err(format!(
                        "parameter {formal}'s pointee is `{record}`, not `{}`",
                        state.member.tag
                    ));
                }
            }
            None => {
                assumption.get_or_insert(format!("parameter {formal}'s pointee is not read"));
            }
        }
        let state_of = |path: &str| stated.iter().find(|state| state.member.path == path);
        let below_a_stated = |path: &str| {
            stated
                .iter()
                .any(|state| state.member.path.starts_with(&format!("{path}.")))
        };
        for node in &nodes {
            match kind(node) {
                "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                    let [left, right] = children(node) else {
                        continue;
                    };
                    let Some(path) = path_of(left, id) else {
                        continue;
                    };
                    if below_a_stated(&path) {
                        return Err(format!(
                            "a store of the record `{path}` holding a stated path"
                        ));
                    }
                    if !is_pointer(left) {
                        continue;
                    }
                    match state_of(&path) {
                        Some(state)
                            if matches!(state.state.fate, Fate::Released | Fate::Moved)
                                && crate::member_path_reading::null_constant(right) => {}
                        Some(state) => {
                            return Err(format!(
                                "a store into `{path}`, stated `{}`, that is not a null pointer \
                                 constant into a taken path",
                                state.state.spelling()
                            ));
                        }
                        None => {
                            return Err(format!("a pointer store into the unstated path `{path}`"));
                        }
                    }
                }
                "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                    let Some(inner) = children(node).first() else {
                        continue;
                    };
                    if let Some(path) = path_of(inner, id)
                        && let Some(state) = state_of(&path)
                        && matches!(state.state.fate, Fate::Stale | Fate::Kept)
                    {
                        return Err(format!(
                            "the address of `{path}`, stated `{}`, is taken",
                            state.state.spelling()
                        ));
                    }
                }
                _ => {}
            }
        }
        if assumption.is_some() {
            continue;
        }
        // Card MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: the path reading.
        let paths = stated
            .iter()
            .map(|state| state.member.path.clone())
            .collect::<Vec<_>>();
        let mut reader = Reader::new(paths.clone(), callee)
            .with_link_members(links.get(&formal).cloned().into_iter().collect());
        if let Some(helper) = helper {
            let stale = stated
                .iter()
                .map(|state| state.state.fate == Fate::Stale && !state.state.maybe)
                .collect();
            reader = reader.with_helpers(helper, stale);
        }
        let name = function["name"].as_str().unwrap_or("the declaration");
        let views = std::collections::BTreeMap::from([(formal, None)]);
        let outcomes = reader.read(name, function, &views, vec![ORIG; paths.len()]);
        if helper.is_some() {
            let here = match &outcomes {
                None => Taken::Assumption(
                    reader
                        .failure
                        .clone()
                        .unwrap_or_else(|| "the reading failed".into()),
                ),
                Some(outcomes) => match taken_exits(&stated, &paths, outcomes) {
                    Taken::Assumption(why) if !reader.take_failures.is_empty() => {
                        let failures = reader.take_failures.iter().cloned().collect::<Vec<_>>();
                        Taken::Assumption(format!("{why}; {}", failures.join("; ")))
                    }
                    other => other,
                },
            };
            taken = match (taken, here) {
                (Taken::Assumption(why), _) | (_, Taken::Assumption(why)) => Taken::Assumption(why),
                (Taken::Established, _) | (_, Taken::Established) => Taken::Established,
                _ => Taken::NotStated,
            };
            if matches!(taken, Taken::Established) {
                composed.extend(reader.composed.iter().cloned());
            }
        }
        let verdict = match outcomes {
            None => Err(reader
                .failure
                .clone()
                .unwrap_or_else(|| "the reading failed".into())),
            Some(outcomes) => validated_exits(&stated, &paths, &outcomes, &reader.stored),
        };
        if let Err(why) = verdict {
            let calls = reader.not_followed.iter().cloned().collect::<Vec<_>>();
            assumption = Some(if calls.is_empty() {
                why
            } else {
                format!("{why}; not followed: {}", calls.join("; "))
            });
        }
    }
    let standing = match assumption {
        None => Standing::Validated,
        Some(why) => Standing::Assumption(why),
    };
    if let (Standing::Assumption(why), Taken::Established) = (&standing, &taken) {
        // A take is established only where the reading itself stands.
        taken = Taken::Assumption(why.clone());
    }
    let composed = if helper.is_some() && taken == Taken::Established {
        composed.into_iter().collect()
    } else {
        Vec::new()
    };
    Ok(Checked {
        standing,
        taken,
        composed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::callback::CallbackMember;
    use qcore_schemas::handoff::{GuardOp, HandOffGuard};
    use qcore_schemas::member_path::PathState;
    use serde_json::json;

    fn member(base: &str, path: &[&str]) -> Value {
        let mut node = json!({"kind": "DeclRefExpr", "referencedDecl": {"id": base}});
        for (index, segment) in path.iter().enumerate() {
            node = json!({
                "kind": "MemberExpr",
                "name": segment,
                "isArrow": index == 0,
                "type": {"qualType": "void *"},
                "inner": [node],
            });
        }
        node
    }

    fn store_into(base: &str, path: &[&str], value: Value) -> Value {
        json!({"kind": "BinaryOperator", "opcode": "=", "inner": [member(base, path), value]})
    }

    fn store(path: &[&str], value: Value) -> Value {
        store_into("p", path, value)
    }

    fn null() -> Value {
        json!({"kind": "ImplicitCastExpr", "castKind": "NullToPointer",
               "inner": [{"kind": "IntegerLiteral", "value": "0"}]})
    }

    fn reference(id: &str) -> Value {
        json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
               "inner": [{"kind": "DeclRefExpr", "referencedDecl": {"id": id, "kind": "ParmVarDecl"}}]})
    }

    fn enumerator(name: &str) -> Value {
        json!({"kind": "DeclRefExpr", "referencedDecl": {"kind": "EnumConstantDecl", "name": name}})
    }

    fn returning(value: Option<Value>) -> Value {
        match value {
            Some(value) => json!({"kind": "ReturnStmt", "inner": [value]}),
            None => json!({"kind": "ReturnStmt"}),
        }
    }

    fn call(name: &str, arguments: Vec<Value>) -> Value {
        let mut inner = vec![
            json!({"kind": "ImplicitCastExpr", "castKind": "FunctionToPointerDecay",
            "inner": [{"kind": "DeclRefExpr", "referencedDecl": {"kind": "FunctionDecl", "name": name}}]}),
        ];
        inner.extend(arguments);
        json!({"kind": "CallExpr", "type": {"qualType": "int"}, "inner": inner})
    }

    fn branch(condition: Value, then: Value, otherwise: Option<Value>) -> Value {
        let mut inner = vec![condition, then];
        inner.extend(otherwise);
        json!({"kind": "IfStmt", "inner": inner})
    }

    fn opaque() -> Value {
        json!({"kind": "DeclRefExpr", "referencedDecl": {"id": "flag", "kind": "VarDecl"}})
    }

    fn compound(statements: Vec<Value>) -> Value {
        json!({"kind": "CompoundStmt", "inner": statements})
    }

    fn defined(
        name: &str,
        parameter: &str,
        storage: Option<&str>,
        statements: Vec<Value>,
    ) -> Value {
        let mut function = json!({"kind": "FunctionDecl", "name": name, "inner": [
            {"kind": "ParmVarDecl", "id": parameter, "type": {"qualType": "struct op_s *"}},
            compound(statements),
        ]});
        if let Some(storage) = storage {
            function["storageClass"] = json!(storage);
        }
        function
    }

    fn function(statements: Vec<Value>) -> Value {
        defined("callback", "p", None, statements)
    }

    /// The check with the pointee read from the spelled `struct TAG *` and
    /// the helpers in `unit`.
    fn check(
        function: &Value,
        states: &[MemberPathState],
        unit: &[Value],
    ) -> Result<Standing, String> {
        validates(
            function,
            states,
            &|parameter| {
                parameter["type"]["qualType"]
                    .as_str()?
                    .strip_prefix("struct ")?
                    .split_whitespace()
                    .next()
                    .map(str::to_owned)
            },
            &|name| {
                unit.iter()
                    .find(|helper| helper["name"].as_str() == Some(name))
            },
        )
    }

    fn validates_spelled(function: &Value, states: &[MemberPathState]) -> Result<bool, String> {
        check(function, states, &[]).map(|standing| standing == Standing::Validated)
    }

    fn state(path: &str, fate: Fate, maybe: bool) -> MemberPathState {
        MemberPathState {
            parameter: 0,
            member: CallbackMember {
                tag: "op_s".into(),
                path: path.into(),
            },
            state: PathState { fate, maybe },
            guard: None,
            parameter_guard: None,
            helper: None,
        }
    }

    fn guarded(mut state: MemberPathState) -> MemberPathState {
        state.guard = Some(HandOffGuard {
            op: GuardOp::Ne,
            constant: "KEEP".into(),
        });
        state
    }

    #[test]
    fn a_top_level_null_store_before_every_return_validates() {
        let body = function(vec![store(&["u", "a"], null()), returning(None)]);
        assert_eq!(
            validates_spelled(&body, &[state("u.a", Fate::Moved, false)]),
            Ok(true)
        );
    }

    #[test]
    fn a_return_before_the_store_is_an_assumption_and_a_set_needs_one_store() {
        let body = function(vec![
            branch(opaque(), returning(None), None),
            store(&["u", "a"], null()),
        ]);
        assert_eq!(
            validates_spelled(&body, &[state("u.a", Fate::Released, false)]),
            Ok(false)
        );
        assert_eq!(
            validates_spelled(&body, &[state("u.a", Fate::Released, true)]),
            Ok(true)
        );
        assert_eq!(
            validates_spelled(&function(vec![]), &[state("u.a", Fate::Moved, true)]),
            Ok(false)
        );
    }

    #[test]
    fn contradictions_withdraw() {
        let other = function(vec![store(&["u", "b"], null())]);
        assert!(validates_spelled(&other, &[state("u.a", Fate::Moved, false)]).is_err());
        let value = function(vec![store(
            &["u", "a"],
            json!({"kind": "DeclRefExpr", "referencedDecl": {"id": "q"}}),
        )]);
        assert!(validates_spelled(&value, &[state("u.a", Fate::Moved, false)]).is_err());
        let stale = function(vec![store(&["u", "a"], null())]);
        assert!(validates_spelled(&stale, &[state("u.a", Fate::Stale, false)]).is_err());
        let wrong_tag = json!({"kind": "FunctionDecl", "inner": [
            {"kind": "ParmVarDecl", "id": "p", "type": {"qualType": "struct other *"}},
            {"kind": "CompoundStmt", "inner": []},
        ]});
        assert!(validates_spelled(&wrong_tag, &[state("u.a", Fate::Stale, false)]).is_err());
    }

    #[test]
    fn an_unstored_stale_path_validates() {
        let body = function(vec![returning(None)]);
        assert_eq!(
            validates_spelled(&body, &[state("u.a", Fate::Stale, false)]),
            Ok(true)
        );
    }

    // Card MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS.

    #[test]
    fn a_static_helper_that_clears_the_member_validates() {
        let helper = defined(
            "clear",
            "o",
            Some("static"),
            vec![store_into("o", &["u", "a"], null())],
        );
        let body = function(vec![call("clear", vec![reference("p")]), returning(None)]);
        assert_eq!(
            check(
                &body,
                &[state("u.a", Fate::Released, false)],
                std::slice::from_ref(&helper)
            ),
            Ok(Standing::Validated)
        );
        // The same helper with external linkage is not followed.
        let external = defined(
            "clear",
            "o",
            None,
            vec![store_into("o", &["u", "a"], null())],
        );
        assert!(matches!(
            check(&body, &[state("u.a", Fate::Released, false)], &[external]),
            Ok(Standing::Assumption(_))
        ));
        // A helper reached through the address of a member path above the
        // stated one, as `rd_kafka_replyq_enq(&rko->…replyq, …)`.
        let enq = defined(
            "enq",
            "r",
            Some("static"),
            vec![
                store_into("r", &["q"], null()),
                call("external", vec![reference("r")]),
            ],
        );
        let through_address = function(vec![
            call(
                "enq",
                vec![json!({"kind": "UnaryOperator", "opcode": "&",
                            "inner": [member("p", &["u", "replyq"])]})],
            ),
            returning(None),
        ]);
        // `enq` then hands its whole view to an external call: unknown.
        assert!(matches!(
            check(
                &through_address,
                &[state("u.replyq.q", Fate::Released, false)],
                &[enq]
            ),
            Ok(Standing::Assumption(_))
        ));
        let enq = defined(
            "enq",
            "r",
            Some("static"),
            vec![store_into("r", &["q"], null())],
        );
        assert_eq!(
            check(
                &through_address,
                &[state("u.replyq.q", Fate::Released, false)],
                &[enq]
            ),
            Ok(Standing::Validated)
        );
    }

    #[test]
    fn a_helper_on_only_some_paths_does_not_validate() {
        let helper = defined(
            "clear",
            "o",
            Some("static"),
            vec![store_into("o", &["u", "a"], null())],
        );
        let body = function(vec![
            branch(opaque(), call("clear", vec![reference("p")]), None),
            returning(None),
        ]);
        assert!(matches!(
            check(
                &body,
                &[state("u.a", Fate::Released, false)],
                std::slice::from_ref(&helper)
            ),
            Ok(Standing::Assumption(_))
        ));
        // A null test of the path on the other branch closes it.
        let tested = function(vec![
            branch(
                json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
                       "inner": [member("p", &["u", "a"])]}),
                call("clear", vec![reference("p")]),
                None,
            ),
            returning(None),
        ]);
        assert_eq!(
            check(&tested, &[state("u.a", Fate::Released, false)], &[helper]),
            Ok(Standing::Validated)
        );
    }

    #[test]
    fn a_goto_to_a_shared_exit_that_clears_validates() {
        let body = function(vec![
            branch(
                opaque(),
                json!({"kind": "GotoStmt", "targetLabelDeclId": "out"}),
                None,
            ),
            json!({"kind": "LabelStmt", "declId": "out", "name": "out",
                   "inner": [store(&["u", "a"], null())]}),
            returning(None),
        ]);
        assert_eq!(
            check(&body, &[state("u.a", Fate::Released, false)], &[]),
            Ok(Standing::Validated)
        );
    }

    #[test]
    fn a_goto_that_skips_the_clear_does_not_validate() {
        let body = function(vec![
            branch(
                opaque(),
                json!({"kind": "GotoStmt", "targetLabelDeclId": "out"}),
                None,
            ),
            store(&["u", "a"], null()),
            json!({"kind": "LabelStmt", "declId": "out", "name": "out",
                   "inner": [returning(None)]}),
        ]);
        assert!(matches!(
            check(&body, &[state("u.a", Fate::Released, false)], &[]),
            Ok(Standing::Assumption(_))
        ));
    }

    #[test]
    fn recursion_does_not_validate() {
        // `clear` clears the member, then calls itself with the formal: the
        // recursive call is not followed and leaves the member unknown.
        let helper = defined(
            "clear",
            "o",
            Some("static"),
            vec![
                store_into("o", &["u", "a"], null()),
                call("clear", vec![reference("o")]),
            ],
        );
        let body = function(vec![call("clear", vec![reference("p")]), returning(None)]);
        assert!(matches!(
            check(&body, &[state("u.a", Fate::Released, false)], &[helper]),
            Ok(Standing::Assumption(_))
        ));
    }

    #[test]
    fn a_wrapper_keeps_its_workers_outcomes_and_the_guard_exempts_its_constant() {
        // The worker returns KEEP without clearing, HANDLED after clearing.
        let worker = defined(
            "worker",
            "o",
            Some("static"),
            vec![
                branch(opaque(), returning(Some(enumerator("KEEP"))), None),
                store_into("o", &["u", "a"], null()),
                returning(Some(enumerator("HANDLED"))),
            ],
        );
        let wrapper = function(vec![returning(Some(call("worker", vec![reference("p")])))]);
        let stated = [guarded(state("u.a", Fate::Released, false))];
        assert_eq!(
            check(&wrapper, &stated, std::slice::from_ref(&worker)),
            Ok(Standing::Validated)
        );
        // Unguarded, the KEEP exit must establish it too.
        assert!(matches!(
            check(&wrapper, &[state("u.a", Fate::Released, false)], &[worker]),
            Ok(Standing::Assumption(_))
        ));
    }

    #[test]
    fn a_call_that_is_not_followed_after_the_store_does_not_validate() {
        let body = function(vec![
            store(&["u", "a"], null()),
            call("external", vec![reference("p")]),
            returning(None),
        ]);
        assert!(matches!(
            check(&body, &[state("u.a", Fate::Released, false)], &[]),
            Ok(Standing::Assumption(_))
        ));
        // A member's value passed on reaches nothing of the formal.
        let value = function(vec![
            store(&["u", "a"], null()),
            call(
                "external",
                vec![
                    json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
                            "inner": [member("p", &["u", "b"])]}),
                ],
            ),
            returning(None),
        ]);
        assert_eq!(
            check(&value, &[state("u.a", Fate::Released, false)], &[]),
            Ok(Standing::Validated)
        );
    }

    #[test]
    fn a_copy_of_the_formal_into_a_local_does_not_validate() {
        let body = function(vec![
            json!({"kind": "DeclStmt", "inner": [{"kind": "VarDecl", "name": "alias",
                   "inner": [reference("p")]}]}),
            store(&["u", "a"], null()),
            returning(None),
        ]);
        assert!(matches!(
            check(&body, &[state("u.a", Fate::Released, false)], &[]),
            Ok(Standing::Assumption(_))
        ));
    }

    // Card SCHEDULER-DESTROY-MEMBER-STATES (gateway rule 50).

    const THR: &str = "struct thr *";

    fn typed_member(base: &str, path: &[&str], ty: &str) -> Value {
        let mut node = member(base, path);
        node["type"] = json!({ "qualType": ty });
        json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
               "type": {"qualType": ty}, "inner": [node]})
    }

    fn local(id: &str, ty: &str) -> Value {
        json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue", "type": {"qualType": ty},
               "inner": [{"kind": "DeclRefExpr", "type": {"qualType": ty},
                          "referencedDecl": {"id": id, "kind": "VarDecl", "name": id}}]})
    }

    fn declare(id: &str, ty: &str) -> Value {
        json!({"kind": "DeclStmt", "inner": [{"kind": "VarDecl", "id": id, "name": id,
               "type": {"qualType": ty}}]})
    }

    fn assign(id: &str, value: Value) -> Value {
        json!({"kind": "BinaryOperator", "opcode": "=", "type": {"qualType": THR}, "inner": [
            {"kind": "DeclRefExpr", "type": {"qualType": THR},
             "referencedDecl": {"id": id, "kind": "VarDecl", "name": id}},
            value,
        ]})
    }

    fn destroyer_helper() -> Helper {
        Helper {
            parameter: 0,
            tag: "thr".into(),
            releases: true,
            invokes_nothing: Ok(()),
            states: vec![("callee".into(), "#stale".into())],
            invocations: vec!["cb".into()],
        }
    }

    fn unit() -> Vec<Value> {
        vec![
            defined(
                "destroy_thr",
                "t",
                Some("static"),
                vec![call("free", vec![reference("t")])],
            ),
            defined(
                "destroy_external",
                "t",
                None,
                vec![call("free", vec![reference("t")])],
            ),
        ]
    }

    fn compose(
        function: &Value,
        states: &[MemberPathState],
        unit: &[Value],
        links: &BTreeMap<usize, String>,
        helper: &dyn Fn(&str) -> Option<Helper>,
    ) -> Result<Checked, String> {
        validates_composing(
            function,
            states,
            &|parameter| {
                parameter["type"]["qualType"]
                    .as_str()?
                    .strip_prefix("struct ")?
                    .split_whitespace()
                    .next()
                    .map(str::to_owned)
            },
            &|name| {
                unit.iter()
                    .find(|helper| helper["name"].as_str() == Some(name))
            },
            links,
            Some(helper),
        )
    }

    fn stale_parent() -> Vec<MemberPathState> {
        vec![
            state("parent", Fate::Stale, false),
            state("session", Fate::Kept, false),
        ]
    }

    fn taken_of(statements: Vec<Value>) -> Taken {
        let helper = |name: &str| (name == "destroy_thr").then(destroyer_helper);
        compose(
            &function(statements),
            &stale_parent(),
            &unit(),
            &BTreeMap::new(),
            &helper,
        )
        .expect("no contradiction")
        .taken
    }

    fn established(statements: Vec<Value>) -> bool {
        taken_of(statements) == Taken::Established
    }

    fn copy_then(statements: Vec<Value>) -> Vec<Value> {
        let mut body = vec![
            declare("th", THR),
            assign("th", typed_member("p", &["parent"], THR)),
        ];
        body.extend(statements);
        body
    }

    #[test]
    fn a_helper_handed_the_copy_of_a_stale_member_takes_its_object() {
        let helper = |name: &str| (name == "destroy_thr").then(destroyer_helper);
        let body = function(copy_then(vec![
            call("destroy_thr", vec![local("th", THR)]),
            returning(None),
        ]));
        let checked = compose(&body, &stale_parent(), &unit(), &BTreeMap::new(), &helper)
            .expect("no contradiction");
        assert_eq!(checked.standing, Standing::Validated);
        assert_eq!(checked.taken, Taken::Established);
        assert!(
            checked
                .composed
                .contains(&"`parent`'s object is released by `destroy_thr`".to_owned()),
            "{:?}",
            checked.composed
        );
        assert!(
            checked
                .composed
                .contains(&"`parent->callee` reads #stale (`destroy_thr`)".to_owned()),
            "{:?}",
            checked.composed
        );
        // The member's value handed directly composes the same way.
        assert!(established(vec![
            call("destroy_thr", vec![typed_member("p", &["parent"], THR)]),
            returning(None),
        ]));
        // Without the helpers, rule 30's standing alone (no take read).
        assert_eq!(
            validates(&body, &stale_parent(), &|_| Some("op_s".into()), &|_| None),
            Ok(Standing::Validated)
        );
    }

    #[test]
    fn a_take_on_some_paths_or_twice_is_an_assumption() {
        assert!(!established(copy_then(vec![
            branch(opaque(), call("destroy_thr", vec![local("th", THR)]), None),
            returning(None),
        ])));
        assert!(!established(copy_then(vec![
            call("destroy_thr", vec![local("th", THR)]),
            call("destroy_thr", vec![local("th", THR)]),
            returning(None),
        ])));
        // A path that returns before the take.
        assert!(!established(copy_then(vec![
            branch(opaque(), returning(None), None),
            call("destroy_thr", vec![local("th", THR)]),
            returning(None),
        ])));
        // No take at all.
        assert!(matches!(
            taken_of(vec![returning(None)]),
            Taken::Assumption(_)
        ));
    }

    #[test]
    fn an_escaping_value_spoils_the_take() {
        // Handed to an external call before the helper.
        assert!(!established(copy_then(vec![
            call("external", vec![local("th", THR)]),
            call("destroy_thr", vec![local("th", THR)]),
            returning(None),
        ])));
        // Stored elsewhere.
        assert!(!established(copy_then(vec![
            assign("other", local("th", THR)),
            call("destroy_thr", vec![local("th", THR)]),
            returning(None),
        ])));
        // Returned.
        assert!(!established(copy_then(vec![
            call("destroy_thr", vec![local("th", THR)]),
            returning(Some(local("th", THR))),
        ])));
        // A local written twice is no copy: its writes carry the value away.
        assert!(!established(copy_then(vec![
            assign("th", typed_member("p", &["parent"], THR)),
            call("destroy_thr", vec![local("th", THR)]),
            returning(None),
        ])));
    }

    #[test]
    fn only_a_static_helper_in_force_that_releases_the_record_is_composed() {
        // A non-static callee with the same declaration.
        let helper =
            |name: &str| matches!(name, "destroy_thr" | "destroy_external").then(destroyer_helper);
        let external = function(copy_then(vec![
            call("destroy_external", vec![local("th", THR)]),
            returning(None),
        ]));
        assert!(matches!(
            compose(
                &external,
                &stale_parent(),
                &unit(),
                &BTreeMap::new(),
                &helper
            )
            .expect("no contradiction")
            .taken,
            Taken::Assumption(_)
        ));
        let body = function(copy_then(vec![
            call("destroy_thr", vec![local("th", THR)]),
            returning(None),
        ]));
        let with = |helper: Helper| {
            compose(
                &body,
                &stale_parent(),
                &unit(),
                &BTreeMap::new(),
                &move |name: &str| (name == "destroy_thr").then(|| helper.clone()),
            )
            .expect("no contradiction")
            .taken
        };
        // No declaration in force.
        assert!(matches!(
            compose(&body, &stale_parent(), &unit(), &BTreeMap::new(), &|_| None)
                .expect("no contradiction")
                .taken,
            Taken::Assumption(_)
        ));
        // A helper that does not release its formal.
        assert!(matches!(
            with(Helper {
                releases: false,
                ..destroyer_helper()
            }),
            Taken::Assumption(_)
        ));
        // A member invocation whose run set is not closed and empty.
        assert!(matches!(
            with(Helper {
                invokes_nothing: Err("open".into()),
                ..destroyer_helper()
            }),
            Taken::Assumption(_)
        ));
        // A formal of another record.
        assert!(matches!(
            with(Helper {
                tag: "other".into(),
                ..destroyer_helper()
            }),
            Taken::Assumption(_)
        ));
        // Its states at another position.
        assert!(matches!(
            with(Helper {
                parameter: 1,
                ..destroyer_helper()
            }),
            Taken::Assumption(_)
        ));
        assert_eq!(with(destroyer_helper()), Taken::Established);
    }

    #[test]
    fn a_helper_reached_through_a_followed_helper_is_not_composed() {
        let wrap = defined(
            "wrap",
            "o",
            Some("static"),
            vec![call(
                "destroy_thr",
                vec![typed_member("o", &["parent"], THR)],
            )],
        );
        let mut helpers = unit();
        helpers.push(wrap);
        let helper = |name: &str| (name == "destroy_thr").then(destroyer_helper);
        let body = function(vec![call("wrap", vec![reference("p")]), returning(None)]);
        assert!(matches!(
            compose(&body, &stale_parent(), &helpers, &BTreeMap::new(), &helper)
                .expect("no contradiction")
                .taken,
            Taken::Assumption(_)
        ));
    }

    #[test]
    fn a_store_below_the_unlinks_link_member_inside_a_helper_is_that_clauses() {
        let unlink = defined(
            "unlink",
            "l",
            Some("static"),
            vec![store_into("l", &["next"], null())],
        );
        let body = function(vec![
            call(
                "unlink",
                vec![json!({"kind": "UnaryOperator", "opcode": "&",
                            "inner": [member("p", &["link"])]})],
            ),
            returning(None),
        ]);
        let states = [state("parent", Fate::Stale, false)];
        let helper = |_: &str| None;
        // Without the link member the helper's store ends the reading.
        assert!(matches!(
            compose(
                &body,
                &states,
                std::slice::from_ref(&unlink),
                &BTreeMap::new(),
                &helper
            )
            .expect("no contradiction")
            .standing,
            Standing::Assumption(_)
        ));
        let links = BTreeMap::from([(0, "link".to_owned())]);
        assert_eq!(
            compose(
                &body,
                &states,
                std::slice::from_ref(&unlink),
                &links,
                &helper
            )
            .expect("no contradiction")
            .standing,
            Standing::Validated
        );
        // Another member is still an unstated store.
        let other = BTreeMap::from([(0, "other".to_owned())]);
        assert!(matches!(
            compose(&body, &states, &[unlink], &other, &helper)
                .expect("no contradiction")
                .standing,
            Standing::Assumption(_)
        ));
    }
}
