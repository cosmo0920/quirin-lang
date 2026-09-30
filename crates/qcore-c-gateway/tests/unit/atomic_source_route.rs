//! Unit tests of card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): the atomic
//! form table, the order decoding, and the mechanical binding to
//! `proof/QCore/AtomicSourceRoute.lean` and to the core model's own order
//! validity.

use super::*;
use qcore_core::{C11Execution, MemoryEvent, RelationEdge, WeakMemoryError, check_c11_execution};
use serde_json::json;

const LEAN: &str = include_str!("../../../../proof/QCore/AtomicSourceRoute.lean");

/// The body of the Lean list `start … \n]`.
fn block(start: &str) -> String {
    let body = LEAN.split(start).nth(1).expect(start);
    body[..body.find("\n]").expect("list end")].to_owned()
}

fn quoted(row: &str) -> Vec<&str> {
    row.split('"').skip(1).step_by(2).collect()
}

/// A row's last field without the tuple's own closing `),`.
fn tuple_tail(field: &str) -> &str {
    let field = field.trim_end().trim_end_matches(',');
    field.strip_suffix(')').expect(field)
}

fn class_name(class: FormClass) -> &'static str {
    match class {
        FormClass::Load => ".load",
        FormClass::Store => ".store",
        FormClass::Init => ".init",
        FormClass::ByPointer => ".byPointer",
        FormClass::ReadModifyWrite => ".readModifyWrite",
        FormClass::CompareExchange => ".compareExchange",
        FormClass::Fence => ".fence",
    }
}

/// `formTable` = `ATOMIC_FORM_TABLE`, row for row.
#[test]
fn cg19_lean_form_table_matches_the_rust_table() {
    let rows = block("def formTable : List (String × FormClass) := [")
        .lines()
        .filter(|line| line.trim_start().starts_with('('))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), ATOMIC_FORM_TABLE.len());
    for (row, (name, class)) in rows.iter().zip(ATOMIC_FORM_TABLE) {
        assert_eq!(quoted(row), vec![name], "{row}");
        let lean_class = row
            .rsplit(", ")
            .next()
            .unwrap()
            .trim_end_matches(',')
            .trim_end_matches(')');
        assert_eq!(lean_class, class_name(class), "{row}");
        assert_eq!(form_class(name), Some(class));
        assert!(names_atomic_builtin(name));
    }
}

/// A synthetic `AtomicExpr` of builtin `name` with order operand `order`
/// (`None`: an enumeration constant the resolver does not evaluate).
fn atomic_expr(name: &str, order: Option<i64>) -> Value {
    let pointer = json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
        "type": {"qualType": "int *"},
        "inner": [{"kind": "DeclRefExpr", "type": {"qualType": "int *"},
            "referencedDecl": {"kind": "ParmVarDecl", "name": "p"}}]});
    let order = match order {
        Some(value) => json!({"kind": "IntegerLiteral", "type": {"qualType": "int"},
            "value": value.to_string()}),
        None => json!({"kind": "DeclRefExpr", "type": {"qualType": "int"},
            "referencedDecl": {"kind": "EnumConstantDecl", "name": "memory_order_acquire"}}),
    };
    let value = json!({"kind": "IntegerLiteral", "type": {"qualType": "int"}, "value": "1"});
    let inner = match form_class(name) {
        Some(FormClass::Init) => vec![pointer, value],
        Some(FormClass::Store) => vec![pointer, order, value],
        _ => vec![pointer, order],
    };
    json!({"kind": "AtomicExpr", "name": name, "type": {"qualType": "int"}, "inner": inner})
}

