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

use qcore_frontend::frontend::*;

fn statements(all: &[TypedStatement]) -> Vec<&TypedStatement> {
    all.iter()
        .flat_map(|statement| {
            let mut result = vec![statement];
            result.extend(statements(&statement.nested));
            result
        })
        .collect()
}

fn typed_matches(typed: &TypedSourceProgram) -> Vec<&ResolvedResultMatch> {
    typed
        .declarations
        .iter()
        .flat_map(|d| statements(&d.statements))
        .filter_map(|s| s.result_match.as_ref())
        .collect()
}

fn arm_statements<'a>(
    typed: &'a TypedSourceProgram,
    matched: &ResolvedResultMatch,
    arm: &ResolvedResultMatchArm,
) -> &'a [TypedStatement] {
    let owner = typed
        .declarations
        .iter()
        .flat_map(|d| statements(&d.statements))
        .find(|s| s.span == matched.syntax.span)
        .unwrap();
    owner.nested.get(arm.statement_range.clone()).unwrap()
}

#[test]
fn canonical_spawn_match_preserves_complete_scrutinee_patterns_and_arms() {
    let source = "#import std.concurrency; fn worker(x: i32) -> unit { return; } fn main(x: i32) -> unit { thread scope s { match spawn s worker(copy x) { Result::Ok { value: h } => { join move h; } Result::Err { error: e } => { e; } } } }";
    let parsed = parse_source_program(source).unwrap();
    let syntax = &parsed.result_matches()[0];
    assert_eq!(syntax.scrutinee_source, "spawn s worker(copy x)");
    assert_eq!(
        &source[syntax.scrutinee_span.start..syntax.scrutinee_span.end],
        syntax.scrutinee_source
    );
    assert_eq!(syntax.arms[0].pattern_source, "Result::Ok { value: h }");
    assert_eq!(syntax.arms[1].pattern_source, "Result::Err { error: e }");
    assert_eq!(
        &source[syntax.arms[0].body.span.start..syntax.arms[0].body.span.end],
        "{ join move h; }"
    );
    let typed = elaborate_source(&parsed).unwrap();
    let matched = typed_matches(&typed)[0];
    let Some(ResolvedResultMatchScrutinee::Task(task)) = &matched.scrutinee else {
        panic!()
    };
    assert!(matches!(
        task.kind,
        ResolvedTaskExpressionKind::Spawn { .. }
    ));
    let success = &matched.arms[0];
    let error = &matched.arms[1];
    assert_ne!(success.binding.unwrap(), error.binding.unwrap());
    assert_eq!(
        success.syntax.pattern.as_ref().unwrap().kind,
        ResultPatternKind::Ok
    );
    let ResolvedTaskExpressionKind::Join { handle } =
        &arm_statements(&typed, matched, success)[0].task_expressions[0].kind
    else {
        panic!()
    };
    assert_eq!(handle.root_symbol, success.binding.unwrap());
    assert_eq!(
        arm_statements(&typed, matched, error)[0].referenced_symbols,
        vec![error.binding.unwrap()]
    );
    assert!(parsed.syntax_equivalent(&parse_source_program(&parsed.to_string()).unwrap()));
}

#[test]
fn existing_result_scrutinee_resolves_before_same_name_arm_binders() {
    let source = "fn main(result: i32) -> unit { match result { Result::Ok { value: result } => { result; } Result::Err { error: result } => { result; } } result; }";
    let typed = elaborate_source(&parse_source_program(source).unwrap()).unwrap();
    let main = &typed.declarations[0];
    let outer = main.parameters[0].symbol;
    let matched = typed_matches(&typed)[0];
    assert_eq!(
        matched.scrutinee,
        Some(ResolvedResultMatchScrutinee::Value(outer))
    );
    assert_ne!(matched.arms[0].binding, matched.arms[1].binding);
    for arm in &matched.arms {
        assert_ne!(arm.binding, Some(outer));
        assert_eq!(
            arm_statements(&typed, matched, arm)[0].referenced_symbols,
            vec![arm.binding.unwrap()]
        );
    }
    assert_eq!(main.statements[1].referenced_symbols, vec![outer]);
    // Structural resolution deliberately does not claim the i32 input is Result.
}

