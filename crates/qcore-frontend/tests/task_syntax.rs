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

fn all_statements(statements: &[TypedStatement]) -> Vec<&TypedStatement> {
    statements
        .iter()
        .flat_map(|statement| {
            let mut result = vec![statement];
            result.extend(all_statements(&statement.nested));
            result
        })
        .collect()
}

#[test]
fn ordered_captures_and_share_place_trees_retain_exact_spans() {
    let source = "#import std.concurrency; fn work(a: i32, b: i32, c: i32, d: i32) -> unit { return; } fn main(x: i32, p: i32, a: i32, n: i32, k: i32) -> unit { thread scope s { let h = spawn s work(copy x, move *p, read a[n].field share (k + 1), write a[0])?; join move h; } }";
    let parsed = parse_source_program(source).unwrap();
    assert_eq!(parsed.task_expressions().len(), 2);
    let task = &parsed.task_expressions()[0];
    assert_eq!(
        &source[task.span.start..task.span.end],
        "spawn s work(copy x, move *p, read a[n].field share (k + 1), write a[0])?"
    );
    assert!(task.propagates_error);
    let SourceTaskExpressionKind::Spawn {
        scope,
        worker,
        captures,
    } = &task.kind
    else {
        panic!()
    };
    assert_eq!(scope.value, "s");
    assert_eq!(worker.value, "work");
    assert_eq!(captures.len(), 4);
    assert!(matches!(captures[0].value.mode, SourceCaptureMode::Copy));
    assert!(matches!(captures[1].value.mode, SourceCaptureMode::Move));
    assert!(matches!(
        captures[1].value.place.value,
        SourcePlace::Dereference(_)
    ));
    assert!(matches!(
        captures[2].value.mode,
        SourceCaptureMode::Read {
            share: Located {
                value: SourceShareExpression::Parenthesized(_),
                ..
            }
        }
    ));
    assert!(matches!(
        captures[2].value.place.value,
        SourcePlace::Field { .. }
    ));
    assert!(matches!(captures[3].value.mode, SourceCaptureMode::Write));
    for capture in captures {
        assert_eq!(
            &source[capture.value.place.value.root().span.start
                ..capture.value.place.value.root().span.end],
            capture.value.place.value.root().value
        );
    }
    let typed = elaborate_source(&parsed).unwrap();
    let main = typed
        .declarations
        .iter()
        .find(|d| d.name == "main")
        .unwrap();
    let tasks = all_statements(&main.statements)
        .into_iter()
        .flat_map(|s| &s.task_expressions)
        .collect::<Vec<_>>();
    let ResolvedTaskExpressionKind::Spawn {
        scope,
        worker,
        captures,
    } = &tasks[0].kind
    else {
        panic!()
    };
    assert_eq!(*worker, typed.globals["work"]);
    assert_ne!(*scope, *worker);
    assert_eq!(
        captures
            .iter()
            .map(|c| c.place.root_symbol)
            .collect::<Vec<_>>(),
        vec![
            main.parameters[0].symbol,
            main.parameters[1].symbol,
            main.parameters[2].symbol,
            main.parameters[2].symbol
        ]
    );
    assert!(matches!(
        tasks[1].kind,
        ResolvedTaskExpressionKind::Join { .. }
    ));
}

#[test]
fn malformed_task_and_capture_grammar_is_rejected() {
    for expression in [
        "spawn s work(x)",
        "spawn s work(move x + 1)",
        "spawn s work(copy make())",
        "spawn s work(read x)",
        "spawn s work(read x share)",
        "spawn s work(read x share 1 * 2)",
        "spawn s work(write x share 1)",
        "spawn s work(copy x,)",
        "spawn s (copy x)",
        "spawn s work",
        "spawn s work(move)",
        "join h",
        "join copy h",
        "join move",
        "join move h[]",
    ] {
        let source = format!("fn main() -> unit {{ {expression}; }}");
        assert!(
            parse_source_program(&source).is_err(),
            "accepted {expression}"
        );
    }
}

