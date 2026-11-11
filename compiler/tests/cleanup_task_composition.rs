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

//! Executed cleanup composed with a live task (card
//! `CT24-CLEANUP-TASK-COMPOSITION`).
//!
//! The region erases to its real cleanup release at the statement's source
//! position. The existing task ledger therefore decides the loan boundary:
//! destroying the lent owner before join refuses, while moving the same
//! cleanup after join and cleaning an independent owner both admit.

use std::path::{Path, PathBuf};

use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::task_execution::TaskInstructionKind;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/epoch_credit/CT24_COMPOSITION")
}

fn fixture(name: &str) -> SourceFile {
    let path = fixtures().join(name);
    SourceFile {
        path: format!("compiler/tests/fixtures/epoch_credit/CT24_COMPOSITION/{name}"),
        source: std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("fixture {} is readable: {error}", path.display()))
            .replace("\r\n", "\n"),
    }
}

fn compile(name: &str) -> Result<CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(&[fixture(name)], &CompileOptions::default())
}

#[test]
fn cleanup_of_the_lent_owner_before_join_is_rejected_at_the_cleanup() {
    let diagnostics = compile("before_join.qr").expect_err("the cleanup must refuse");
    assert_eq!(diagnostics.len(), 1, "one refusal, not a cascade");
    assert_eq!(diagnostics[0].code, "Q600");
    assert!(
        diagnostics[0].message.contains("LoanOutstanding"),
        "the existing task ledger attributes the refusal to the cleanup release: {}",
        diagnostics[0].message
    );
    assert!(
        diagnostics[0]
            .location
            .line_source
            .contains("error region R"),
        "the synthesized release stays at the cleanup statement: {:?}",
        diagnostics[0].location
    );
}

#[test]
fn the_same_cleanup_after_join_is_admitted_and_published() {
    let module = compile("after_join.qr").expect("the loan has expired");
    let composed = module
        .checked
        .as_composed_epoch_tasks()
        .expect("the composed route publishes evidence");
    let region = composed.region.as_ref().expect("cleanup evidence");
    assert_eq!(region.released, ["owner"]);
    assert_eq!(region.abandoned_credit, 3);
}

#[test]
fn cleanup_of_an_independent_owner_before_join_is_admitted() {
    let module =
        compile("independent_owner.qr").expect("a live task does not block independent cleanup");
    let composed = module
        .checked
        .as_composed_epoch_tasks()
        .expect("the composed route publishes evidence");
    let region = composed.region.as_ref().expect("cleanup evidence");
    assert_eq!(region.released, ["owner"]);
    assert_eq!(region.abandoned_credit, 3);

    // Publication describes the success arm on which the region is reached.
    // The spawn-failure arm still owns and releases the buffer independently;
    // erasure must not consume or hide that recovery path.
    let tasks = composed.tasks.module();
    let creator = &tasks.functions[&tasks.entry];
    let drops = creator
        .blocks
        .values()
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| {
            matches!(instruction.kind, TaskInstructionKind::DropBuffer { .. })
                .then_some(instruction.span)
        })
        .collect::<Vec<_>>();
    assert_eq!(drops.len(), 2, "cleanup arm plus spawn-failure recovery");
    assert_eq!(
        drops
            .iter()
            .filter(|span| region.span.start <= span.start && span.end <= region.span.end)
            .count(),
        1,
        "exactly one drop is synthesized by the cleanup"
    );
    assert_eq!(
        drops
            .iter()
            .filter(|span| span.end <= region.span.start || region.span.end <= span.start)
            .count(),
        1,
        "the spawn-failure arm keeps its written owner release"
    );
}
