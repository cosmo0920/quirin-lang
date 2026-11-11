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

//! Unit tests of card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i), extended
//! by card CG28-WEAK-MEMORY-EVENTS (rule 11t): the atomic form table, the
//! order decoding, and the mechanical binding to
//! `proof/QCore/AtomicSourceRoute.lean` and to the core model's own order
//! validity.

use super::*;
use qcore_core::{C11Execution, MemoryEvent, RelationEdge, WeakMemoryError, check_c11_execution};
use serde_json::json;

const LEAN: &str = include_str!("../../../../proof/QCore/AtomicSourceRoute.lean");

/// The body of the Lean list `start … \n]`.
fn block(start: &str) -> String {
    let lean = LEAN.replace("\r\n", "\n");
    let body = lean.split(start).nth(1).expect(start);
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
        FormClass::SignalFence => ".signalFence",
        FormClass::LegacySync => ".legacySync",
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

/// A synthetic order (or weak flag) operand: an integer literal, or (`None`)
/// an enumeration constant the resolver does not evaluate.
fn order_operand(order: Option<i64>) -> Value {
    match order {
        Some(value) => json!({"kind": "IntegerLiteral", "type": {"qualType": "int"},
            "value": value.to_string()}),
        None => json!({"kind": "DeclRefExpr", "type": {"qualType": "int"},
            "referencedDecl": {"kind": "EnumConstantDecl", "name": "memory_order_acquire"}}),
    }
}

fn pointer_operand(name: &str) -> Value {
    json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
        "type": {"qualType": "int *"},
        "inner": [{"kind": "DeclRefExpr", "type": {"qualType": "int *"},
            "referencedDecl": {"kind": "ParmVarDecl", "name": name}}]})
}

/// A synthetic node of builtin `name` with order operand `order`: an
/// `AtomicExpr` in Clang's child order, or (card CG28) a fence call.
fn atomic_expr(name: &str, order: Option<i64>) -> Value {
    let value = json!({"kind": "IntegerLiteral", "type": {"qualType": "int"}, "value": "1"});
    let order_node = order_operand(order);
    let inner = match form_class(name) {
        Some(FormClass::Fence | FormClass::SignalFence) => {
            return json!({"kind": "CallExpr", "type": {"qualType": "void"},
                "inner": [{"kind": "ImplicitCastExpr", "castKind": "BuiltinFnToFnPtr",
                    "inner": [{"kind": "DeclRefExpr",
                        "referencedDecl": {"kind": "FunctionDecl", "name": name}}]},
                    order_node]});
        }
        Some(FormClass::Init) => vec![pointer_operand("p"), value],
        Some(FormClass::Store | FormClass::ReadModifyWrite) => {
            vec![pointer_operand("p"), order_node, value]
        }
        Some(FormClass::CompareExchange) => {
            return cas_expr(name, order, Some(5), Some(0));
        }
        _ => vec![pointer_operand("p"), order_node],
    };
    json!({"kind": "AtomicExpr", "name": name, "type": {"qualType": "int"}, "inner": inner})
}

/// A synthetic compare-exchange `AtomicExpr`: pointer, success order,
/// expected pointer, failure order, desired value (by pointer for the
/// generic form) and, for the GNU forms, the weak flag.
fn cas_expr(name: &str, success: Option<i64>, failure: Option<i64>, weak: Option<i64>) -> Value {
    let desired = if name == "__atomic_compare_exchange" {
        json!({"kind": "UnaryOperator", "opcode": "&", "type": {"qualType": "int *"},
            "inner": [{"kind": "DeclRefExpr", "type": {"qualType": "int"},
                "referencedDecl": {"kind": "ParmVarDecl", "name": "d"}}]})
    } else {
        json!({"kind": "IntegerLiteral", "type": {"qualType": "int"}, "value": "1"})
    };
    let mut inner = vec![
        pointer_operand("p"),
        order_operand(success),
        pointer_operand("e"),
        order_operand(failure),
        desired,
    ];
    if !name.starts_with("__c11_") {
        inner.push(
            json!({"kind": "ImplicitCastExpr", "castKind": "IntegralToBoolean",
            "type": {"qualType": "_Bool"}, "inner": [order_operand(weak)]}),
        );
    }
    json!({"kind": "AtomicExpr", "name": name, "type": {"qualType": "_Bool"}, "inner": inner})
}

