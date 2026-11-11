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

//! Card CG29-EXECUTION-COVERAGE-AND-RECLAMATION: hazard-pointer and
//! epoch-based reclamation, bound to `proof/QCore/Reclamation.lean`. The
//! traces and the `hpVectors` / `ebrVectors` tables are **parsed from the
//! Lean file**, run through the checkers, and the first refused step (index
//! and violation) compared with the tables the Lean theorems
//! `hp_vectors_hold` and `ebr_vectors_hold` prove.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/Reclamation.lean");

fn lean() -> String {
    LEAN.replace("\r\n", "\n")
}

fn definition(name: &str, type_name: &str) -> String {
    let lean = lean();
    let head = format!("def {name} : {type_name} :=");
    let start = lean.find(&head).unwrap_or_else(|| panic!("{head}")) + head.len();
    let rest = &lean[start..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    rest[..end].to_owned()
}

/// The items of a `[a, b, …]` list at top bracket depth.
fn items(text: &str) -> Vec<String> {
    let text = text.trim();
    let inner = &text[text.find('[').unwrap() + 1..text.rfind(']').unwrap()];
    let mut result = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in inner.chars() {
        match c {
            '(' | '[' => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => result.push(std::mem::take(&mut current).trim().to_owned()),
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        result.push(current.trim().to_owned());
    }
    result
}

fn event(item: &str) -> ReclamationEvent {
    use ReclamationEvent as E;
    let words = item.split_whitespace().collect::<Vec<_>>();
    let n = |index: usize| words[index].parse::<u32>().unwrap();
    match words[0] {
        ".link" => E::Link { object: n(1) },
        ".unlink" => E::Unlink { object: n(1) },
        ".protect" => E::Protect {
            thread: n(1),
            slot: n(2),
            object: n(3),
        },
        ".validate" => E::Validate {
            thread: n(1),
            slot: n(2),
            object: n(3),
        },
        ".clear" => E::Clear {
            thread: n(1),
            slot: n(2),
        },
        ".access" => E::Access {
            thread: n(1),
            slot: n(2),
            object: n(3),
        },
        ".retire" => E::Retire {
            thread: n(1),
            object: n(2),
        },
        ".reclaim" => E::Reclaim {
            thread: n(1),
            object: n(2),
        },
        ".enter" => E::Enter { thread: n(1) },
        ".exit" => E::Exit { thread: n(1) },
        ".advance" => E::Advance { thread: n(1) },
        other => panic!("unmapped Lean event {other}"),
    }
}

fn violation(name: &str) -> ReclamationViolation {
    use ReclamationViolation as V;
    match name {
        ".notFresh" => V::NotFresh,
        ".notLinked" => V::NotLinked,
        ".outOfRange" => V::OutOfRange,
        ".validationFailed" => V::ValidationFailed,
        ".unprotectedAccess" => V::UnprotectedAccess,
        ".notUnlinked" => V::NotUnlinked,
        ".notRetired" => V::NotRetired,
        ".hazardHeld" => V::HazardHeld,
        ".foreignEvent" => V::ForeignEvent,
        ".notActive" => V::NotActive,
        ".alreadyActive" => V::AlreadyActive,
        ".epochNotQuiescent" => V::EpochNotQuiescent,
        ".gracePeriod" => V::GracePeriod,
        other => panic!("unmapped Lean violation {other}"),
    }
}

/// `(name, trace, outcome)` rows of a vector table.
fn table(name: &str) -> Vec<(String, Vec<ReclamationEvent>, Result<(), ProtocolViolation>)> {
    let body = definition(name, "List (String × List Ev × Option (Nat × Violation))");
    items(&body)
        .into_iter()
        .map(|row| {
            let fields = items(&format!("[{}]", &row[1..row.len() - 1]));
            let label = fields[0].trim_matches('"').to_owned();
            let trace = items(&definition(&fields[1], "List Ev"))
                .iter()
                .map(|item| event(item))
                .collect();
            let outcome = if fields[2] == "none" {
                Ok(())
            } else {
                let inner = fields[2]
                    .trim_start_matches("some")
                    .trim()
                    .trim_start_matches('(')
                    .trim_end_matches(')');
                let (index, name) = inner.split_once(',').unwrap();
                Err(ProtocolViolation {
                    index: index.trim().parse().unwrap(),
                    violation: violation(name.trim()),
                })
            };
            (label, trace, outcome)
        })
        .collect()
}

#[test]
fn lean_trace_vectors_agree() {
    let hp = table("hpVectors");
    assert_eq!(hp.len(), 6);
    for (name, trace, outcome) in &hp {
        assert_eq!(&check_hazard_pointer_trace(2, 1, trace), outcome, "{name}");
    }
    let ebr = table("ebrVectors");
    assert_eq!(ebr.len(), 5);
    for (name, trace, outcome) in &ebr {
        assert_eq!(&check_epoch_trace(2, trace), outcome, "{name}");
    }
}

fn call(thread: u32, operation: &str, object: Option<u32>, slot: Option<u32>) -> ProtocolCall {
    ProtocolCall {
        thread,
        operation: operation.to_owned(),
        object,
        slot,
    }
}

fn operation(name: &str, role: ProtocolRole) -> OperationContract {
    OperationContract {
        name: name.to_owned(),
        role,
    }
}

fn hazard_contract(handshake: HandshakeOrdering) -> ReclamationContract {
    ReclamationContract {
        protocol: ReclamationProtocol::HazardPointers {
            threads: 2,
            slots: 1,
        },
        operations: vec![
            operation("list_insert", ProtocolRole::Link),
            operation("list_remove", ProtocolRole::Unlink),
            operation("hp_protect", ProtocolRole::ProtectAndValidate),
            operation("hp_clear", ProtocolRole::Clear),
            operation("node_read", ProtocolRole::Access),
            operation("hp_retire", ProtocolRole::Retire),
            operation("hp_scan_free", ProtocolRole::Reclaim),
        ],
        handshake,
    }
}

const FENCED: HandshakeOrdering = HandshakeOrdering {
    access_order: MemoryOrder::Relaxed,
    fences: true,
};

fn protected_read() -> Vec<ProtocolCall> {
    vec![
        call(1, "list_insert", Some(1), None),
        call(0, "hp_protect", Some(1), Some(0)),
        call(1, "list_remove", Some(1), None),
        call(1, "hp_retire", Some(1), None),
        call(0, "node_read", Some(1), Some(0)),
        call(0, "hp_clear", None, Some(0)),
        call(1, "hp_scan_free", Some(1), None),
    ]
}

/// The contract layer: a hazard-pointer contract whose helpers are checked
/// through the protocol, with each helper boundary `Declared` and the
/// handshake decided over every execution.
#[test]
fn hazard_pointer_contract_is_checked_not_trusted() {
    let certificate =
        check_reclamation(&hazard_contract(FENCED), &protected_read()).expect("protected read");
    assert_eq!(
        certificate.steps, 8,
        "hp_protect is a protect and a validate"
    );
    let HandshakeBasis::ExecutionCoverage(coverage) = &certificate.handshake else {
        panic!("{:?}", certificate.handshake);
    };
    assert!(coverage.contains("every candidate execution"));
    for helper in [
        "list_insert",
        "hp_protect",
        "list_remove",
        "hp_retire",
        "node_read",
        "hp_clear",
        "hp_scan_free",
    ] {
        assert!(
            certificate.assumptions.iter().any(|assumption| assumption
                .starts_with("Declared boundary: helper `")
                && assumption.contains(&format!("`{helper}`"))),
            "{helper}"
        );
    }
    // A relaxed handshake without fences races: the contract is refused.
    assert_eq!(
        check_reclamation(
            &hazard_contract(HandshakeOrdering {
                access_order: MemoryOrder::Relaxed,
                fences: false,
            }),
            &protected_read()
        ),
        Err(ReclamationRefusal::HandshakeRaces)
    );
    // seq_cst accesses without fences are race-free.
    assert!(matches!(
        check_reclamation(
            &hazard_contract(HandshakeOrdering {
                access_order: MemoryOrder::SequentiallyConsistent,
                fences: false,
            }),
            &protected_read()
        ),
        Ok(ReclamationCertificate {
            handshake: HandshakeBasis::ExecutionCoverage(_),
            ..
        })
    ));
    // Contracts are checked: a role of the other protocol, a duplicate, an
    // uncontracted call and a missing operand are refused.
    let mut foreign = hazard_contract(FENCED);
    foreign
        .operations
        .push(operation("epoch_enter", ProtocolRole::Enter));
    assert!(matches!(
        check_reclamation(&foreign, &protected_read()),
        Err(ReclamationRefusal::RoleOfOtherProtocol { .. })
    ));
    let mut duplicate = hazard_contract(FENCED);
    duplicate
        .operations
        .push(operation("node_read", ProtocolRole::Retire));
    assert_eq!(
        check_reclamation(&duplicate, &protected_read()),
        Err(ReclamationRefusal::DuplicateOperation(
            "node_read".to_owned()
        ))
    );
    let mut calls = protected_read();
    calls.push(call(0, "free", Some(1), None));
    assert_eq!(
        check_reclamation(&hazard_contract(FENCED), &calls),
        Err(ReclamationRefusal::UncontractedOperation("free".to_owned()))
    );
    let mut calls = protected_read();
    calls[1].slot = None;
    assert!(matches!(
        check_reclamation(&hazard_contract(FENCED), &calls),
        Err(ReclamationRefusal::MissingOperand { index: 1, .. })
    ));
}

/// The brief's hazard-pointer control: a use after retire is never `Safe`.
/// Without a validated hazard the access is refused; a hazard published
/// after the unlink cannot be validated; a reclaim while the hazard is held
/// is refused.
#[test]
fn hazard_pointer_use_after_retire_is_never_safe() {
    let contract = hazard_contract(FENCED);
    let unprotected = vec![
        call(1, "list_insert", Some(1), None),
        call(1, "list_remove", Some(1), None),
        call(1, "hp_retire", Some(1), None),
        call(0, "node_read", Some(1), Some(0)),
    ];
    assert_eq!(
        check_reclamation(&contract, &unprotected),
        Err(ReclamationRefusal::Protocol(ProtocolViolation {
            index: 3,
            violation: ReclamationViolation::UnprotectedAccess,
        }))
    );
    let late = vec![
        call(1, "list_insert", Some(1), None),
        call(1, "list_remove", Some(1), None),
        call(1, "hp_retire", Some(1), None),
        call(0, "hp_protect", Some(1), Some(0)),
        call(0, "node_read", Some(1), Some(0)),
    ];
    assert_eq!(
        check_reclamation(&contract, &late),
        Err(ReclamationRefusal::Protocol(ProtocolViolation {
            index: 4,
            violation: ReclamationViolation::ValidationFailed,
        }))
    );
    let early_free = vec![
        call(1, "list_insert", Some(1), None),
        call(0, "hp_protect", Some(1), Some(0)),
        call(1, "list_remove", Some(1), None),
        call(1, "hp_retire", Some(1), None),
        call(1, "hp_scan_free", Some(1), None),
    ];
    assert_eq!(
        check_reclamation(&contract, &early_free),
        Err(ReclamationRefusal::Protocol(ProtocolViolation {
            index: 5,
            violation: ReclamationViolation::HazardHeld,
        }))
    );
    // And the access after the reclaim, had the reclaim been admitted.
    let after = vec![
        call(1, "list_insert", Some(1), None),
        call(1, "list_remove", Some(1), None),
        call(1, "hp_retire", Some(1), None),
        call(1, "hp_scan_free", Some(1), None),
        call(0, "node_read", Some(1), Some(0)),
    ];
    assert!(matches!(
        check_reclamation(&contract, &after),
        Err(ReclamationRefusal::Protocol(ProtocolViolation {
            violation: ReclamationViolation::UnprotectedAccess,
            ..
        }))
    ));
}

#[test]
fn epoch_contract_waits_two_epochs() {
    let contract = ReclamationContract {
        protocol: ReclamationProtocol::EpochBased { threads: 2 },
        operations: vec![
            operation("insert", ProtocolRole::Link),
            operation("remove", ProtocolRole::Unlink),
            operation("ebr_enter", ProtocolRole::Enter),
            operation("ebr_exit", ProtocolRole::Exit),
            operation("read", ProtocolRole::Access),
            operation("ebr_retire", ProtocolRole::Retire),
            operation("ebr_try_advance", ProtocolRole::Advance),
            operation("ebr_free", ProtocolRole::Reclaim),
        ],
        handshake: FENCED,
    };
    let grace = vec![
        call(1, "insert", Some(1), None),
        call(0, "ebr_enter", None, None),
        call(0, "read", Some(1), None),
        call(1, "remove", Some(1), None),
        call(1, "ebr_retire", Some(1), None),
        call(0, "read", Some(1), None),
        call(0, "ebr_exit", None, None),
        call(1, "ebr_try_advance", None, None),
        call(1, "ebr_try_advance", None, None),
        call(1, "ebr_free", Some(1), None),
    ];
    let certificate = check_reclamation(&contract, &grace).expect("grace period");
    assert_eq!(certificate.steps, 10);
    let mut early = grace.clone();
    early.remove(8);
    assert_eq!(
        check_reclamation(&contract, &early),
        Err(ReclamationRefusal::Protocol(ProtocolViolation {
            index: 8,
            violation: ReclamationViolation::GracePeriod,
        }))
    );
    // A reader still in the old epoch stalls the advance.
    let mut stalled = grace;
    stalled.remove(6);
    assert_eq!(
        check_reclamation(&contract, &stalled),
        Err(ReclamationRefusal::Protocol(ProtocolViolation {
            index: 7,
            violation: ReclamationViolation::EpochNotQuiescent,
        }))
    );
}
