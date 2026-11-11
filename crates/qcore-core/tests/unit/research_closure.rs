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

use super::*;
use crate::{
    AccessFootprint, AccessKind, AdvancedOp, ConcurrentEvent, DestructorOutcome, DestructorSpec,
    SourceOp,
};
use std::collections::BTreeMap;

fn closed_program() -> ResearchProgram {
    ResearchProgram {
        sequential: SourceProgram::new(
            "main",
            vec![1],
            ["Failure".to_owned()],
            vec![
                SourceOp::Acquire {
                    resource: 1,
                    succeeds: true,
                    error: "Failure".into(),
                },
                SourceOp::Drop(1),
                SourceOp::Return(7),
            ],
        )
        .unwrap(),
        c11_stdout: "OK 7\nA 1\nR 1\n".into(),
        concurrent: ConcurrentProgram {
            initial_custody: BTreeMap::from([(9, 0)]),
            events: vec![ConcurrentEvent::Destroy {
                task: 0,
                resource: 9,
            }],
            ..ConcurrentProgram::default()
        },
        advanced: AdvancedProgram {
            destructors: vec![DestructorSpec {
                id: 3,
                outcome: DestructorOutcome::Ok,
            }],
            operations: vec![
                AdvancedOp::Acquire {
                    resource: 8,
                    destructor: 3,
                },
                AdvancedOp::Return(0),
            ],
            ..AdvancedProgram::default()
        },
    }
}

#[test]
fn closes_the_combined_language_with_all_ledgers_discharged() {
    let certificate = close_research_program(&closed_program()).unwrap();
    assert_eq!(certificate.translation.source, certificate.translation.c11);
    assert_eq!(certificate.progress, Progress::Terminal);
    assert_eq!(certificate.resources.sequential_identities, vec![1]);
    assert_eq!(certificate.resources.concurrent_identities, vec![9]);
    assert_eq!(certificate.resources.advanced_identities, vec![8]);
}

#[test]
fn target_mismatch_prevents_a_closure_certificate() {
    let mut program = closed_program();
    program.c11_stdout = "OK 8\nA 1\nR 1\n".into();
    assert!(matches!(
        close_research_program(&program),
        Err(ResearchClosureError::Translation(
            TranslationValidationError::CfgC11Mismatch { .. }
        ))
    ));
}

#[test]
fn a_concurrent_race_prevents_a_closure_certificate() {
    let mut program = closed_program();
    program.concurrent = ConcurrentProgram {
        events: vec![
            ConcurrentEvent::Spawn {
                parent: 0,
                child: 1,
            },
            ConcurrentEvent::Access {
                task: 0,
                footprint: AccessFootprint::new(4, 0, 1),
                kind: AccessKind::Write,
                guarded_by: Some(1),
            },
            ConcurrentEvent::Access {
                task: 1,
                footprint: AccessFootprint::new(4, 0, 1),
                kind: AccessKind::Read,
                guarded_by: Some(2),
            },
            ConcurrentEvent::Join {
                parent: 0,
                child: 1,
            },
        ],
        ..ConcurrentProgram::default()
    };
    assert!(matches!(
        close_research_program(&program),
        Err(ResearchClosureError::Concurrency(_))
    ));
}

#[test]
fn undestroyed_concurrent_custody_is_not_closed() {
    let mut program = closed_program();
    program.concurrent.events.clear();
    assert_eq!(
        close_research_program(&program),
        Err(ResearchClosureError::ConcurrentLedger)
    );
}

#[test]
fn cycle_17_promotes_every_cycle_16_research_boundary() {
    assert_eq!(PROMOTED_RESEARCH_QUESTIONS.len(), 6);
    assert!(OPEN_RESEARCH_QUESTIONS.is_empty());
    assert!(PROMOTED_RESEARCH_QUESTIONS.contains(&OpenResearchQuestion::PrincipalInference));
    assert!(PROMOTED_RESEARCH_QUESTIONS.contains(&OpenResearchQuestion::UnrestrictedLegacyC));
}