fn lean_refusal(text: &str) -> fn(&AtomicRouteRefusal) -> bool {
    match text {
        ".readModifyWrite" => |r| matches!(r, AtomicRouteRefusal::ReadModifyWrite(_)),
        ".compareExchange" => |r| matches!(r, AtomicRouteRefusal::CompareExchange(_)),
        ".fence" => |r| matches!(r, AtomicRouteRefusal::Fence(_)),
        ".consumeLoad" => |r| matches!(r, AtomicRouteRefusal::ConsumeLoad(_)),
        ".invalidOrder" => |r| matches!(r, AtomicRouteRefusal::InvalidOrder(_)),
        ".unevaluatedOrder" => |r| matches!(r, AtomicRouteRefusal::UnevaluatedOrder(_)),
        ".byPointer" => |r| matches!(r, AtomicRouteRefusal::ByPointer(_)),
        ".unlisted" => |r| matches!(r, AtomicRouteRefusal::Unlisted(_)),
        other => panic!("unmapped Lean refusal {other}"),
    }
}

fn model_order(text: &str) -> Option<MemoryOrder> {
    match text {
        "none" => None,
        "(some .relaxed)" => Some(MemoryOrder::Relaxed),
        "(some .acquire)" => Some(MemoryOrder::Acquire),
        "(some .release)" => Some(MemoryOrder::Release),
        "(some .seqCst)" => Some(MemoryOrder::SequentiallyConsistent),
        other => panic!("unmapped Lean order {other}"),
    }
}

fn operation(text: &str) -> Operation {
    match text {
        ".load" => Operation::Load,
        ".store" => Operation::Store,
        ".init" => Operation::Init,
        other => panic!("unmapped Lean operation {other}"),
    }
}

/// Assert one Rust answer against one Lean `Outcome` spelling.
fn check_outcome(answer: Result<(Operation, Option<MemoryOrder>), AtomicRouteRefusal>, lean: &str) {
    let lean = lean.trim();
    if let Some(refusal) = lean.strip_prefix(".refused ") {
        let refused = answer.expect_err(lean);
        assert!(lean_refusal(refusal)(&refused), "{lean}: {refused:?}");
    } else if let Some(routed) = lean.strip_prefix(".routed ") {
        let (op, order) = routed.split_once(' ').expect(lean);
        assert_eq!(
            answer.expect(lean),
            (operation(op), model_order(order)),
            "{lean}"
        );
    } else {
        panic!("unparsed Lean outcome {lean}");
    }
}

/// `routeVectors` through `classify_builtin` on synthetic nodes.
#[test]
fn cg19_lean_route_vectors_match_classify_builtin() {
    let rows = block("def routeVectors : List (String × Option Int × Outcome) := [")
        .lines()
        .filter(|line| line.trim_start().starts_with('('))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 18);
    for row in &rows {
        let name = quoted(row)[0];
        let rest = row.split_once("\", ").unwrap().1;
        let (order, outcome) = rest.split_once(", ").unwrap();
        let order = match order {
            "none" => None,
            some => Some(some.trim_start_matches("some ").parse::<i64>().unwrap()),
        };
        let outcome = tuple_tail(outcome);
        let node = atomic_expr(name, order);
        let answer =
            classify_builtin(&node, &|_| None).map(|routed| (routed.operation, routed.order));
        check_outcome(answer, outcome);
    }
}

/// `orderVectors` through `decode_order` and `route_order`.
#[test]
fn cg19_lean_order_vectors_match_route_order() {
    let rows = block("def orderVectors : List (Operation × Int × Outcome) := [")
        .lines()
        .filter(|line| line.trim_start().starts_with('('))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 16);
    for row in &rows {
        let fields = row
            .trim()
            .trim_start_matches('(')
            .splitn(3, ", ")
            .collect::<Vec<_>>();
        let op = operation(fields[0]);
        let value = fields[1].parse::<i64>().unwrap();
        let outcome = tuple_tail(fields[2]);
        let answer = match decode_order(value) {
            None => Err(AtomicRouteRefusal::InvalidOrder("row".into())),
            Some(order) => route_order("row", op, order).map(|m| (op, Some(m))),
        };
        check_outcome(answer, outcome);
    }
}