#[test]
fn result_arm_bindings_neither_leak_nor_resolve_their_own_scrutinee() {
    for source in [
        "fn main(r: i32) -> unit { match r { Result::Ok { value: h } => { h; } Result::Err { error: e } => { h; } } }",
        "fn main(r: i32) -> unit { match r { Result::Ok { value: h } => { h; } Result::Err { error: e } => { e; } } h; }",
        "fn main() -> unit { match h { Result::Ok { value: h } => { h; } Result::Err { error: e } => { e; } } }",
        "fn main(r: i32) -> unit { match r { Result::Ok { value: h } => { let h = 1; } Result::Err { error: e } => { e; } } }",
    ] {
        let parsed = parse_source_program(source).unwrap();
        assert!(elaborate_source(&parsed).is_err(), "accepted {source}");
    }
}

#[test]
fn nested_match_and_scope_shadowing_keep_each_binder_identity() {
    let source = "#import std.concurrency; fn main(r: i32) -> unit { match r { Result::Ok { value: h } => { thread scope s { let h = h; h; } match h { Result::Ok { value: h } => { h; } Result::Err { error: e } => { e; } } } Result::Err { error: e } => { e; } } }";
    let typed = elaborate_source(&parse_source_program(source).unwrap()).unwrap();
    let matches = typed_matches(&typed);
    assert_eq!(matches.len(), 2);
    let outer = matches[0].arms[0].binding.unwrap();
    let scope = &arm_statements(&typed, matches[0], &matches[0].arms[0])[0];
    assert!(scope.body_binding.is_some());
    let local = &scope.nested[0];
    assert_ne!(local.binding, Some(outer));
    assert_eq!(local.referenced_symbols, vec![outer]);
    assert_eq!(
        scope.nested[1].referenced_symbols,
        vec![local.binding.unwrap()]
    );
    assert_eq!(
        matches[1].scrutinee,
        Some(ResolvedResultMatchScrutinee::Value(outer))
    );
    assert_ne!(matches[1].arms[0].binding, Some(outer));
}

#[test]
fn direct_join_match_is_resolved_without_implicit_propagation() {
    let source = "#import std.concurrency; fn worker(x: i32) -> unit { return; } fn main(x: i32) -> unit { thread scope s { let h = spawn s worker(copy x)?; match join move h { Result::Ok { value: value } => { value; } Result::Err { error: e } => { e; } } } }";
    let typed = elaborate_source(&parse_source_program(source).unwrap()).unwrap();
    let matched = typed_matches(&typed)[0];
    let Some(ResolvedResultMatchScrutinee::Task(task)) = &matched.scrutinee else {
        panic!()
    };
    assert!(matches!(task.kind, ResolvedTaskExpressionKind::Join { .. }));
    assert!(!task.syntax.propagates_error);
}

#[test]
fn unsupported_patterns_and_compound_scrutinees_are_retained_without_classification() {
    for pattern in [
        "Result::Ok(h)",
        "Other::Ok { value: h }",
        "Result::Ok { error: h }",
        "Result::Ok { value: h, extra: h }",
        "Result::Ok { value: (h) }",
        "Result::Ok { value: h } if true",
        "Result::Ok { value: _ }",
    ] {
        let source = format!(
            "fn main(r: i32) -> unit {{ match r {{ {pattern} => {{ }} Result::Err {{ error: e }} => {{ }} }} }}"
        );
        let parsed = parse_source_program(&source).unwrap();
        let first = &parsed.result_matches()[0].arms[0];
        assert_eq!(first.pattern_source, pattern);
        assert!(first.pattern.is_none(), "classified {pattern}");
        let typed = elaborate_source(&parsed).unwrap();
        assert!(typed_matches(&typed)[0].arms[0].binding.is_none());
    }
    let source = "fn main(r: i32) -> unit { match r + 0 { Result::Ok { value: h } => { } Result::Err { error: e } => { } } }";
    let typed = elaborate_source(&parse_source_program(source).unwrap()).unwrap();
    let matched = typed_matches(&typed)[0];
    assert_eq!(matched.syntax.scrutinee_source, "r + 0");
    assert!(matched.scrutinee.is_none());
}

