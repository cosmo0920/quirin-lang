//! Unit tests of the CG02 elaborator (gateway rule 10s): the evaluation
//! graph, the order model, the value forms, the named refusals, and the
//! mechanical binding to `proof/QCore/ElaborationCore.lean`.

use std::collections::BTreeSet;

use serde_json::Value;

use super::*;
use crate::legacy_c::C11TranslationUnit;
use crate::test_support::clang_available;

/// A route-free context: every scalar and pointer is read, every path is
/// admitted with no facts, and aliasing is read from the function's AST.
struct Plain {
    automatic: BTreeSet<String>,
    taken: BTreeSet<String>,
}

impl Plain {
    fn new(function: &Value) -> Self {
        fn walk(node: &Value, automatic: &mut BTreeSet<String>, taken: &mut BTreeSet<String>) {
            match kind(node) {
                "ParmVarDecl" | "VarDecl" if node["storageClass"].as_str().is_none() => {
                    automatic.insert(node["id"].as_str().unwrap().to_owned());
                }
                "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                    let operand = strip_parens(&children(node)[0]);
                    if let Some(id) = operand["referencedDecl"]["id"].as_str() {
                        taken.insert(id.to_owned());
                    }
                }
                _ => {}
            }
            for child in children(node) {
                walk(child, automatic, taken);
            }
        }
        let mut plain = Plain {
            automatic: BTreeSet::new(),
            taken: BTreeSet::new(),
        };
        walk(function, &mut plain.automatic, &mut plain.taken);
        plain
    }
}

impl Context for Plain {
    type Facts = ();
    fn scalar(&self, node: &Value) -> bool {
        crate::typed_access_path::is_scalar(node)
    }
    fn pointer(&self, node: &Value) -> bool {
        crate::typed_access_path::is_pointer(node)
    }
    fn place(&self, _node: &Value, _store: bool) -> Result<(), Refusal> {
        Ok(())
    }
    fn unaliased(&self, decl: &str) -> bool {
        self.automatic.contains(decl) && !self.taken.contains(decl)
    }
}

fn function(source: &str) -> Value {
    crate::typed_c_ast::function_ast(
        source,
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        "subject",
    )
    .unwrap_or_else(|error| panic!("Clang rejected the fixture: {error}\n{source}"))
}

/// The subject of a fixture: its first body statement that holds a value form
/// or a call (a `return`'s value for a return).
fn subject(function: &Value) -> Value {
    fn interesting(node: &Value) -> bool {
        value_form(node).is_some()
            || kind(node) == "CallExpr"
            || matches!(
                kind(node),
                "ConditionalOperator" | "StmtExpr" | "GenericSelectionExpr"
            )
            || (kind(node) == "BinaryOperator"
                && matches!(node["opcode"].as_str(), Some("&&" | "||")))
            || children(node).iter().any(interesting)
    }
    let body = children(function)
        .iter()
        .find(|node| kind(node) == "CompoundStmt")
        .expect("a body");
    for statement in children(body) {
        if interesting(statement) {
            return if kind(statement) == "ReturnStmt" {
                children(statement)[0].clone()
            } else {
                statement.clone()
            };
        }
    }
    panic!("no subject statement");
}

fn run(source: &str) -> Result<Elaboration<()>, Refusal> {
    let function = function(source);
    let node = subject(&function);
    elaborate(&node, &Plain::new(&function))
}

fn object(name: &str) -> impl Fn(&Event) -> bool + '_ {
    move |event| match event {
        Event::Read(Place::Object { name: n, .. })
        | Event::Write {
            place: Place::Object { name: n, .. },
            ..
        } => n == name,
        _ => false,
    }
}

#[test]
fn cg02_unsequenced_same_place_pairs_are_refused() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(int i) { i = i++; return i; }",
        "int subject(int i, int *a) { a[i] = i++; return i; }",
        "int f(int, int); int subject(int x) { return f(x++, x); }",
        "int subject(int x) { x += x++; return x; }",
        "int subject(int i) { int j = i++ + i; return j; }",
        "int subject(int i) { int j = i + (i = 2); return j; }",
    ] {
        match run(source) {
            Err(Refusal::UnsequencedConflict(place)) => assert_eq!(place, place.trim(), "{source}"),
            other => panic!("{source}: {other:?}"),
        }
    }
}

