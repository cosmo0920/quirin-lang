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

//! Card **W43-INDEX-TRAVERSAL**: the syntactic half of index and pointer
//! traversal.
//!
//! Code that walks a buffer or a string does so by index (`s[i]`, `i++`) or by
//! pointer increment (`p++`, `p += k`, `*p++ = c`). The typed scalar-access
//! rule of card W43-INDIRECT-ACCESS-GENERAL already reads one access; this
//! module recognises the statements that **move** the pointer or the index,
//! so the flow analysis can follow them. Every function here is a pure
//! recogniser over the analyzer's own token texts and the declaration facts
//! of [`crate::typed_field_access`]; none of them admits anything by itself.
//! What a recognised statement does to the abstract state is decided in
//! [`crate::legacy_c`], where it is checked flow-sensitively.
//!
//! The recognisers are deliberately narrow:
//!
//! * a **pointer step** is `P++`, `++P`, `P--`, `--P`, `P += E`, `P -= E`,
//!   `P = Q + E` or `P = Q - E` (the last two also as a declaration
//!   `T * P = Q ± E`), where `P` is a declared `SCALAR *` or `struct T *`
//!   name, `Q` any declared single-level pointer object and `E` a scalar value
//!   with no memory access;
//! * a **stepped access** is one typed scalar access statement in which
//!   `* P ++`/`* P --` (also `* ( P ++ )`) or an index `[ X ++ ]`/`[ ++ X ]`
//!   (and the decrements) appears, with the stepped name nowhere else in the
//!   statement: the access is admitted over the old value and the step or the
//!   index's forgotten fact follows it, which is the order C sequences them
//!   in (C11 6.5.2.4p2: the value computation precedes the side effect, and
//!   the statement's end is a sequence point).
//!
//! Nothing here decides bounds. An offset is a scalar the rule does not
//! evaluate; the only fact kept about it is whether it is a positive integer
//! literal, which is what lets a stepped pointer be told apart from its base.

pub(crate) use crate::legacy_c::StepOffset;
use crate::typed_field_access::{
    FunctionTypes, PointerKind, TypedAccessModel, TypedScalarPlan, is_name,
    is_scalar_value_expression, typed_scalar_statement,
};

/// One recognised pointer step: `destination = source ± offset`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PointerStepPlan {
    pub(crate) destination: String,
    pub(crate) source: String,
    pub(crate) offset: StepOffset,
}

/// One side effect a stepped access performs after its access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TraversalEffect {
    /// `* P ++`: the pointer moves after the access through its old value.
    Step(PointerStepPlan),
    /// `[ X ++ ]`: the declared scalar index's old fact is forgotten.
    Forget(String),
}

/// Strip parentheses that enclose the whole run.
fn strip_enclosing<'a, 'b>(mut texts: &'b [&'a str]) -> &'b [&'a str] {
    while texts.len() >= 2 && texts[0] == "(" && closes_at(texts, 0) == Some(texts.len() - 1) {
        texts = &texts[1..texts.len() - 1];
    }
    texts
}

fn closes_at(texts: &[&str], open: usize) -> Option<usize> {
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

/// A positive decimal integer literal, with an optional integer suffix:
/// `1`, `2u`, `16UL`. `0`, `00`, octal-looking and hexadecimal spellings are
/// not claimed.
fn is_positive_literal(texts: &[&str]) -> bool {
    let [literal] = strip_enclosing(texts) else {
        return false;
    };
    let digits = literal.trim_end_matches(['u', 'U', 'l', 'L']);
    !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && !digits.starts_with('0')
}

/// A name the rule may step in place: `SCALAR *` or `struct T *`.
fn steppable(name: &str, types: &FunctionTypes) -> bool {
    matches!(
        types.pointer_kind(name),
        Some(PointerKind::Scalar | PointerKind::Record)
    )
}

/// The one name a `++`/`--` applies to, and whether it increments.
fn unary_step<'a>(texts: &[&'a str]) -> Option<(&'a str, bool)> {
    let texts = strip_enclosing(texts);
    let (operand, increment) = match texts {
        ["+", "+", rest @ ..] => (rest, true),
        ["-", "-", rest @ ..] => (rest, false),
        [rest @ .., "+", "+"] => (rest, true),
        [rest @ .., "-", "-"] => (rest, false),
        _ => return None,
    };
    match strip_enclosing(operand) {
        [name] if is_name(name) => Some((name, increment)),
        _ => None,
    }
}