fn lean_refusal(text: &str) -> fn(&AtomicRouteRefusal) -> bool {
    match text {
        ".consumeLoad" => |r| matches!(r, AtomicRouteRefusal::ConsumeLoad(_)),
        ".consumeOrder" => |r| matches!(r, AtomicRouteRefusal::ConsumeOrder(_)),
        ".legacySync" => |r| matches!(r, AtomicRouteRefusal::LegacySync(_)),
        ".unevaluatedStrength" => |r| matches!(r, AtomicRouteRefusal::UnevaluatedStrength(_)),
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
        "(some .acqRel)" => Some(MemoryOrder::AcquireRelease),
        "(some .seqCst)" => Some(MemoryOrder::SequentiallyConsistent),
        other => panic!("unmapped Lean order {other}"),
    }
}

fn operation(text: &str) -> Operation {
    match text {
        ".load" => Operation::Load,
        ".store" => Operation::Store,
        ".init" => Operation::Init,
        ".readModifyWrite" => Operation::ReadModifyWrite,
        ".compareExchange" => Operation::CompareExchange,
        ".threadFence" => Operation::ThreadFence,
        ".signalFence" => Operation::SignalFence,
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
    assert_eq!(rows.len(), 26);
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
    assert_eq!(rows.len(), 32);
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
    let lean = LEAN.replace("\r\n", "\n");
    let body = lean
        .split("def modelRepresents : Operation → ModelOrder → Bool")
        .nth(1)
        .unwrap();
    let body = &body[..body.find("\n\n").unwrap()];
    let represented = |op: &str, order: &str| {
        body.contains(&format!("{op}, {order}")) || body.contains(&format!("{op}, _"))
    };
    let orders = [
        (".relaxed", MemoryOrder::Relaxed),
        (".acquire", MemoryOrder::Acquire),
        (".release", MemoryOrder::Release),
        (".acqRel", MemoryOrder::AcquireRelease),
        (".seqCst", MemoryOrder::SequentiallyConsistent),
    ];
    for (name, order) in orders {
        let kinds = [
            (
                ".load",
                MemoryEventKind::AtomicLoad {
                    location: 0,
                    value: 0,
                    order,
                },
            ),
            (
                ".store",
                MemoryEventKind::AtomicStore {
                    location: 0,
                    value: 0,
                    order,
                },
            ),
            (
                ".readModifyWrite",
                MemoryEventKind::AtomicReadModifyWrite {
                    location: 0,
                    read_value: 0,
                    written_value: 0,
                    order,
                },
            ),
            (
                ".threadFence",
                MemoryEventKind::AtomicFence {
                    order,
                    scope: qcore_core::FenceScope::Thread,
                },
            ),
            (
                ".signalFence",
                MemoryEventKind::AtomicFence {
                    order,
                    scope: qcore_core::FenceScope::Signal,
                },
            ),
            // A compare-exchange's success order, with the relaxed failure
            // order every success order admits.
            (
                ".compareExchange",
                MemoryEventKind::AtomicCompareExchange {
                    location: 0,
                    expected: 0,
                    observed: 0,
                    desired: 0,
                    success: order,
                    failure: MemoryOrder::Relaxed,
                    strength: qcore_core::CompareExchangeStrength::Strong,
                    succeeded: true,
                },
            ),
        ];
        for (op, kind) in kinds {
            assert_eq!(
                event_order_represented(&kind),
                represented(op, name),
                "{op} {name}"
            );
        }
    }
    for value in -1..=6 {
        for op in [
            Operation::Load,
            Operation::Store,
            Operation::Init,
            Operation::ReadModifyWrite,
            Operation::CompareExchange,
            Operation::ThreadFence,
            Operation::SignalFence,
        ] {
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
                    failure_order: Some(MemoryOrder::Relaxed),
                    strength: Some(qcore_core::CompareExchangeStrength::Strong),
                    expected: None,
                    value_by_pointer: false,
                };
                let event = routed.event(0, routed.shape_values()).unwrap();
                assert!(event_order_represented(&event), "{op:?} {value}");
            }
        }
    }
}