/// `modelRepresents` is the core model's own `event_order_represented`, and
/// every order the route gives is one of them, for every operation and value.
#[test]
fn cg19_routed_orders_are_the_models_own() {
    let body = LEAN
        .split("def modelRepresents : Operation → ModelOrder → Bool")
        .nth(1)
        .unwrap();
    let body = &body[..body.find("\n\n").unwrap()];
    let represented = |op: &str, order: &str| body.contains(&format!("{op}, {order}"));
    let orders = [
        (".relaxed", MemoryOrder::Relaxed),
        (".acquire", MemoryOrder::Acquire),
        (".release", MemoryOrder::Release),
        (".acqRel", MemoryOrder::AcquireRelease),
        (".seqCst", MemoryOrder::SequentiallyConsistent),
    ];
    for (name, order) in orders {
        let load = MemoryEventKind::AtomicLoad {
            location: 0,
            value: 0,
            order,
        };
        let store = MemoryEventKind::AtomicStore {
            location: 0,
            value: 0,
            order,
        };
        assert_eq!(
            event_order_represented(&load),
            represented(".load", name),
            "{name}"
        );
        assert_eq!(
            event_order_represented(&store),
            represented(".store", name),
            "{name}"
        );
    }
    for value in -1..=6 {
        for op in [Operation::Load, Operation::Store, Operation::Init] {
            let Some(order) = decode_order(value) else {
                continue;
            };
            if let Ok(model) = route_order("row", op, order) {
                let routed = RoutedAtomic {
                    form: "row".into(),
                    operation: op,
                    order: Some(model),
                    object: &Value::Null,
                    value: None,
                    value_type: "int".into(),
                };
                assert!(event_order_represented(&routed.event_kind(0, 0)));
            }
        }
    }
}

/// A routed plain store and load, as events of one candidate execution, are
/// accepted by the core checker exactly when the orders synchronise: the
/// routed vocabulary is the model's, not a parallel one.
#[test]
fn cg19_routed_events_run_through_the_core_checker() {
    let store = atomic_expr("__c11_atomic_store", Some(3));
    let load = atomic_expr("__c11_atomic_load", Some(2));
    let store = classify_builtin(&store, &|_| None).unwrap();
    let load = classify_builtin(&load, &|_| None).unwrap();
    let execution =
        |store_kind: MemoryEventKind, load_kind: MemoryEventKind, sw: bool| C11Execution {
            events: vec![
                MemoryEvent {
                    id: 0,
                    thread: 0,
                    kind: MemoryEventKind::AtomicInit {
                        location: 7,
                        value: 0,
                    },
                },
                MemoryEvent {
                    id: 1,
                    thread: 1,
                    kind: store_kind,
                },
                MemoryEvent {
                    id: 2,
                    thread: 2,
                    kind: load_kind,
                },
            ],
            reads_from: vec![RelationEdge::new(1, 2)],
            modification_order: vec![RelationEdge::new(0, 1)],
            synchronizes_with: if sw {
                vec![RelationEdge::new(1, 2)]
            } else {
                vec![]
            },
            ..C11Execution::default()
        };
    let checked = check_c11_execution(&execution(
        store.event_kind(7, 1),
        load.event_kind(7, 1),
        true,
    ));
    assert!(checked.is_ok(), "{checked:?}");
    // A relaxed load of the same store does not synchronise: the model
    // refuses the supplied edge.
    let relaxed = atomic_expr("__c11_atomic_load", Some(0));
    let relaxed = classify_builtin(&relaxed, &|_| None).unwrap();
    let refused = check_c11_execution(&execution(
        store.event_kind(7, 1),
        relaxed.event_kind(7, 1),
        true,
    ))
    .unwrap_err();
    assert!(refused.contains(&WeakMemoryError::InvalidSynchronizesWith(
        RelationEdge::new(1, 2)
    )));
}