/// Card **W43-INDEX-TRAVERSAL**: recognise one complete statement (or `for`
/// clause) that moves a pointer. See the module documentation for the forms.
pub(crate) fn pointer_step(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<PointerStepPlan> {
    if !types.is_readable()
        || texts
            .iter()
            .any(|text| matches!(*text, "{" | "}" | ";" | "," | "[" | "]" | "&"))
    {
        return None;
    }
    if let Some((name, increment)) = unary_step(texts) {
        return steppable(name, types).then(|| PointerStepPlan {
            destination: name.to_owned(),
            source: name.to_owned(),
            offset: if increment {
                StepOffset::Forward
            } else {
                StepOffset::Unknown
            },
        });
    }
    // `P += E` and `P -= E`, tokenized one character at a time.
    if let [name, operator @ ("+" | "-"), "=", amount @ ..] = texts
        && is_name(name)
        && amount.first() != Some(&"=")
    {
        if !steppable(name, types) || !is_scalar_value_expression(amount, model, types) {
            return None;
        }
        return Some(PointerStepPlan {
            destination: (*name).to_owned(),
            source: (*name).to_owned(),
            offset: if *operator == "+" && is_positive_literal(amount) {
                StepOffset::Forward
            } else {
                StepOffset::Unknown
            },
        });
    }
    // `P = Q + E`, `P = Q - E`, and the declaration `T * P = Q ± E`.
    let equal = texts.iter().position(|text| *text == "=")?;
    let (left, right) = (&texts[..equal], &texts[equal + 1..]);
    let (&destination, specifiers) = left.split_last()?;
    if !is_name(destination)
        || !steppable(destination, types)
        || right.first() == Some(&"=")
        || matches!(left.last(), Some(&("+" | "-" | "=" | "!" | "<" | ">")))
    {
        return None;
    }
    if !specifiers.is_empty()
        && (specifiers.iter().filter(|text| **text == "*").count() != 1
            || specifiers.last() != Some(&"*")
            || !specifiers
                .iter()
                .all(|text| *text == "*" || crate::legacy_c::is_identifier(text)))
    {
        return None;
    }
    let [source, operator @ ("+" | "-"), amount @ ..] = right else {
        return None;
    };
    if !is_name(source)
        || types.pointer_kind(source).is_none()
        || !is_scalar_value_expression(amount, model, types)
    {
        return None;
    }
    Some(PointerStepPlan {
        destination: destination.to_owned(),
        source: (*source).to_owned(),
        offset: if *operator == "+" && is_positive_literal(amount) {
            StepOffset::Forward
        } else {
            StepOffset::Unknown
        },
    })
}

/// Card **W43-INDEX-TRAVERSAL**: recognise one typed scalar access statement
/// that also steps a pointer it dereferences or an index it subscripts with,
/// and split it into the access over the old values and the effects that
/// follow. See the module documentation.
pub(crate) fn stepped_access(
    texts: &[&str],
    model: &TypedAccessModel,
    types: &FunctionTypes,
) -> Option<(TypedScalarPlan, Vec<TraversalEffect>)> {
    if !types.is_readable() {
        return None;
    }
    let mut rewritten: Vec<&str> = Vec::with_capacity(texts.len());
    let mut effects = Vec::new();
    let mut stepped: Vec<&str> = Vec::new();
    let mut index = 0usize;
    while index < texts.len() {
        let rest = &texts[index..];
        // `* P ++`, `* P --`, `* ( P ++ )`, `* ( P -- )`.
        let deref = match rest {
            ["*", name, "+", "+", ..] => Some((*name, true, 4)),
            ["*", name, "-", "-", ..] => Some((*name, false, 4)),
            ["*", "(", name, "+", "+", ")", ..] => Some((*name, true, 6)),
            ["*", "(", name, "-", "-", ")", ..] => Some((*name, false, 6)),
            _ => None,
        };
        if let Some((name, increment, width)) = deref {
            if !is_name(name) || types.pointer_kind(name) != Some(PointerKind::Scalar) {
                return None;
            }
            rewritten.extend(["*", name]);
            effects.push(TraversalEffect::Step(PointerStepPlan {
                destination: name.to_owned(),
                source: name.to_owned(),
                offset: if increment {
                    StepOffset::Forward
                } else {
                    StepOffset::Unknown
                },
            }));
            stepped.push(name);
            index += width;
            continue;
        }
        // `[ X ++ ]`, `[ X -- ]`, `[ ++ X ]`, `[ -- X ]`.
        let subscript = match rest {
            ["[", name, "+", "+", "]", ..]
            | ["[", name, "-", "-", "]", ..]
            | ["[", "+", "+", name, "]", ..]
            | ["[", "-", "-", name, "]", ..] => Some(*name),
            _ => None,
        };
        if let Some(name) = subscript {
            if !is_name(name) || !types.declared_is_scalar(name) {
                return None;
            }
            rewritten.extend(["[", name, "]"]);
            effects.push(TraversalEffect::Forget(name.to_owned()));
            stepped.push(name);
            index += 5;
            continue;
        }
        rewritten.push(texts[index]);
        index += 1;
    }
    if effects.is_empty()
        || rewritten
            .windows(2)
            .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
    {
        return None;
    }
    // A stepped name read or written anywhere else in the statement is an
    // unsequenced access to it (C11 6.5p2), and two steps of one name are
    // two unsequenced side effects: both are refused rather than ordered.
    for name in &stepped {
        if rewritten.iter().filter(|text| *text == name).count() != 1 {
            return None;
        }
    }
    let plan = typed_scalar_statement(&rewritten, model, types)?;
    if plan
        .destination
        .as_deref()
        .is_some_and(|destination| stepped.contains(&destination))
    {
        return None;
    }
    Some((plan, effects))
}

/// W48 bottom-up batch 2: a scalar-pointer formal that the body only reads
/// through and steps itself with the result discarded. No copy, address, call actual, store or other
/// binding use is admitted. The caller additionally checks its own contract
/// has no entry custody/out-slot mode and the declaration is a scalar pointer.
pub(crate) fn read_only_formal(function: &serde_json::Value, name: &str) -> bool {
    use crate::typed_c_ast::{children, kind};
    use serde_json::Value;

    fn peel(mut node: &Value) -> &Value {
        while matches!(kind(node), "ImplicitCastExpr" | "ParenExpr") && children(node).len() == 1 {
            node = &children(node)[0];
        }
        node
    }
    fn reference(node: &Value, id: &str) -> bool {
        let node = peel(node);
        kind(node) == "DeclRefExpr" && node["referencedDecl"]["id"].as_str() == Some(id)
    }
    fn walk(node: &Value, name: &str, id: &str, discarded: bool) -> bool {
        if kind(node) == "DeclRefExpr" && node["referencedDecl"]["name"].as_str() == Some(name) {
            return false;
        }
        // Clang's lvalue-to-rvalue conversion is the distinction between a
        // scalar read and a store/address of the dereferenced object.
        if kind(node) == "ImplicitCastExpr"
            && node["castKind"].as_str() == Some("LValueToRValue")
            && children(node).len() == 1
        {
            let operand = peel(&children(node)[0]);
            if kind(operand) == "UnaryOperator"
                && operand["opcode"].as_str() == Some("*")
                && children(operand).len() == 1
                && reference(&children(operand)[0], id)
            {
                return true;
            }
        }
        if kind(node) == "UnaryOperator"
            && matches!(node["opcode"].as_str(), Some("++" | "--"))
            && children(node).len() == 1
            && reference(&children(node)[0], id)
        {
            return discarded;
        }
        // A step's pointer result may not be handed to a call, binding,
        // return or store. Track only whole discarded statement positions;
        // nested steps (including `*s++`) are outside this bounded frame.
        let inner = children(node);
        inner.iter().enumerate().all(|(index, child)| {
            let statement = discarded
                && match kind(node) {
                    "CompoundStmt" => true,
                    "IfStmt" => {
                        let arms = if node["hasElse"].as_bool() == Some(true) {
                            2
                        } else {
                            1
                        };
                        index >= inner.len().saturating_sub(arms)
                    }
                    "ForStmt" => inner.len() == 5 && matches!(index, 0 | 3 | 4),
                    "WhileStmt" | "SwitchStmt" | "LabelStmt" | "CaseStmt" | "DefaultStmt"
                    | "AttributedStmt" => index + 1 == inner.len(),
                    "DoStmt" => index == 0,
                    "ParenExpr" => inner.len() == 1,
                    _ => false,
                };
            walk(child, name, id, statement)
        })
    }
    let formals = children(function)
        .iter()
        .filter(|node| kind(node) == "ParmVarDecl" && node["name"].as_str() == Some(name))
        .collect::<Vec<_>>();
    let [formal] = formals.as_slice() else {
        return false;
    };
    let Some(id) = formal["id"].as_str() else {
        return false;
    };
    let Some(body) = children(function)
        .iter()
        .find(|node| kind(node) == "CompoundStmt")
    else {
        return false;
    };
    walk(body, name, id, true)
}

/// `QCore.IndexTraversal.formalStepEligible`, after exact self-step matching.
pub(crate) fn formal_step_eligible(scalar_pointer: bool, fixed: bool, read_only: bool) -> bool {
    scalar_pointer && !fixed && read_only
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(source: &str) -> Vec<&str> {
        let bytes = source.as_bytes();
        let mut tokens = Vec::new();
        let mut cursor = 0;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if byte.is_ascii_whitespace() {
                cursor += 1;
            } else if byte.is_ascii_alphanumeric() || byte == b'_' {
                let start = cursor;
                while cursor < bytes.len()
                    && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
                {
                    cursor += 1;
                }
                tokens.push(&source[start..cursor]);
            } else {
                tokens.push(&source[cursor..cursor + 1]);
                cursor += 1;
            }
        }
        tokens
    }

    fn facts(parameters: &str, body: &str) -> (TypedAccessModel, FunctionTypes) {
        let unit = "struct rec { int count; struct rec *next; };";
        let model = TypedAccessModel::read(&texts(unit));
        let types = FunctionTypes::read(
            &model,
            &texts(parameters),
            &texts(body),
            &std::collections::BTreeSet::new(),
        );
        (model, types)
    }

    const PARAMETERS: &str = "char * s , unsigned long n , struct rec * r , int * v";
    const BODY: &str = "char * p ; char buf [ 8 ] ; unsigned long i ; int k ;";

    fn step(statement: &str) -> Option<(String, String, StepOffset)> {
        let (model, types) = facts(PARAMETERS, BODY);
        pointer_step(&texts(statement), &model, &types)
            .map(|plan| (plan.destination, plan.source, plan.offset))
    }

    #[test]
    fn the_step_forms_are_recognised() {
        let forward = StepOffset::Forward;
        let unknown = StepOffset::Unknown;
        let own = |name: &str, offset| Some((name.to_owned(), name.to_owned(), offset));
        assert_eq!(step("p ++"), own("p", forward));
        assert_eq!(step("++ p"), own("p", forward));
        assert_eq!(step("( p ) ++"), own("p", forward));
        assert_eq!(step("p --"), own("p", unknown));
        assert_eq!(step("-- s"), own("s", unknown));
        assert_eq!(step("r ++"), own("r", forward));
        assert_eq!(step("p += 2"), own("p", forward));
        assert_eq!(step("p += 2UL"), own("p", forward));
        assert_eq!(step("p += 0"), own("p", unknown));
        assert_eq!(step("p += n"), own("p", unknown));
        assert_eq!(step("p -= 1"), own("p", unknown));
        assert_eq!(step("p += ( unsigned long ) k * 2"), own("p", unknown));
        assert_eq!(
            step("p = s + 1"),
            Some(("p".to_owned(), "s".to_owned(), forward))
        );
        assert_eq!(
            step("p = buf + n"),
            Some(("p".to_owned(), "buf".to_owned(), unknown))
        );
        assert_eq!(
            step("char * p = s - 1"),
            Some(("p".to_owned(), "s".to_owned(), unknown))
        );
        assert_eq!(
            step("const char * p = s + 3"),
            Some(("p".to_owned(), "s".to_owned(), forward))
        );
    }

    #[test]
    fn everything_else_is_refused() {
        for statement in [
            "i ++",            // a scalar: the existing ForgetScalar route
            "buf ++",          // an array is not stepped in place
            "v ++",            // `int *` is SCALAR *: steppable
            "p = s",           // a copy is the Alias route
            "p = s + * v",     // an access in the offset
            "p = s + i ++",    // a write in the offset
            "p = f ( s ) + 1", // a call
            "p += n , i ++",   // a comma
            "p = s + 1 + 2",   // still a scalar amount: `1 + 2`
            "p = ( char * ) s + 1",
            "p = & buf [ 1 ]",
            "* p ++",
            "q ++",      // undeclared
            "p = n + 1", // the source is a scalar
            "char * p , * q = s + 1",
        ] {
            let got = step(statement);
            let expected_admitted = matches!(statement, "v ++" | "p = s + 1 + 2");
            assert_eq!(got.is_some(), expected_admitted, "{statement}: {got:?}");
        }
    }

    fn stepped(statement: &str) -> Option<(Vec<String>, Vec<TraversalEffect>)> {
        let (model, types) = facts(PARAMETERS, BODY);
        stepped_access(&texts(statement), &model, &types)
            .map(|(plan, effects)| (plan.bases, effects))
    }

    #[test]
    fn a_stepped_access_splits_into_the_access_and_its_steps() {
        let step_p = TraversalEffect::Step(PointerStepPlan {
            destination: "p".to_owned(),
            source: "p".to_owned(),
            offset: StepOffset::Forward,
        });
        assert_eq!(
            stepped("* p ++ = 0"),
            Some((vec!["p".to_owned()], vec![step_p.clone()]))
        );
        assert_eq!(
            stepped("* ( p ++ ) = k"),
            Some((vec!["p".to_owned()], vec![step_p.clone()]))
        );
        assert_eq!(
            stepped("k = * p ++"),
            Some((vec!["p".to_owned()], vec![step_p.clone()]))
        );
        assert_eq!(
            stepped("* p ++ = * s ++"),
            Some((
                vec!["p".to_owned(), "s".to_owned()],
                vec![
                    step_p,
                    TraversalEffect::Step(PointerStepPlan {
                        destination: "s".to_owned(),
                        source: "s".to_owned(),
                        offset: StepOffset::Forward,
                    })
                ]
            ))
        );
        assert_eq!(
            stepped("buf [ i ++ ] = 0"),
            Some((
                vec!["buf".to_owned()],
                vec![TraversalEffect::Forget("i".to_owned())]
            ))
        );
        assert_eq!(
            stepped("k = s [ -- i ]"),
            Some((
                vec!["s".to_owned()],
                vec![TraversalEffect::Forget("i".to_owned())]
            ))
        );
    }

    #[test]
    fn an_unsequenced_or_untyped_stepped_access_is_refused() {
        for statement in [
            "* p ++ = p [ 0 ]", // the stepped pointer read again
            "* p ++ = * p ++",  // stepped twice
            "buf [ i ++ ] = i", // the index read again
            "i = s [ i ++ ]",   // the index is the destination
            "* r ++ = 0",       // not a scalar pointer
            "* p ++",           // no assignment: not a typed statement form
            "p ++",             // no access
            "* p ++ = f ( 0 )", // a call
            "buf [ i ++ + 1 ] = 0",
        ] {
            assert_eq!(stepped(statement), None, "{statement}");
        }
    }
}

