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

// CG09-TYPED-SOURCE-CONTROL (rule 11l): route T reads the control
// statements, admits only what the calling consumer lists, lowers the acyclic
// forms into guarded paths that end every left scope's locals, and refuses
// every back-edge by name.
use super::control::{CONTROL_STATEMENTS, LEGACY_STATEMENTS};
use super::*;

/// A test consumer that lists every acyclic form (as the Monkey wrapper
/// projection does).
const ACYCLIC: ControlAdmission = ControlAdmission::new(
    "CG09 test consumer",
    &["switch", "break", "continue", "goto"],
);

const DECLARATIONS: &str = "void cg_1(void); void cg_2(void); void cg_3(void); \
void cg_7(void); void q_free(void *p);\n";

fn import(c: &str, control: &ControlAdmission) -> Result<Result<SourceSummary, String>, String> {
    let source = format!("{DECLARATIONS}{c}");
    let results = import_source_functions_admitting_control(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "cg09-fixture",
        &["f"],
        &SourceImportProfile {
            records: &[],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
        &NodeAdmission::LEGACY,
        control,
    )?;
    Ok(match results.into_iter().next().expect("one result") {
        SourceResult::Summary(summary) => Ok(summary),
        SourceResult::Unknown { reason, .. } => Err(reason),
    })
}

fn admitted(c: &str) -> SourceSummary {
    import(c, &ACYCLIC)
        .unwrap()
        .unwrap_or_else(|reason| panic!("refused: {reason}\n{c}"))
}

fn refused(c: &str, control: &ControlAdmission) -> String {
    match import(c, control).unwrap() {
        Ok(summary) => panic!("expected a refusal: {c}\n{summary:?}"),
        Err(reason) => reason,
    }
}

fn not_admitted(form: &str, consumer: &str) -> String {
    format!("typed-source control `{form}` is not admitted by the {consumer}")
}

fn back_edge(form: &str, consumer: &str) -> String {
    format!(
        "typed-source control `{form}` has a back-edge, and the {consumer} supplies no \
         invariant or summary for it (CG27)"
    )
}

const LEGACY: &str = "legacy typed-source consumer";
const TEST: &str = "CG09 test consumer";

/// Each path's calls, by name, in order.
fn calls(path: &SourcePath) -> Vec<String> {
    path.operations
        .iter()
        .filter_map(|operation| match operation {
            SourceOperation::Call { function, .. } => Some(function.clone()),
            _ => None,
        })
        .collect()
}

/// Each path's `selector == constant` guards: (constant, taken).
fn case_guards(path: &SourcePath) -> Vec<(u64, bool)> {
    path.operations
        .iter()
        .filter_map(|operation| match operation {
            SourceOperation::Guard {
                condition:
                    SourceValue {
                        expression:
                            SourceExpression::Binary {
                                operator, right, ..
                            },
                        ..
                    },
                taken,
            } if operator == "==" => match &right.expression {
                SourceExpression::Integer { value } => Some((value.parse().unwrap(), *taken)),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn local_ends(path: &SourcePath) -> Vec<usize> {
    path.operations
        .iter()
        .filter_map(|operation| match operation {
            SourceOperation::LocalEnd { local } => Some(*local),
            _ => None,
        })
        .collect()
}

#[test]
fn cg09_legacy_consumer_refuses_each_form_by_name() {
    if !crate::test_support::clang_available() {
        return;
    }
    let legacy = &ControlAdmission::LEGACY;
    for (c, form) in [
        (
            "int f(int a) { switch (a) { case 1: return 1; default: return 0; } }",
            "switch",
        ),
        (
            "int f(int a) { do { if (a) break; cg_1(); } while (0); return a; }",
            "break",
        ),
        (
            "int f(int a) { do { if (a) continue; cg_1(); } while (0); return a; }",
            "continue",
        ),
        (
            "int f(int a) { if (a) goto out; a = 1; out: return a; }",
            "goto",
        ),
        ("int f(int a) { out: return a; }", "goto"),
    ] {
        assert_eq!(refused(c, legacy), not_admitted(form, LEGACY), "{c}");
    }
    assert_eq!(
        refused("int f(int a) { while (a) { a = 0; } return a; }", legacy),
        back_edge("loop while", LEGACY)
    );
    assert_eq!(
        refused(
            "int f(int a) { do { a = 0; } while (a); return a; }",
            legacy
        ),
        back_edge("loop do", LEGACY)
    );
    assert_eq!(
        refused(
            "int f(int a) { int i; for (i = 0; i < a; i++) { a = 0; } return a; }",
            legacy
        ),
        format!(
            "{}; not a canonical scan: scan body requires one conditional early return",
            back_edge("loop for", LEGACY)
        )
    );
    // The legacy shapes are unchanged: a canonical scan and `do … while (0)`.
    assert!(
        import(
            "int f(int *p, int n) { int i; for (i = 0; i < n; i++) { if (p[i]) return i; } return -1; }",
            legacy
        )
        .unwrap()
        .unwrap()
        .has_scan_loop()
    );
    assert!(
        import(
            "int f(int a) { do { cg_1(); } while (0); return a; }",
            legacy
        )
        .unwrap()
        .is_ok()
    );
}

/// Refusal control: a `while` whose back-edge no invariant decides, in a
/// consumer that admits every acyclic form; the same for every other cycle.
#[test]
fn cg09_every_back_edge_is_refused_in_an_admitting_consumer() {
    if !crate::test_support::clang_available() {
        return;
    }
    assert_eq!(
        refused("int f(int a) { while (a) { a = 0; } return a; }", &ACYCLIC),
        back_edge("loop while", TEST)
    );
    assert_eq!(
        refused(
            "int f(int a) { do { cg_1(); } while (a); return a; }",
            &ACYCLIC
        ),
        back_edge("loop do", TEST)
    );
    assert!(
        refused("int f(int a) { for (;;) { break; } return a; }", &ACYCLIC)
            .starts_with(&back_edge("loop for", TEST))
    );
    assert_eq!(
        refused(
            "int f(int a) { top: cg_1(); if (a) goto top; return a; }",
            &ACYCLIC
        ),
        back_edge("goto backward", TEST)
    );
    // No admission can list a back-edge form, an unknown form or a duplicate:
    // the whole import fails before any function is read.
    for rows in [
        &["loop while"][..],
        &["goto backward"][..],
        &["switch", "switch"][..],
        &["while"][..],
    ] {
        let invalid = ControlAdmission::new("invalid consumer", rows);
        assert!(invalid.validate().is_err(), "{rows:?}");
        assert!(import("int f(int a) { return a; }", &invalid).is_err());
    }
    ACYCLIC.validate().unwrap();
    ControlAdmission::LEGACY.validate().unwrap();
}

#[test]
fn cg09_switch_evaluates_its_selector_once_and_lowers_every_entry() {
    if !crate::test_support::clang_available() {
        return;
    }
    let summary = admitted(
        "int f(int *p) { switch (*p) { case 1: cg_1(); break; case 2: cg_2(); break; } return 0; }",
    );
    assert_eq!(summary.paths().len(), 3);
    for path in summary.paths() {
        let reads = path
            .operations
            .iter()
            .filter(|operation| matches!(operation, SourceOperation::Read { .. }))
            .count();
        assert_eq!(reads, 1, "{path:?}");
        // Every guard compares the one read's symbol.
        for operation in &path.operations {
            if let SourceOperation::Guard { condition, .. } = operation {
                let SourceExpression::Binary { left, .. } = &condition.expression else {
                    panic!("{condition:?}")
                };
                assert_eq!(left.expression, SourceExpression::Symbol { operation: 0 });
            }
        }
    }
    let lowered = summary
        .paths()
        .iter()
        .map(|path| (case_guards(path), calls(path)))
        .collect::<Vec<_>>();
    assert_eq!(
        lowered,
        [
            (vec![(1, true)], vec!["cg_1".to_owned()]),
            (vec![(2, true)], vec!["cg_2".to_owned()]),
            (vec![(1, false), (2, false)], vec![]),
        ]
    );
    // An unsigned selector: Clang converts each case label, never the host.
    let summary = admitted("int f(unsigned a) { switch (a) { case -1: cg_1(); } return 0; }");
    let SourceOperation::Guard { condition, .. } = &summary.paths()[0].operations[0] else {
        panic!("{summary:?}")
    };
    let SourceExpression::Binary { right, .. } = &condition.expression else {
        panic!()
    };
    assert!(matches!(
        &right.expression,
        SourceExpression::Cast { cast, .. } if cast == "IntegralCast"
    ));
    assert_eq!(right.c_type, "unsigned int");
}

#[test]
fn cg09_inner_break_leaves_only_the_inner_switch() {
    if !crate::test_support::clang_available() {
        return;
    }
    let summary = admitted(
        "int f(int a, int b) { switch (a) { case 1: switch (b) { case 1: cg_1(); break; } \
         cg_2(); break; default: cg_3(); } return 0; }",
    );
    let calls = summary.paths().iter().map(calls).collect::<Vec<_>>();
    assert_eq!(
        calls,
        [
            vec!["cg_1".to_owned(), "cg_2".to_owned()],
            vec!["cg_2".to_owned()],
            vec!["cg_3".to_owned()],
        ]
    );
}

#[test]
fn cg09_do_once_takes_break_and_continue() {
    if !crate::test_support::clang_available() {
        return;
    }
    let summary = admitted(
        "int f(int a, int b) { do { if (a) break; cg_1(); if (b) continue; cg_2(); } while (0); \
         return 0; }",
    );
    // `break` skips the condition; `continue` evaluates it (a guard on the
    // literal zero, not taken); falling off the body evaluates it too.
    let rows = summary
        .paths()
        .iter()
        .map(|path| {
            let zero_guards = path
                .operations
                .iter()
                .filter(|operation| {
                    matches!(operation, SourceOperation::Guard {
                        condition: SourceValue { expression: SourceExpression::Integer { value }, .. },
                        taken: false,
                    } if value == "0")
                })
                .count();
            (calls(path), zero_guards)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            (vec![], 0),
            (vec!["cg_1".to_owned()], 1),
            (vec!["cg_1".to_owned(), "cg_2".to_owned()], 1),
        ]
    );
}

#[test]
fn cg09_goto_out_of_nested_scopes_ends_their_locals_innermost_first() {
    if !crate::test_support::clang_available() {
        return;
    }
    // Locals: a = 0 (parameter), r = 1, x = 2, y = 3.
    let summary = admitted(
        "int f(int a) { int r = 0; { int x = 1; { int y = 2; if (a) goto out; r = y; } r = x; } \
         out: return r; }",
    );
    let [jumped, fell] = summary.paths() else {
        panic!("{summary:?}")
    };
    assert_eq!(local_ends(jumped), [3, 2, 1]);
    assert_eq!(local_ends(fell), [3, 2, 1]);
    // On the jump, both scopes end before the label's `return`.
    let position = |path: &SourcePath, wanted: &SourceOperation| {
        path.operations.iter().position(|op| op == wanted).unwrap()
    };
    let returned = jumped
        .operations
        .iter()
        .position(|op| matches!(op, SourceOperation::ReturnValue { .. }))
        .unwrap();
    assert!(position(jumped, &SourceOperation::LocalEnd { local: 3 }) < returned);
    assert!(
        position(jumped, &SourceOperation::LocalEnd { local: 3 })
            < position(jumped, &SourceOperation::LocalEnd { local: 2 })
    );
    // The jump path skipped both assignments to `r`.
    let writes = |path: &SourcePath| {
        path.operations
            .iter()
            .filter(|op| matches!(op, SourceOperation::LocalWrite { local: 1, .. }))
            .count()
    };
    assert_eq!((writes(jumped), writes(fell)), (1, 3));
}

/// Refusal control: a `goto` out of a scope holding a local whose address is
/// still live. The jump ends the local, so the later dereference is refused
/// (on the fall-through path too: its scope ended as well).
#[test]
fn cg09_goto_out_of_a_scope_holding_a_live_local_is_refused() {
    if !crate::test_support::clang_available() {
        return;
    }
    assert_eq!(
        refused(
            "int f(int a) { int *p = 0; { int x = 1; p = &x; if (a) goto out; } out: return *p; }",
            &ACYCLIC
        ),
        "local storage used after its lifetime ended"
    );
    // Control: the same jump without the later use is admitted, ending `x`.
    let summary = admitted(
        "int f(int a) { int *p = 0; { int x = 1; p = &x; if (a) goto out; } out: return 0; }",
    );
    for path in summary.paths() {
        assert!(local_ends(path).contains(&2), "{path:?}");
    }
    // With no jump, the same dangling use: refused under an admitting
    // consumer, while the legacy admission keeps T's pre-CG09 projection of
    // the ended storage (the legacy consumer decides it; the Monkey callback
    // tests pin that).
    let dangling = "int f(int a) { int *p = 0; { int x = 1; p = &x; } return *p; }";
    assert_eq!(
        refused(dangling, &ACYCLIC),
        "local storage used after its lifetime ended"
    );
    assert!(import(dangling, &ControlAdmission::LEGACY).unwrap().is_ok());
}

/// Refusal control: `switch` fallthrough into a case that releases again.
/// Every consumer that decides releases imports with the legacy admission and
/// refuses the `switch` by name; the projection that admits it carries both
/// releases on the fallthrough path, so no consumer can miss the second.
#[test]
fn cg09_switch_fallthrough_into_a_second_release() {
    if !crate::test_support::clang_available() {
        return;
    }
    let c =
        "void f(int a, void *p) { switch (a) { case 1: q_free(p); case 2: q_free(p); break; } }";
    assert_eq!(
        refused(c, &ControlAdmission::LEGACY),
        not_admitted("switch", LEGACY)
    );
    let summary = admitted(c);
    let released = summary
        .paths()
        .iter()
        .map(|path| {
            path.operations
                .iter()
                .filter_map(|operation| match operation {
                    SourceOperation::Call {
                        function,
                        arguments,
                        ..
                    } if function == "q_free" => Some(arguments[0].expression.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let p = SourceExpression::Parameter { index: 1 };
    assert_eq!(released, [vec![p.clone(), p.clone()], vec![p], vec![]]);
}

#[test]
fn cg09_unsupported_shapes_are_refused_by_name() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (c, reason) in [
        (
            "int f(int a) { switch (a) { case 1 ... 3: cg_1(); } return 0; }",
            "GNU case range is outside route T",
        ),
        (
            "int f(int a) { switch (a) { int x; case 1: cg_1(); } return 0; }",
            "declaration at a switch body's top level",
        ),
        (
            "int f(int a) { switch (a) { case 1: l: cg_1(); } return 0; }",
            "label at a switch body's top level",
        ),
        (
            "int f(int a) { switch (a) { case 1: if (a) { case 2: cg_1(); } } return 0; }",
            "case label outside its switch body's top level",
        ),
        (
            "int f(int a) { goto in; if (a) { in: cg_1(); } return 0; }",
            "goto target is not a later statement of an enclosing block",
        ),
        (
            "int f(int a) { if (a) goto out; int x = 1; cg_1(); out: return 0; }",
            "goto skips a declaration in its target scope",
        ),
        (
            "int f(int a) { switch (a) { case 1: cg_1(); } switch (a) { default: goto in; } \
             { in: cg_2(); } return 0; }",
            "",
        ),
    ] {
        if reason.is_empty() {
            // Control: a forward jump into a block with no skipped declaration.
            admitted(c);
        } else {
            assert_eq!(refused(c, &ACYCLIC), reason, "{c}");
        }
    }
}

/// Budget: a `switch` whose entries exceed the path budget is Unknown.
#[test]
fn cg09_switch_path_budget_is_a_refusal() {
    if !crate::test_support::clang_available() {
        return;
    }
    let cases = (0..70)
        .map(|n| format!("case {n}: cg_1(); break;"))
        .collect::<String>();
    assert_eq!(
        refused(
            &format!("int f(int a) {{ switch (a) {{ {cases} }} return 0; }}"),
            &ACYCLIC
        ),
        "callback path limit exceeded"
    );
}

// ---------------------------------------------------------------------------
// The Lean binding.
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Term {
    List(Vec<Term>),
    Tuple(Vec<Term>),
    Atom(String),
    App(Vec<Term>),
}

fn lean_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if matches!(character, '(' | ')' | '[' | ']' | ',') || character.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            if !character.is_whitespace() {
                tokens.push(character.to_string());
            }
        } else {
            current.push(character);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn lean_term(tokens: &[String], position: &mut usize) -> Term {
    let token = tokens[*position].clone();
    *position += 1;
    let close = match token.as_str() {
        "[" => "]",
        "(" => ")",
        _ => return Term::Atom(token),
    };
    let mut elements = Vec::new();
    while tokens[*position] != close {
        let mut parts = Vec::new();
        while tokens[*position] != "," && tokens[*position] != close {
            parts.push(lean_term(tokens, position));
        }
        elements.push(if parts.len() == 1 {
            parts.pop().unwrap()
        } else {
            Term::App(parts)
        });
        if tokens[*position] == "," {
            *position += 1;
        }
    }
    *position += 1;
    if close == "]" {
        Term::List(elements)
    } else {
        Term::Tuple(elements)
    }
}

fn number(term: &Term) -> u64 {
    match term {
        Term::Atom(text) => text.parse().unwrap(),
        other => panic!("not a number: {other:?}"),
    }
}

fn list(term: &Term) -> &[Term] {
    match term {
        Term::List(items) => items,
        other => panic!("not a list: {other:?}"),
    }
}

fn tuple(term: &Term) -> &[Term] {
    match term {
        Term::Tuple(items) => items,
        other => panic!("not a tuple: {other:?}"),
    }
}

type Lowered = (Vec<(u64, bool)>, Vec<u64>);

#[test]
fn cg09_lean_vectors_match_the_rust_tables() {
    let lean = include_str!("../../../../proof/QCore/TypedSourceControl.lean");
    let section = |start: &str| {
        let body = lean.split(start).nth(1).expect(start);
        body[..body.find("\n]").unwrap()].to_owned()
    };
    let quoted = |text: &str| {
        text.split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    // The control table: spelling and back-edge fact, row for row.
    let rows = section("def controlTable : List Form := [")
        .split('⟨')
        .skip(1)
        .map(|row| {
            let (spelling, rest) = row.trim_start_matches('"').split_once('"').unwrap();
            (spelling.to_owned(), rest.contains("true"))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        CONTROL_TABLE
            .iter()
            .map(|facts| (facts.spelling.to_owned(), facts.back_edge))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        quoted(&section("def legacyStatements : List String := [")),
        LEGACY_STATEMENTS
    );
    let pairs = quoted(&section(
        "def controlStatements : List (String × String) := [",
    ));
    assert_eq!(
        pairs
            .chunks(2)
            .map(|pair| (pair[0].as_str(), pair[1].as_str()))
            .collect::<Vec<_>>(),
        CONTROL_STATEMENTS
    );
    // The budgets.
    let budgets = lean
        .split("def budgets : Usage := ⟨")
        .nth(1)
        .unwrap()
        .split('⟩')
        .next()
        .unwrap()
        .split(',')
        .map(|n| n.trim().parse::<usize>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(budgets, [MAX_PATHS, MAX_OPERATIONS, MAX_DEPTH, MAX_NODES]);
    // The switch vectors: each body runs through the importer and must lower
    // to exactly the paths `switch_vectors_hold` proves the model computes.
    if !crate::test_support::clang_available() {
        return;
    }
    let text = section("def switchVectors : List (List Item × List Lowered) := [");
    let tokens = lean_tokens(&format!("[{text}]"));
    let vectors = lean_term(&tokens, &mut 0);
    let vectors = list(&vectors);
    assert_eq!(vectors.len(), 6);
    for vector in vectors {
        let [body, expected] = tuple(vector) else {
            panic!("{vector:?}")
        };
        let c_body = list(body)
            .iter()
            .map(|item| match item {
                Term::App(parts) => match (&parts[0], &parts[1]) {
                    (Term::Atom(head), n) if head == ".case" => format!("case {}:", number(n)),
                    (Term::Atom(head), n) if head == ".stmt" => format!("cg_{}();", number(n)),
                    other => panic!("{other:?}"),
                },
                Term::Atom(head) if head == ".dflt" => "default:".to_owned(),
                Term::Atom(head) if head == ".brk" => "break;".to_owned(),
                other => panic!("{other:?}"),
            })
            .collect::<Vec<_>>()
            .join(" ");
        let expected = list(expected)
            .iter()
            .map(|path| {
                let [guards, statements] = tuple(path) else {
                    panic!("{path:?}")
                };
                let guards = list(guards)
                    .iter()
                    .map(|guard| {
                        let [constant, taken] = tuple(guard) else {
                            panic!("{guard:?}")
                        };
                        (
                            number(constant),
                            matches!(taken, Term::Atom(value) if value == "true"),
                        )
                    })
                    .collect();
                (guards, list(statements).iter().map(number).collect())
            })
            .collect::<Vec<Lowered>>();
        let c = format!("int f(int a) {{ switch (a) {{ {c_body} }} return 0; }}");
        let actual = admitted(&c)
            .paths()
            .iter()
            .map(|path| {
                (
                    case_guards(path),
                    calls(path)
                        .iter()
                        .map(|name| name.strip_prefix("cg_").unwrap().parse().unwrap())
                        .collect(),
                )
            })
            .collect::<Vec<Lowered>>();
        assert_eq!(actual, expected, "{c}");
    }
}