/// `casVectors` through `classify_builtin` on synthetic compare-exchange
/// nodes: success and failure orders and the strength (card CG28).
#[test]
fn cg28_lean_cas_vectors_match_classify_builtin() {
    let rows = block(
        "def casVectors : List (String × Option Int × Option Int × Option Int × Outcome) := [",
    )
    .lines()
    .filter(|line| line.trim_start().starts_with('('))
    .map(str::to_owned)
    .collect::<Vec<_>>();
    assert_eq!(rows.len(), 12);
    let int = |text: &str| match text.trim() {
        "none" => None,
        some => Some(some.trim_start_matches("some ").parse::<i64>().unwrap()),
    };
    let strength = |text: &str| match text {
        ".strong" => qcore_core::CompareExchangeStrength::Strong,
        ".weak" => qcore_core::CompareExchangeStrength::Weak,
        other => panic!("unmapped Lean strength {other}"),
    };
    let order = |text: &str| model_order(&format!("(some {text})")).unwrap();
    for row in &rows {
        let name = quoted(row)[0];
        let fields = row
            .split_once("\", ")
            .unwrap()
            .1
            .splitn(4, ", ")
            .collect::<Vec<_>>();
        let outcome = tuple_tail(fields[3]).trim();
        let node = cas_expr(name, int(fields[0]), int(fields[1]), int(fields[2]));
        let answer = classify_builtin(&node, &|_| None);
        if let Some(refusal) = outcome.strip_prefix(".refused ") {
            let refused = answer.expect_err(row);
            assert!(lean_refusal(refusal)(&refused), "{row}: {refused:?}");
        } else {
            let parts = outcome
                .strip_prefix(".routedCas ")
                .expect(row)
                .split(' ')
                .collect::<Vec<_>>();
            let routed = answer.expect(row);
            assert_eq!(routed.operation, Operation::CompareExchange, "{row}");
            assert_eq!(routed.order, Some(order(parts[0])), "{row}");
            assert_eq!(routed.failure_order, Some(order(parts[1])), "{row}");
            assert_eq!(routed.strength, Some(strength(parts[2])), "{row}");
            assert_eq!(
                routed.value_by_pointer,
                name == "__atomic_compare_exchange",
                "{row}"
            );
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
        store.event_kind(7, 1).unwrap(),
        load.event_kind(7, 1).unwrap(),
        true,
    ));
    assert!(checked.is_ok(), "{checked:?}");
    // A relaxed load of the same store does not synchronise: the model
    // refuses the supplied edge.
    let relaxed = atomic_expr("__c11_atomic_load", Some(0));
    let relaxed = classify_builtin(&relaxed, &|_| None).unwrap();
    let refused = check_c11_execution(&execution(
        store.event_kind(7, 1).unwrap(),
        relaxed.event_kind(7, 1).unwrap(),
        true,
    ))
    .unwrap_err();
    assert!(refused.contains(&WeakMemoryError::InvalidSynchronizesWith(
        RelationEdge::new(1, 2)
    )));
}

/// Card CG28: a routed release fence, relaxed store, relaxed read and
/// acquire fence are the message-passing-with-fences graph the core accepts;
/// a routed RMW is the model's own read-modify-write; and `event_kind` (the
/// plain event) answers nothing for an operation that is not plain.
#[test]
fn cg28_routed_rmw_cas_and_fence_events_run_through_the_core_checker() {
    let nodes = [
        atomic_expr("__c11_atomic_thread_fence", Some(3)),
        atomic_expr("__atomic_thread_fence", Some(2)),
        atomic_expr("__c11_atomic_store", Some(0)),
        atomic_expr("__c11_atomic_load", Some(0)),
        atomic_expr("__c11_atomic_fetch_add", Some(4)),
        cas_expr("__c11_atomic_compare_exchange_weak", Some(5), Some(2), None),
    ];
    let routed = nodes
        .iter()
        .map(|node| classify_builtin(node, &|_| None).unwrap())
        .collect::<Vec<_>>();
    let [release, acquire, store, load, rmw, cas] = routed.as_slice() else {
        unreachable!()
    };
    assert_eq!(release.event_kind(7, 0), None);
    let event = |id, thread, kind| MemoryEvent { id, thread, kind };
    let execution = C11Execution {
        events: vec![
            event(
                0,
                0,
                MemoryEventKind::AtomicInit {
                    location: 7,
                    value: 0,
                },
            ),
            event(1, 1, MemoryEventKind::NonAtomicWrite { location: 9 }),
            event(2, 1, release.event(7, EventValues::Fence).unwrap()),
            event(3, 1, store.event_kind(7, 1).unwrap()),
            event(4, 2, load.event_kind(7, 1).unwrap()),
            event(5, 2, acquire.event(7, EventValues::Fence).unwrap()),
            event(6, 2, MemoryEventKind::NonAtomicRead { location: 9 }),
        ],
        reads_from: vec![RelationEdge::new(3, 4)],
        modification_order: vec![RelationEdge::new(0, 3)],
        synchronizes_with: vec![RelationEdge::new(2, 5)],
        ..C11Execution::default()
    };
    assert!(check_c11_execution(&execution).is_ok());
    assert_eq!(
        rmw.event(
            7,
            EventValues::ReadModifyWrite {
                read: 0,
                written: 1
            }
        ),
        Some(MemoryEventKind::AtomicReadModifyWrite {
            location: 7,
            read_value: 0,
            written_value: 1,
            order: MemoryOrder::AcquireRelease,
        })
    );
    assert_eq!(rmw.event(7, EventValues::Plain(0)), None);
    let failed = cas
        .event(
            7,
            EventValues::CompareExchange {
                expected: 0,
                observed: 0,
                desired: 1,
                succeeded: false,
            },
        )
        .unwrap();
    assert!(event_order_represented(&failed));
    assert!(matches!(
        failed,
        MemoryEventKind::AtomicCompareExchange {
            failure: MemoryOrder::Acquire,
            strength: qcore_core::CompareExchangeStrength::Weak,
            ..
        }
    ));
}