/// Every listed refusing class, and an unlisted family member, refuse by name.
#[test]
fn cg19_refusals_name_the_form_and_card() {
    for (name, text) in [
        (
            "__c11_atomic_fetch_add",
            "atomic read-modify-write `__c11_atomic_fetch_add` is not represented by the weak-memory model (CG28)",
        ),
        (
            "__c11_atomic_compare_exchange_strong",
            "atomic compare-exchange `__c11_atomic_compare_exchange_strong` is not represented by the weak-memory model (CG28)",
        ),
        (
            "__c11_atomic_thread_fence",
            "atomic fence `__c11_atomic_thread_fence` is not represented by the weak-memory model (CG28)",
        ),
        (
            "__atomic_fetch_umax",
            "unlisted atomic form `__atomic_fetch_umax` (CG19)",
        ),
    ] {
        let refused = classify_builtin(&atomic_expr(name, Some(5)), &|_| None).unwrap_err();
        assert_eq!(refused.reason(), text);
    }
    let consume =
        classify_builtin(&atomic_expr("__c11_atomic_load", Some(1)), &|_| None).unwrap_err();
    assert_eq!(
        consume.reason(),
        "memory_order_consume load `__c11_atomic_load` is not represented by the weak-memory model (CG28)"
    );
    // An enumeration-constant order the resolver evaluates is routed.
    let resolved = atomic_expr("__c11_atomic_load", None);
    let resolved = classify_builtin(&resolved, &|_| Some(2)).unwrap();
    assert_eq!(resolved.order, Some(MemoryOrder::Acquire));
    assert!(!names_atomic_builtin("atomic_load_helper"));
}

/// The four qualified rows, in `QUALIFIED_FORMS` order, and the Lean
/// assumption the publication verdict carries.
#[test]
fn cg19_qualified_forms_and_single_graph_assumption() {
    let load = json!({"kind": "ImplicitCastExpr", "castKind": "AtomicToNonAtomic",
        "type": {"qualType": "int"},
        "inner": [{"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
            "type": {"qualType": "_Atomic(int)"},
            "inner": [{"kind": "DeclRefExpr", "type": {"qualType": "_Atomic(int)"},
                "referencedDecl": {"kind": "VarDecl", "name": "g"}}]}]});
    let routed = classify_qualified(&load).unwrap().unwrap();
    assert_eq!(routed.form, QUALIFIED_FORMS[0].0);
    assert_eq!(routed.order, Some(MemoryOrder::SequentiallyConsistent));
    let lhs = json!({"kind": "DeclRefExpr", "type": {"qualType": "_Atomic(int)"},
        "referencedDecl": {"kind": "VarDecl", "name": "g"}});
    let store = json!({"kind": "BinaryOperator", "opcode": "=", "type": {"qualType": "int"},
        "inner": [lhs.clone(), {"kind": "ImplicitCastExpr", "castKind": "NonAtomicToAtomic",
            "type": {"qualType": "_Atomic(int)"},
            "inner": [{"kind": "IntegerLiteral", "type": {"qualType": "int"}, "value": "1"}]}]});
    assert_eq!(
        classify_qualified(&store).unwrap().unwrap().operation,
        Operation::Store
    );
    let compound = json!({"kind": "CompoundAssignOperator", "opcode": "+=",
        "type": {"qualType": "int"},
        "inner": [lhs.clone(), {"kind": "IntegerLiteral", "type": {"qualType": "int"}, "value": "1"}]});
    assert!(matches!(
        classify_qualified(&compound),
        Some(Err(AtomicRouteRefusal::ReadModifyWrite(_)))
    ));
    let increment = json!({"kind": "UnaryOperator", "opcode": "++", "type": {"qualType": "int"},
        "inner": [lhs]});
    assert!(matches!(
        classify_qualified(&increment),
        Some(Err(AtomicRouteRefusal::ReadModifyWrite(_)))
    ));
    // A `volatile` object that is not atomic is none of these forms.
    let volatile = json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
        "type": {"qualType": "volatile int"},
        "inner": [{"kind": "DeclRefExpr", "type": {"qualType": "volatile int"},
            "referencedDecl": {"kind": "VarDecl", "name": "v"}}]});
    assert!(classify_qualified(&volatile).is_none());
    assert!(LEAN.contains("assumptions := [.singleGraph,"));
    assert!(SINGLE_GRAPH_ASSUMPTION.contains("one supplied candidate execution graph"));
}
