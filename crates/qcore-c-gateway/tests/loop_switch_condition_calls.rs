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

//! Card B03-LOOP-AND-SWITCH: modeled custody calls in C controlling
//! expressions, through the real Clang-backed gateway.

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyCfg, LegacyDiagnosticKind, LegacyInstruction,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *); \
int r_release_code(void *p); int r_consume_code(void *p); int r_borrow_code(void *p);";

const LOOP: &str = "unmodeled call, aggregate or indirect access in loop condition";
const FOR: &str = "unmodeled call, aggregate or indirect access in for-loop condition";
const DO_WHILE: &str = "unmodeled call, aggregate or indirect access in do-while condition";
const SWITCH: &str = "unmodeled call, aggregate or indirect access in switch expression";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(
        &[
            (
                "q_base.qcore".to_owned(),
                include_str!("fixtures/contracts/q_base.qv").to_owned(),
            ),
            (
                "r_return_call.qcore".to_owned(),
                include_str!("fixtures/contracts/r_return_call.qv").to_owned(),
            ),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

#[derive(Debug)]
struct Run {
    status: AnalysisStatus,
    diagnostics: Vec<LegacyDiagnosticKind>,
    cfg: LegacyCfg,
}

impl Run {
    fn unsupported(&self) -> Vec<String> {
        self.diagnostics
            .iter()
            .filter_map(|kind| match kind {
                LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
                _ => None,
            })
            .collect()
    }

    fn double_releases(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|kind| matches!(kind, LegacyDiagnosticKind::DoubleRelease { .. }))
            .count()
    }
}

fn run(body: &str) -> Run {
    let source = format!("{DECLARATIONS}\n{body}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    let summary = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.expect("these fixtures project partially")
        }
    };
    let function = summary
        .functions
        .get("subject")
        .expect("the entry is projected");
    Run {
        status: function.status,
        diagnostics: function
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
        cfg: function.cfg.clone(),
    }
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent B03 test: {reason}");
            false
        }
    }
}

#[test]
fn loop_headers_evaluate_the_call_again_on_the_back_edge() {
    if !clang_available() {
        return;
    }
    for (header, refusal) in [
        ("while (r_release_code(p)) { }", LOOP),
        ("for (; r_release_code(p); ) { }", FOR),
        ("do { } while (r_release_code(p));", DO_WHILE),
    ] {
        let run = run(&format!(
            "void subject(void) {{ void *p = q_alloc(); {header} }}"
        ));
        assert!(
            !run.unsupported().contains(&refusal.to_owned()),
            "{header}: {:?}",
            run.unsupported()
        );
        assert_ne!(run.status, AnalysisStatus::Safe, "{header}");
        assert!(run.double_releases() > 0, "{header}: {run:?}");
    }
}

#[test]
fn switch_evaluates_the_call_once_before_any_arm() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) {
            void *p = q_alloc();
            switch (r_release_code(p)) { case 0: break; default: break; }
        }");
    assert_eq!(run.status, AnalysisStatus::Safe, "{run:?}");
    assert!(!run.unsupported().contains(&SWITCH.to_owned()), "{run:?}");
    assert_eq!(run.double_releases(), 0, "{run:?}");
}

#[test]
fn every_loop_keeps_its_exit_successor_after_modeling_the_call() {
    if !clang_available() {
        return;
    }
    for header in [
        "while (r_borrow_code(p)) { break; }",
        "for (; r_borrow_code(p); ) { break; }",
        "do { break; } while (r_borrow_code(p));",
    ] {
        let run = run(&format!(
            "void subject(void) {{ void *p = q_alloc(); {header} }}"
        ));
        assert_ne!(run.status, AnalysisStatus::Safe, "{header}: {run:?}");
        assert!(
            run.diagnostics
                .iter()
                .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
            "the retained exit must still expose the live acquisition: {header}: {run:?}"
        );
    }
}

/// Card W44-BRANCH-CONDITION-CALLS replaces the conditionally evaluated half of
/// this pin, deliberately: `ready && r_release_code(p)` is now desugared, the
/// release sitting on `ready`'s true edge (`legacy_condition_desugar.rs`). The
/// replacement asserts what the refusal protected: the path that skips the
/// release still leaks, and the back edge that re-evaluates it double-releases.
/// The post-release access keeps its refusal.
#[test]
fn conditionally_evaluated_calls_are_desugared_and_post_release_accesses_stay_refused() {
    if !clang_available() {
        return;
    }
    let desugared = run(
        "void subject(int ready) { void *p = q_alloc(); while (ready && r_release_code(p)) { } }",
    );
    assert!(
        !desugared.unsupported().contains(&LOOP.to_owned()),
        "{desugared:?}"
    );
    assert_eq!(desugared.status, AnalysisStatus::Unsafe, "{desugared:?}");
    assert!(
        desugared
            .diagnostics
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
        "{desugared:?}"
    );
    assert!(desugared.double_releases() > 0, "{desugared:?}");
    let refused = run(
        "struct cell { int value; }; void subject(struct cell *p) { while (r_release_code(p) + p->value) { } }",
    );
    assert!(
        refused.unsupported().contains(&LOOP.to_owned()),
        "{refused:?}"
    );
}

#[test]
fn continue_paths_revisit_the_call_and_for_runs_its_increment_first() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(void) {
            void *p = q_alloc();
            while (r_borrow_code(p)) {
                continue;
            }
            q_free(p);
        }",
        "void subject(void) {
            void *p = q_alloc();
            do {
                continue;
            } while (r_borrow_code(p));
            q_free(p);
        }",
    ] {
        let run = run(body);
        let header = run
            .cfg
            .nodes
            .iter()
            .position(|node| {
                matches!(&node.instruction,
                LegacyInstruction::ContractCall { function, .. } if function == "r_borrow_code")
            })
            .expect("condition call node");
        let continue_node = run
            .cfg
            .nodes
            .iter()
            .find(|node| {
                matches!(node.instruction, LegacyInstruction::NoOp) && node.successors == [header]
            })
            .expect("continue node");
        assert_eq!(continue_node.successors, [header], "{run:?}");
    }

    let run = run("void subject(void) {
            void *p = q_alloc(); unsigned long n = 0;
            for (; r_borrow_code(p); n = n + 1) {
                continue;
            }
            q_free(p);
        }");
    let header = run
        .cfg
        .nodes
        .iter()
        .position(|node| {
            matches!(&node.instruction,
            LegacyInstruction::ContractCall { function, .. } if function == "r_borrow_code")
        })
        .expect("condition call node");
    let increment = run
        .cfg
        .nodes
        .iter()
        .position(|node| {
            matches!(&node.instruction,
            LegacyInstruction::ForgetScalar { destination } if destination == "n")
        })
        .expect("for increment node");
    let continue_node = run
        .cfg
        .nodes
        .iter()
        .find(|node| {
            matches!(node.instruction, LegacyInstruction::NoOp) && node.successors == [increment]
        })
        .expect("continue node");
    assert_eq!(continue_node.successors, [increment], "{run:?}");
    assert_eq!(run.cfg.nodes[increment].successors, [header], "{run:?}");
}

#[test]
fn do_while_break_exits_before_the_condition_call() {
    if !clang_available() {
        return;
    }
    let run = run("void subject(void) {
            void *p = q_alloc();
            do { break; } while (r_release_code(p));
        }");
    assert!(!run.unsupported().contains(&DO_WHILE.to_owned()), "{run:?}");
    assert_eq!(run.double_releases(), 0, "{run:?}");
    assert!(
        run.diagnostics
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. })),
        "break must bypass the post-test call: {run:?}"
    );
}