#[test]
fn scope_and_worker_roles_cannot_be_satisfied_by_other_visible_names() {
    for (declarations, body, expected) in [
        (
            "fn work(x: i32) -> unit { return; }",
            "let s = 1; let h = spawn s work(copy x);",
            "live enclosing thread scope",
        ),
        (
            "fn work(x: i32) -> unit { return; }",
            "thread scope s { } let h = spawn s work(copy x);",
            "live enclosing thread scope",
        ),
        (
            "fn work(x: i32) -> unit { return; }",
            "thread scope s { if (true) { let s = 1; let h = spawn s work(copy x); } }",
            "live enclosing thread scope",
        ),
        (
            "extern fn work(x: i32) -> unit;",
            "thread scope s { let h = spawn s work(copy x); }",
            "first-order source function",
        ),
        (
            "const work: i32 = 0;",
            "thread scope s { let h = spawn s work(copy x); }",
            "first-order source function",
        ),
        (
            "fn work(x: i32) -> unit { return; }",
            "thread scope s { let work = 1; let h = spawn s work(copy x); }",
            "first-order source function",
        ),
        (
            "fn work(x: i32) -> unit { return; }",
            "thread scope s { let h = spawn s work(); }",
            "expects 1 captures",
        ),
    ] {
        let parsed = parse_source_program(&format!(
            "#import std.concurrency; {declarations} fn main(x: i32) -> unit {{ {body} }}"
        ))
        .unwrap();
        let errors = elaborate_source(&parsed).unwrap_err();
        assert!(
            errors.iter().any(|error| error.message.contains(expected)),
            "{body}: {errors:?}"
        );
    }
}

#[test]
fn unknown_capture_index_share_and_handle_names_are_not_erased() {
    for body in [
        "let h = spawn s work(copy absent);",
        "let h = spawn s work(copy x[absent]);",
        "let h = spawn s work(read x share absent);",
        "join move absent;",
    ] {
        let source = format!(
            "#import std.concurrency; fn work(x: i32) -> unit {{ return; }} fn main(x: i32) -> unit {{ thread scope s {{ {body} }} }}"
        );
        let errors = elaborate_source(&parse_source_program(&source).unwrap()).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("absent")));
    }
}

#[test]
fn nested_scopes_resolve_by_identity_and_inner_binding_does_not_escape() {
    let source = "#import std.concurrency; fn work(x: i32) -> unit { return; } fn main(x: i32) -> unit { thread scope s { let a = spawn s work(copy x)?; thread scope t { let b = spawn t work(copy x)?; join move b; } join move a; } }";
    let typed = elaborate_source(&parse_source_program(source).unwrap()).unwrap();
    let main = typed
        .declarations
        .iter()
        .find(|d| d.name == "main")
        .unwrap();
    let tasks = all_statements(&main.statements)
        .into_iter()
        .flat_map(|s| &s.task_expressions)
        .collect::<Vec<_>>();
    let ResolvedTaskExpressionKind::Spawn { scope: outer, .. } = tasks[0].kind else {
        panic!()
    };
    let ResolvedTaskExpressionKind::Spawn { scope: inner, .. } = tasks[1].kind else {
        panic!()
    };
    assert_ne!(outer, inner);
    let bad = source.replace("join move a", "join move b");
    assert!(elaborate_source(&parse_source_program(&bad).unwrap()).is_err());
}