#[cfg(test)]
mod formal_tests {
    #[test]
    fn a_formal_frame_requires_each_step_result_to_be_discarded() {
        if let Err(reason) = crate::clang::command() {
            assert!(std::env::var_os("QCORE_CLANG").is_none(), "{reason}");
            eprintln!("skipping Clang-dependent formal frame test: {reason}");
            return;
        }
        for (definition, expected) in [
            ("void w(char *s) { while (*s) s++; }", true),
            ("void w(char *s) { for (; *s; s++) {} }", true),
            ("void w(char *s) { if (*s) s++; else s--; }", true),
            ("void w(char *s) { do { s++; } while (*s); }", true),
            ("void w(char *s) { note(s++); }", false),
            ("void w(char *s) { char *q = s++; }", false),
            ("void w(char *s) { global = s++; }", false),
            ("char *w(char *s) { return s++; }", false),
            ("void w(char *s) { int x = *s++; }", false),
        ] {
            let source = format!("void note(char *); char *global; {definition}");
            let _scope = crate::typed_access_path::enter_source(
                &source,
                crate::legacy_c::CCompilerDialect::Gnu17,
                None,
            );
            let index = crate::typed_access_path::StatementIndex::read("w")
                .unwrap_or_else(|| panic!("no function AST: {source}"));
            assert_eq!(
                super::read_only_formal(&index.function, "s"),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn formal_step_vectors_match_lean_and_cover_every_boolean_input() {
        let lean = include_str!("../../../proof/QCore/IndexTraversal.lean");
        let rows = lean
            .split("def formalStepVectors")
            .nth(1)
            .unwrap()
            .split("theorem formal_step_vectors_hold")
            .next()
            .unwrap();
        let bits = rows
            .split(|c: char| !c.is_alphabetic())
            .filter_map(|word| match word {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(bits.len(), 32);
        for (index, row) in bits.as_chunks::<4>().0.iter().enumerate() {
            assert_eq!(
                [row[0], row[1], row[2]],
                [index & 4 != 0, index & 2 != 0, index & 1 != 0]
            );
            assert_eq!(super::formal_step_eligible(row[0], row[1], row[2]), row[3]);
        }
    }
}
