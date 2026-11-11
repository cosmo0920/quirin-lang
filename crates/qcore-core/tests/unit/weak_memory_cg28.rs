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

//! Card CG28-WEAK-MEMORY-EVENTS: read-modify-writes, compare-exchange,
//! fences and C11 release sequences in the execution checker, bound to
//! `proof/QCore/WeakMemory.lean`. The litmus executions are **parsed from the
//! Lean file** (each `def NAME : Execution := { … }`), run through
//! `check_c11_execution`, and their error categories compared with the
//! `litmusVectors` the Lean theorem `litmus_vectors_hold` proves.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/WeakMemory.lean");

fn event(id: u32, thread: u32, kind: MemoryEventKind) -> MemoryEvent {
    MemoryEvent { id, thread, kind }
}

fn lean() -> String {
    LEAN.replace("\r\n", "\n")
}

/// The Lean `Category` a Rust error belongs to (`None`: a category the Lean
/// fragment does not model, which no vector may produce).
fn category(error: &WeakMemoryError) -> Option<&'static str> {
    use WeakMemoryError as E;
    Some(match error {
        E::InvalidMemoryOrder(_) => ".invalidOrder",
        E::InvalidReadsFrom(_) | E::MissingReadsFrom(_) | E::MultipleReadsFrom(_) => ".readsFrom",
        E::InvalidModificationOrder(_) | E::IncompleteModificationOrder { .. } => {
            ".modificationOrder"
        }
        E::InvalidCompareExchange(_) => ".compareExchange",
        E::AtomicityViolation(_) => ".atomicity",
        E::InvalidSynchronizesWith(_) | E::MissingSynchronizesWith(_) => ".synchronizesWith",
        E::CyclicRelation(_) => ".cyclic",
        E::CoherenceViolation(_) => ".coherence",
        E::InvalidSequentiallyConsistentOrder => ".scOrder",
        E::DataRace { .. } => ".race",
        E::DuplicateEvent(_)
        | E::UnknownEndpoint(_)
        | E::InvalidMutexHistory(_)
        | E::ObservationMismatch => return None,
    })
}

/// The Lean `Category` constructors in declaration order: `check` lists
/// failing categories in this order.
fn category_order() -> Vec<String> {
    let lean = lean();
    let body = lean
        .split("inductive Category where")
        .nth(1)
        .expect("Category");
    let body = &body[..body.find("deriving").expect("deriving")];
    body.split('|')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| format!(".{name}"))
        .collect()
}

fn order(text: &str) -> MemoryOrder {
    match text {
        ".relaxed" => MemoryOrder::Relaxed,
        ".acquire" => MemoryOrder::Acquire,
        ".release" => MemoryOrder::Release,
        ".acqRel" => MemoryOrder::AcquireRelease,
        ".seqCst" => MemoryOrder::SequentiallyConsistent,
        other => panic!("unmapped Lean order {other}"),
    }
}

fn flag(text: &str) -> bool {
    match text {
        "true" => true,
        "false" => false,
        other => panic!("unmapped Lean Bool {other}"),
    }
}

/// One `ev ID THREAD (.kind ARGS)` item.
fn parse_event(item: &str) -> MemoryEvent {
    let (head, kind) = item.split_once('(').expect(item);
    let head = head.split_whitespace().collect::<Vec<_>>();
    assert_eq!(head[0], "ev", "{item}");
    let id = head[1].parse().unwrap();
    let thread = head[2].parse().unwrap();
    let kind = kind.trim().trim_end_matches(')');
    let words = kind.split_whitespace().collect::<Vec<_>>();
    let n = |index: usize| words[index].parse::<i64>().unwrap();
    let location = |index: usize| words[index].parse::<u32>().unwrap();
    let kind = match words[0] {
        ".init" => MemoryEventKind::AtomicInit {
            location: location(1),
            value: n(2),
        },
        ".store" => MemoryEventKind::AtomicStore {
            location: location(1),
            value: n(2),
            order: order(words[3]),
        },
        ".load" => MemoryEventKind::AtomicLoad {
            location: location(1),
            value: n(2),
            order: order(words[3]),
        },
        ".rmw" => MemoryEventKind::AtomicReadModifyWrite {
            location: location(1),
            read_value: n(2),
            written_value: n(3),
            order: order(words[4]),
        },
        ".cas" => MemoryEventKind::AtomicCompareExchange {
            location: location(1),
            expected: n(2),
            observed: n(3),
            desired: n(4),
            success: order(words[5]),
            failure: order(words[6]),
            strength: if flag(words[7]) {
                CompareExchangeStrength::Weak
            } else {
                CompareExchangeStrength::Strong
            },
            succeeded: flag(words[8]),
        },
        ".fence" => MemoryEventKind::AtomicFence {
            order: order(words[1]),
            scope: if flag(words[2]) {
                FenceScope::Signal
            } else {
                FenceScope::Thread
            },
        },
        ".naRead" => MemoryEventKind::NonAtomicRead {
            location: location(1),
        },
        ".naWrite" => MemoryEventKind::NonAtomicWrite {
            location: location(1),
        },
        other => panic!("unmapped Lean kind {other}"),
    };
    MemoryEvent { id, thread, kind }
}