#[test]
fn duplicate_or_missing_arms_remain_visible_for_exhaustiveness_checker() {
    let source = "fn main(r: i32) -> unit { match r { Result::Ok { value: h } => { h; } Result::Ok { value: other } => { other; } } }";
    let typed = elaborate_source(&parse_source_program(source).unwrap()).unwrap();
    let matched = typed_matches(&typed)[0];
    assert_eq!(matched.arms.len(), 2);
    assert!(
        matched
            .arms
            .iter()
            .all(|arm| arm.syntax.pattern.as_ref().unwrap().kind == ResultPatternKind::Ok)
    );
    assert_ne!(matched.arms[0].binding, matched.arms[1].binding);
    // Canonical BNF has no commas between arms. The broad structural parser
    // retains a comma as unsupported pattern syntax; it is never stripped.
    let source = "fn main(r: i32) -> unit { match r { Result::Ok { value: h } => { }, Result::Err { error: e } => { } } }";
    let parsed = parse_source_program(source).unwrap();
    assert!(
        parsed.result_matches()[0].arms[1]
            .pattern_source
            .starts_with(',')
    );
    assert!(parsed.result_matches()[0].arms[1].pattern.is_none());
}

#[test]
fn arm_binders_cannot_impersonate_scope_or_worker_roles() {
    for body in [
        "match x { Result::Ok { value: s } => { let h = spawn s worker(copy x); } Result::Err { error: e } => { } }",
        "match x { Result::Ok { value: worker } => { let h = spawn s worker(copy x); } Result::Err { error: e } => { } }",
    ] {
        let source = format!(
            "#import std.concurrency; fn worker(x: i32) -> unit {{ return; }} fn main(x: i32) -> unit {{ thread scope s {{ {body} }} }}"
        );
        assert!(elaborate_source(&parse_source_program(&source).unwrap()).is_err());
    }
    let source = "#import std.concurrency; fn worker(x: i32) -> unit { return; } fn main(x: i32) -> unit { thread scope s { match x { Result::Ok { value: worker } => { worker; } Result::Err { error: s } => { s; } } let h = spawn s worker(copy x); } }";
    elaborate_source(&parse_source_program(source).unwrap()).unwrap();
}

#[test]
fn nested_match_metadata_indexes_one_canonical_typed_tree() {
    let depth = 20;
    let mut body = "r;".to_string();
    for _ in 0..depth {
        body = format!(
            "match r {{ Result::Ok {{ value: h }} => {{ {body} }} Result::Err {{ error: e }} => {{ e; }} }}"
        );
    }
    let source = format!("fn main(r: i32) -> unit {{ {body} }}");
    let parsed = parse_source_program(&source).unwrap();
    let typed = elaborate_source(&parsed).unwrap();
    assert_eq!(typed, elaborate_source(&parsed).unwrap());
    let all = statements(&typed.declarations[0].statements);
    assert_eq!(all.len(), 2 * depth + 1);
    assert_eq!(typed_matches(&typed).len(), depth);
    for owner in all {
        if let Some(matched) = &owner.result_match {
            let mut end = 0;
            for arm in &matched.arms {
                assert_eq!(arm.statement_range.start, end);
                let children = owner.nested.get(arm.statement_range.clone()).unwrap();
                assert_eq!(children.len(), arm.syntax.body.statements.len());
                for child in children {
                    assert!(child.span.start >= arm.syntax.body.span.start);
                    assert!(child.span.end <= arm.syntax.body.span.end);
                }
                end = arm.statement_range.end;
            }
            assert_eq!(end, owner.nested.len());
        }
    }
}