/// Two paths are never proved disjoint here: an unordered pair on distinct
/// fields refuses without claiming undefined behaviour.
#[test]
fn cg02_path_pairs_are_not_disjoint_and_not_called_undefined() {
    if !clang_available() {
        return;
    }
    let source =
        "struct P { int lo; int hi; }; int subject(struct P *p) { p->lo = p->hi = 0; return 0; }";
    match run(source) {
        Err(Refusal::NotDisjoint(one, other)) => {
            assert_ne!(one, other);
            assert!(
                !Refusal::NotDisjoint(one, other)
                    .reason()
                    .contains("undefined")
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn cg02_unordered_calls_are_refused_for_cg05() {
    if !clang_available() {
        return;
    }
    for source in [
        "int f(int); int g(int); int subject(int a) { return f(a) + g(a); }",
        "int f(int); int subject(int *a) { return f(1) + *a; }",
        "int f(int); int subject(int a, int k) { int x = f(a) + k++; return x; }",
    ] {
        assert!(
            matches!(run(source), Err(Refusal::UnorderedCall(_))),
            "{source}: {:?}",
            run(source)
        );
    }
    assert_eq!(
        run("int f(int); int g(int); int subject(int a) { return f(g(a)); }").unwrap_err(),
        refused("nested call")
    );
}

#[test]
fn cg02_disjoint_and_read_only_groups_are_admitted() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(int i) { int j = 0; j = i++; return j; }",
        "int subject(int a) { int x; int y; x = (y = a); return x; }",
        "int subject(int i) { int j = 0; j = (i++, i); return j; }",
        "int subject(int i, int k) { int j = i++ + k++; return j; }",
        "int subject(int i, int k) { int j = 0; j = i + i; return j + k; }",
        "struct S { int a; }; int subject(struct S *s, int i) { s->a = i++; return i; }",
        "int f(int); int subject(int x) { return f(x++); }",
    ] {
        assert!(run(source).is_ok(), "{source}: {:?}", run(source));
    }
    // A path through a pointer may reach an object whose address is taken.
    let taken = "struct S { int a; }; int subject(struct S *s, int i) { int *q = &i; \
                 s->a = i++; return *q; }";
    assert!(
        matches!(run(taken), Err(Refusal::NotDisjoint(_, _))),
        "{:?}",
        run(taken)
    );
}

#[test]
fn cg02_pre_and_post_values_are_kept_apart() {
    if !clang_available() {
        return;
    }
    for (source, old) in [
        ("int subject(int i) { int j = 0; j = i++; return j; }", true),
        ("int subject(int i) { int j = 0; j = i--; return j; }", true),
        (
            "int subject(int i) { int j = 0; j = ++i; return j; }",
            false,
        ),
        (
            "int subject(int i) { int j = 0; j = --i; return j; }",
            false,
        ),
    ] {
        let function = function(source);
        let node = subject(&function);
        let [_, value] = children(&node) else {
            panic!("an assignment")
        };
        let elaborated = elaborate(value, &Plain::new(&function)).unwrap();
        let Operand::Value { source: value, .. } = elaborated.result else {
            panic!("a value")
        };
        match (old, value) {
            (true, ValueSource::Old(place)) | (false, ValueSource::New(place)) => {
                assert_eq!(place.spelling(), "i");
            }
            other => panic!("{source}: {other:?}"),
        }
        // One read then one write of `i`, the write after the read.
        let events = &elaborated.graph.events;
        let read = events
            .iter()
            .position(|e| matches!(e, Event::Read(_)))
            .unwrap();
        let write = events
            .iter()
            .position(|e| matches!(e, Event::Write { .. }))
            .unwrap();
        assert!(elaborated.graph.sequenced(read, write));
        assert_eq!(events.len(), 2);
    }
}

#[test]
fn cg02_compound_assignment_evaluates_its_place_once() {
    if !clang_available() {
        return;
    }
    let source = "struct S { int a; int arr[4]; }; \
                  int subject(struct S *s, int k) { s->arr[k] += k; return 0; }";
    let elaborated = run(source).unwrap();
    let events = &elaborated.graph.events;
    let path = |event: &Event| {
        matches!(
            event,
            Event::Read(Place::Path { .. })
                | Event::Write {
                    place: Place::Path { .. },
                    ..
                }
        )
    };
    // The address reads `s` and `k` once, the place is read once and written once,
    // and the right operand reads `k`.
    assert_eq!(events.iter().filter(|e| path(e)).count(), 2, "{events:?}");
    assert_eq!(events.iter().filter(|e| object("s")(e)).count(), 1);
    assert_eq!(events.iter().filter(|e| object("k")(e)).count(), 2);
    let write = events
        .iter()
        .position(|e| matches!(e, Event::Write { .. }))
        .unwrap();
    assert_eq!(write, events.len() - 1);
    for before in 0..write {
        assert!(
            elaborated.graph.sequenced(before, write)
                || !matches!(events[before], Event::Read(Place::Path { .. })),
            "{events:?}"
        );
    }
    // The place read and the right operand's read are unsequenced, and commute.
    assert!(!unordered_pairs(&elaborated.graph).is_empty());
    assert_eq!(elaborated.assumptions, vec![Assumption::NoNumericClaim]);
    assert!(
        elaborated
            .obligations
            .iter()
            .any(|o| matches!(o, Obligation::Access(Place::Path { .. })))
    );
}

#[test]
fn cg02_comma_orders_every_left_event_first() {
    if !clang_available() {
        return;
    }
    let source = "int g(int); int subject(int i, int k) { int j = (g(i++), k++, i); return j; }";
    let elaborated = run(source).unwrap();
    let graph = &elaborated.graph;
    let call = calls(graph)[0].0;
    let k_write = graph
        .events
        .iter()
        .position(
            |e| matches!(e, Event::Write { place: Place::Object { name, .. }, .. } if name == "k"),
        )
        .unwrap();
    // Every event of `g(i++)`, the increment included, precedes `k++`, which
    // precedes the final read of `i` and the declaration's write.
    for earlier in 0..=call {
        assert!(graph.sequenced(earlier, k_write), "{graph:?}");
    }
    let j_write = graph.events.len() - 1;
    assert!(graph.sequenced(call, j_write));
    // The value is the last operand's.
    assert!(matches!(elaborated.result, Operand::Void));
    // A literal right operand still orders the left one before a consumer.
    let literal = run("int g(int); int subject(int i) { int j = (g(i), 1); return j; }").unwrap();
    let write = literal.graph.events.len() - 1;
    assert!(literal.graph.sequenced(0, write));
}

#[test]
fn cg02_unlisted_and_out_of_scope_forms_are_refused_by_name() {
    if !clang_available() {
        return;
    }
    for (source, form) in [
        (
            "int subject(int a, int b) { int x = 0; x = (a && b); return x; }",
            "value &&",
        ),
        (
            "int subject(int a, int b) { int x = 0; x = (a || b); return x; }",
            "value ||",
        ),
        (
            "int subject(int a, int b) { int x = 0; x = a ? b : 1; return x; }",
            "conditional",
        ),
        (
            "int subject(int *p) { p++; int x = 0; x = *p++; return x; }",
            "pointer arithmetic",
        ),
        (
            "int subject(void (*f)(int), int a) { int x = 0; x = (f(a), a); return x; }",
            "indirect call",
        ),
        (
            "int subject(volatile int v) { int x = 0; x = v++; return x; }",
            "volatile access",
        ),
        (
            "int subject(int a) { int x = 0; x = _Generic(a, int: a++, default: 0); return x; }",
            "generic selection",
        ),
        (
            "int subject(int a) { int x = 0; x = ({ a++; a; }); return x; }",
            "statement expression",
        ),
        (
            "struct P { int a; }; int subject(int a) { int x = 0; x = (struct P){ a++ }.a; return x; }",
            "compound literal",
        ),
    ] {
        assert_eq!(run(source).unwrap_err(), refused(form), "{source}");
        assert!(matches!(disposition(form), Some(Disposition::Refused(_))));
    }
    // A pointer-valued store is elaborated with its class; route G refuses it.
    let pointer = run("int subject(int *p, int *q) { int *r; r = (p = q); return 0; }").unwrap();
    assert!(
        pointer
            .graph
            .events
            .iter()
            .any(|e| matches!(e, Event::Write { pointer: true, .. }))
    );
}

#[test]
fn cg02_route_t_names_each_value_form() {
    use crate::typed_source::{SourceImportProfile, SourceResult, import_source_functions};
    if !clang_available() {
        return;
    }
    for (source, form) in [
        (
            "int subject(int a) { int x; int y; x = (y = a); return x; }",
            "assignment value",
        ),
        (
            "int subject(int a) { int j = a++; return j; }",
            "post increment",
        ),
        (
            "int subject(int a) { int j = --a; return j; }",
            "pre decrement",
        ),
        ("int subject(int a, int b) { return (a, b); }", "comma"),
        (
            "int subject(int a) { int x = 1; return x += a; }",
            "compound assignment value",
        ),
    ] {
        let results = import_source_functions(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            "cg02",
            &["subject"],
            &SourceImportProfile {
                records: &[],
                sizeof_types: &["int"],
                signature_constraints: &[],
                restricted_calls: &[],
            },
        )
        .unwrap();
        match results.into_iter().next() {
            Some(SourceResult::Unknown { reason, .. }) => {
                assert_eq!(reason, t_refusal(form), "{source}")
            }
            other => panic!("{source}: {other:?}"),
        }
    }
}

/// The mechanical binding of `proof/QCore/ElaborationCore.lean`.
#[test]
fn cg02_lean_vectors_match_the_rust_tables() {
    let lean = include_str!("../../../../proof/QCore/ElaborationCore.lean");
    let block = |start: &str| {
        let body = lean.split(start).nth(1).expect(start);
        body[..body.find("\n]").unwrap()].to_owned()
    };
    // formTable = FORM_TABLE.
    let rows = block("def formTable : List (String × Disposition) := [")
        .lines()
        .filter(|line| line.trim_start().starts_with('('))
        .map(|line| {
            let (form, rest) = line.split('"').nth(1).map(|form| (form, line)).unwrap();
            let disposition = if rest.contains(".admitted") {
                Disposition::Admitted
            } else if rest.contains(".sequenced") {
                Disposition::Sequenced
            } else {
                let card = rest.split('"').nth(3).unwrap();
                Disposition::Refused(
                    FORM_TABLE
                        .iter()
                        .find_map(|(_, d)| match d {
                            Disposition::Refused(c) if *c == card => Some(*c),
                            _ => None,
                        })
                        .unwrap_or_else(|| panic!("unknown card {card}")),
                )
            };
            (form.to_owned(), disposition)
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), FORM_TABLE.len());
    for ((form, disposition), (rust_form, rust_disposition)) in rows.iter().zip(FORM_TABLE) {
        assert_eq!((form.as_str(), *disposition), (rust_form, rust_disposition));
    }
    // commuteVectors against `commutes` on objects numbered by declaration.
    let event = |text: &str| -> Event {
        let text = text.trim().trim_start_matches('(').trim();
        if text == ".call" {
            return Event::Call { callee: "f".into() };
        }
        let (kind, number) = text.trim_start_matches('.').split_once(' ').unwrap();
        let place = Place::Object {
            decl: number.to_owned(),
            name: format!("v{number}"),
        };
        match kind {
            "read" => Event::Read(place),
            "write" => Event::Write {
                place,
                pointer: false,
            },
            other => panic!("unknown Lean event {other}"),
        }
    };
    let mut count = 0;
    for line in block("def commuteVectors : List (Ev × Ev × Bool) := [").lines() {
        let line = line.trim().trim_end_matches(',');
        if !line.starts_with('(') {
            continue;
        }
        let fields = line.trim_end_matches(')').split(", ").collect::<Vec<_>>();
        let expected = fields[2] == "true";
        let decided =
            commutes(&event(fields[0]), &event(fields[1]), &|_| true) == Commutation::Commutes;
        assert_eq!(decided, expected, "{line}");
        count += 1;
    }
    assert_eq!(count, 10);
    // orderVectors: each C source through Clang and `elaborate`.
    if !clang_available() {
        return;
    }
    let vectors = block("def orderVectors : List (String × Expr × Bool) := [");
    let mut count = 0;
    for row in vectors.split("\n  (\"").skip(1) {
        let (source, rest) = row.split_once('"').unwrap();
        let expected = rest
            .trim_end()
            .trim_end_matches(',')
            .trim_end_matches(')')
            .ends_with("true");
        assert_eq!(run(source).is_ok(), expected, "{source}: {:?}", run(source));
        count += 1;
    }
    assert_eq!(count, 10);
}