#[test]
fn repeated_capture_syntax_is_retained_without_fabricating_custody_validation() {
    let source = "#import std.concurrency; fn work(a: i32, b: i32) -> unit { return; } fn main(x: i32) -> unit { thread scope s { let h = spawn s work(move x, move x); join move h; } }";
    let typed = elaborate_source(&parse_source_program(source).unwrap()).unwrap();
    let main = typed
        .declarations
        .iter()
        .find(|d| d.name == "main")
        .unwrap();
    let tasks = all_statements(&main.statements)
        .into_iter()
        .flat_map(|s| &s.task_expressions)
        .collect::<Vec<_>>();
    let ResolvedTaskExpressionKind::Spawn { captures, .. } = &tasks[0].kind else {
        panic!()
    };
    assert_eq!(captures.len(), 2);
    assert_eq!(captures[0].place.root_symbol, captures[1].place.root_symbol);
    assert!(!tasks[0].syntax.propagates_error);
    // Structural retention explicitly does not certify duplicate move captures
    // or use of an undiscriminated spawn result as a handle.
}

#[test]
fn shadowed_handle_binders_are_distinct_and_join_resolves_the_live_declaration() {
    let source = "#import std.concurrency; fn work(x: i32) -> unit { return; } fn main(x: i32) -> unit { thread scope s { let h = spawn s work(copy x)?; thread scope t { let h = spawn t work(copy x)?; join move h; } join move h; } }";
    let typed = elaborate_source(&parse_source_program(source).unwrap()).unwrap();
    let main = typed
        .declarations
        .iter()
        .find(|d| d.name == "main")
        .unwrap();
    let statements = all_statements(&main.statements);
    let task_statements = statements
        .into_iter()
        .filter(|s| !s.task_expressions.is_empty())
        .collect::<Vec<_>>();
    let outer = task_statements[0].binding.unwrap();
    let inner = task_statements[1].binding.unwrap();
    assert_ne!(outer, inner);
    for (statement, expected) in [(task_statements[2], inner), (task_statements[3], outer)] {
        assert_eq!(statement.binding, None);
        let ResolvedTaskExpressionKind::Join { handle } = &statement.task_expressions[0].kind
        else {
            panic!()
        };
        assert_eq!(handle.root_symbol, expected);
    }
}

#[test]
fn initializer_references_precede_its_new_binder_and_assignment_does_not_rebind() {
    let source = "#import std.concurrency; fn work(x: i32) -> unit { return; } fn main(x: i32) -> unit { thread scope s { let h = spawn s work(copy x)?; thread scope t { let h = join move h; h = x; } join move h; } }";
    let typed = elaborate_source(&parse_source_program(source).unwrap()).unwrap();
    let main = typed
        .declarations
        .iter()
        .find(|d| d.name == "main")
        .unwrap();
    let statements = all_statements(&main.statements);
    let outer = statements
        .iter()
        .find(|s| {
            s.task_expressions
                .iter()
                .any(|t| matches!(t.kind, ResolvedTaskExpressionKind::Spawn { .. }))
        })
        .unwrap()
        .binding
        .unwrap();
    let initializer = statements
        .iter()
        .find(|s| {
            s.binding.is_some()
                && s.task_expressions
                    .iter()
                    .any(|t| matches!(t.kind, ResolvedTaskExpressionKind::Join { .. }))
        })
        .unwrap();
    let inner = initializer.binding.unwrap();
    assert_ne!(inner, outer);
    let ResolvedTaskExpressionKind::Join { handle } = &initializer.task_expressions[0].kind else {
        panic!()
    };
    assert_eq!(handle.root_symbol, outer);
    assert!(initializer.referenced_symbols.contains(&outer));
    assert!(!initializer.referenced_symbols.contains(&inner));
    let assignment = statements
        .iter()
        .find(|s| &source[s.span.start..s.span.end] == "h = x;")
        .unwrap();
    assert_eq!(assignment.binding, None);
    assert!(assignment.referenced_symbols.contains(&inner));
    // These are name-resolution assertions, not acceptance of the program's
    // intentionally invalid repeated join or task-result assignment semantics.
    let self_reference = "fn main() -> unit { let h = join move h; }";
    assert!(elaborate_source(&parse_source_program(self_reference).unwrap()).is_err());
}