/// The bracketed list after `field := `.
fn field<'a>(body: &'a str, field: &str) -> &'a str {
    let start = body.find(&format!("{field} := [")).expect(field) + field.len() + 5;
    let rest = &body[start..];
    &rest[..rest.find(']').expect("list end")]
}

fn pairs(list: &str) -> Vec<RelationEdge> {
    list.split(')')
        .map(|item| {
            item.trim_start_matches([',', ' ', '\n'])
                .trim_start_matches('(')
        })
        .filter(|item| !item.trim().is_empty())
        .map(|item| {
            let (from, to) = item.split_once(',').expect(item);
            RelationEdge::new(from.trim().parse().unwrap(), to.trim().parse().unwrap())
        })
        .collect()
}

/// `def NAME : Execution := { … }` from the Lean file.
fn lean_execution(name: &str) -> C11Execution {
    let lean = lean();
    let body = lean
        .split(&format!("def {name} : Execution :=\n"))
        .nth(1)
        .unwrap_or_else(|| panic!("no Lean execution {name}"));
    let body = &body[..body.find(" }\n").expect("execution end")];
    let events = field(body, "events")
        .split("ev ")
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| parse_event(&format!("ev {}", item.trim_end_matches(','))))
        .collect();
    let sc = field(body, "sc");
    C11Execution {
        events,
        reads_from: pairs(field(body, "rf")),
        modification_order: pairs(field(body, "mo")),
        synchronizes_with: pairs(field(body, "sw")),
        sequentially_consistent: sc
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| item.parse().unwrap())
            .collect(),
        ..C11Execution::default()
    }
}

/// The Rust checker's failing categories, in the Lean order.
fn rust_categories(execution: &C11Execution) -> Vec<String> {
    let errors = check_c11_execution(execution).err().unwrap_or_default();
    let mut found = errors
        .iter()
        .map(|error| {
            category(error)
                .unwrap_or_else(|| panic!("error outside the Lean fragment: {error:?}"))
                .to_owned()
        })
        .collect::<Vec<_>>();
    let order = category_order();
    found.sort_by_key(|name| order.iter().position(|known| known == name).expect(name));
    found.dedup();
    found
}

/// `litmusVectors`, row by row.
fn lean_vectors() -> Vec<(String, Vec<String>)> {
    let lean = lean();
    let body = lean
        .split("def litmusVectors : List (String × Execution × List Category) := [")
        .nth(1)
        .expect("litmusVectors");
    let body = &body[..body.find("\n]").expect("list end")];
    body.lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let name = line.split('"').nth(1).unwrap().to_owned();
            let categories = line
                .rsplit_once('[')
                .unwrap()
                .1
                .split(']')
                .next()
                .unwrap()
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_owned)
                .collect();
            (name, categories)
        })
        .collect()
}

#[test]
fn cg28_lean_litmus_vectors_match_the_rust_checker() {
    let vectors = lean_vectors();
    assert_eq!(vectors.len(), 20);
    for (name, expected) in vectors {
        let execution = lean_execution(&name);
        assert_eq!(
            rust_categories(&execution),
            expected,
            "{name}: {:?}",
            check_c11_execution(&execution)
        );
    }
}

/// The brief's named outcomes, independent of the parser: message passing
/// with release/acquire and with fences is accepted, relaxed message passing
/// races, a weak compare-exchange loop is accepted.
#[test]
fn cg28_named_litmus_outcomes() {
    assert!(check_c11_execution(&lean_execution("mpReleaseAcquire")).is_ok());
    assert!(check_c11_execution(&lean_execution("mpFences")).is_ok());
    let relaxed = check_c11_execution(&lean_execution("mpRelaxed")).unwrap_err();
    assert!(matches!(
        relaxed.as_slice(),
        [WeakMemoryError::DataRace {
            first: 1,
            second: 4
        }]
    ));
    let happens_before = check_c11_execution(&lean_execution("mpFences"))
        .unwrap()
        .happens_before;
    assert!(happens_before.contains(&RelationEdge::new(1, 6)));
    assert!(check_c11_execution(&lean_execution("casWeakLoop")).is_ok());
    let strong = check_c11_execution(&lean_execution("casStrongSpurious")).unwrap_err();
    assert_eq!(strong, vec![WeakMemoryError::InvalidCompareExchange(2)]);
    let lost = check_c11_execution(&lean_execution("rmwLostUpdate")).unwrap_err();
    assert_eq!(lost, vec![WeakMemoryError::AtomicityViolation(2)]);
}