/// The refusing classes, and an unlisted family member, refuse by name.
#[test]
fn cg19_refusals_name_the_form_and_card() {
    for (name, order, text) in [
        (
            "__c11_atomic_fetch_add",
            1,
            "memory_order_consume on `__c11_atomic_fetch_add` is outside the weak-memory model: dependency ordering is not modelled and acquire would be a strengthening (CG28)",
        ),
        (
            "__sync_synchronize",
            5,
            "legacy `__sync_synchronize` is a GCC full barrier with no C11 memory order and is not routed to the weak-memory model (CG28)",
        ),
        (
            "__atomic_exchange",
            5,
            "generic by-pointer atomic `__atomic_exchange` is not routed to the weak-memory model (CG19)",
        ),
        (
            "__atomic_fetch_umax",
            5,
            "unlisted atomic form `__atomic_fetch_umax` (CG19)",
        ),
    ] {
        let refused = classify_builtin(&atomic_expr(name, Some(order)), &|_| None).unwrap_err();
        assert_eq!(refused.reason(), text);
    }
    let consume =
        classify_builtin(&atomic_expr("__c11_atomic_load", Some(1)), &|_| None).unwrap_err();
    assert_eq!(
        consume.reason(),
        "memory_order_consume load `__c11_atomic_load` is not represented by the weak-memory model (CG28)"
    );
    let weak = classify_builtin(
        &cas_expr("__atomic_compare_exchange_n", Some(5), Some(5), None),
        &|_| None,
    )
    .unwrap_err();
    assert_eq!(
        weak.reason(),
        "weak flag of compare-exchange `__atomic_compare_exchange_n` is not an integer constant the atomic route evaluates (CG28)"
    );
    // An enumeration-constant order the resolver evaluates is routed.
    let resolved = atomic_expr("__c11_atomic_load", None);
    let resolved = classify_builtin(&resolved, &|_| Some(2)).unwrap();
    assert_eq!(resolved.order, Some(MemoryOrder::Acquire));
    assert!(!names_atomic_builtin("atomic_load_helper"));
}

/// The four qualified rows, in `QUALIFIED_FORMS` order, and the Lean
/// assumption the publication verdict carries. Card CG28: the compound
/// assignment and the increment are seq_cst read-modify-writes.
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
    let compound = classify_qualified(&compound).unwrap().unwrap();
    assert_eq!(
        (
            compound.operation,
            compound.order,
            compound.value_type.as_str()
        ),
        (
            Operation::ReadModifyWrite,
            Some(MemoryOrder::SequentiallyConsistent),
            "int"
        )
    );
    let increment = json!({"kind": "UnaryOperator", "opcode": "++", "type": {"qualType": "int"},
        "inner": [lhs]});
    let increment = classify_qualified(&increment).unwrap().unwrap();
    assert_eq!(increment.operation, Operation::ReadModifyWrite);
    assert_eq!(increment.form, QUALIFIED_FORMS[3].0);
    // A pointer-valued `_Atomic` increment is not routed.
    let pointer = json!({"kind": "UnaryOperator", "opcode": "++", "type": {"qualType": "int *"},
        "inner": [{"kind": "DeclRefExpr", "type": {"qualType": "_Atomic(int *)"},
            "referencedDecl": {"kind": "VarDecl", "name": "q"}}]});
    assert!(matches!(
        classify_qualified(&pointer),
        Some(Err(AtomicRouteRefusal::NonScalarValue(_)))
    ));
    // A `volatile` object that is not atomic is none of these forms.
    let volatile = json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
        "type": {"qualType": "volatile int"},
        "inner": [{"kind": "DeclRefExpr", "type": {"qualType": "volatile int"},
            "referencedDecl": {"kind": "VarDecl", "name": "v"}}]});
    assert!(classify_qualified(&volatile).is_none());
    // Card CG29 (rule 11z): `Unknown` carries the single-graph assumption,
    // `Safe` the all-executions one.
    assert!(LEAN.contains(
        "(if checked = some true ∧ covered = true then .allExecutions else .singleGraph) ::"
    ));
    assert!(SINGLE_GRAPH_ASSUMPTION.contains("one supplied candidate execution graph"));
    assert!(ALL_EXECUTIONS_ASSUMPTION.contains("every candidate execution"));
}