/// `compare_exchange_failure_admitted` is `QCore.WeakMemory.casOrdersValid`
/// on all 25 pairs: the failure order's rank (Lean `loadRank`) is at most the
/// success order's load part (`successRank`).
#[test]
fn cg28_cas_order_validity_matches_lean() {
    let lean = lean();
    let rank = |definition: &str, order: &str| -> Option<u32> {
        let body = lean.split(definition).nth(1).unwrap();
        let body = &body[..body.find("\n\n").unwrap()];
        body.lines().find_map(|line| {
            let line = line.trim().trim_start_matches("| ");
            let (patterns, value) = line.split_once(" => ")?;
            patterns
                .split(" | ")
                .any(|pattern| pattern.trim() == order)
                .then(|| value.trim().trim_start_matches("some ").parse().ok())
                .flatten()
        })
    };
    let orders = [
        (".relaxed", MemoryOrder::Relaxed),
        (".acquire", MemoryOrder::Acquire),
        (".release", MemoryOrder::Release),
        (".acqRel", MemoryOrder::AcquireRelease),
        (".seqCst", MemoryOrder::SequentiallyConsistent),
    ];
    for (success_name, success) in orders {
        for (failure_name, failure) in orders {
            let failure_rank = rank("def Order.loadRank : Order → Option Nat", failure_name);
            let success_rank =
                rank("def Order.successRank : Order → Nat", success_name).expect(success_name);
            let lean_valid = failure_rank.is_some_and(|rank| rank <= success_rank);
            assert_eq!(
                compare_exchange_failure_admitted(success, failure),
                lean_valid,
                "{success_name} {failure_name}"
            );
            let kind = MemoryEventKind::AtomicCompareExchange {
                location: 1,
                expected: 0,
                observed: 0,
                desired: 1,
                success,
                failure,
                strength: CompareExchangeStrength::Strong,
                succeeded: true,
            };
            assert_eq!(event_order_represented(&kind), lean_valid);
        }
    }
}

/// Every order is represented for an RMW and a fence (a relaxed fence has no
/// effect); a signal fence creates no synchronisation even when supplied.
#[test]
fn cg28_rmw_and_fence_orders_and_signal_scope() {
    for order in [
        MemoryOrder::Relaxed,
        MemoryOrder::Acquire,
        MemoryOrder::Release,
        MemoryOrder::AcquireRelease,
        MemoryOrder::SequentiallyConsistent,
    ] {
        assert!(event_order_represented(
            &MemoryEventKind::AtomicReadModifyWrite {
                location: 1,
                read_value: 0,
                written_value: 1,
                order,
            }
        ));
        for scope in [FenceScope::Thread, FenceScope::Signal] {
            assert!(event_order_represented(&MemoryEventKind::AtomicFence {
                order,
                scope
            }));
        }
    }
    let claimed = check_c11_execution(&lean_execution("mpSignalFencesClaimed")).unwrap_err();
    assert_eq!(
        claimed,
        vec![WeakMemoryError::InvalidSynchronizesWith(RelationEdge::new(
            2, 5
        ))]
    );
}

/// A seq_cst RMW is a member of S and an SC read: omitting it from S is
/// refused; a relaxed RMW in S is refused too.
#[test]
fn cg28_seq_cst_rmw_and_fence_join_s() {
    let execution = |sc: Vec<MemoryEventId>, order| C11Execution {
        events: vec![
            event(
                0,
                0,
                MemoryEventKind::AtomicInit {
                    location: 1,
                    value: 0,
                },
            ),
            event(
                1,
                1,
                MemoryEventKind::AtomicReadModifyWrite {
                    location: 1,
                    read_value: 0,
                    written_value: 1,
                    order,
                },
            ),
            event(
                2,
                1,
                MemoryEventKind::AtomicFence {
                    order: MemoryOrder::SequentiallyConsistent,
                    scope: FenceScope::Thread,
                },
            ),
        ],
        reads_from: vec![RelationEdge::new(0, 1)],
        modification_order: vec![RelationEdge::new(0, 1)],
        sequentially_consistent: sc,
        ..C11Execution::default()
    };
    assert!(
        check_c11_execution(&execution(vec![1, 2], MemoryOrder::SequentiallyConsistent)).is_ok()
    );
    for (sc, order) in [
        (vec![2], MemoryOrder::SequentiallyConsistent),
        (vec![1, 2], MemoryOrder::Relaxed),
        (vec![2, 1], MemoryOrder::SequentiallyConsistent),
    ] {
        assert_eq!(
            check_c11_execution(&execution(sc.clone(), order)).unwrap_err(),
            vec![WeakMemoryError::InvalidSequentiallyConsistentOrder],
            "{sc:?} {order:?}"
        );
    }
}
