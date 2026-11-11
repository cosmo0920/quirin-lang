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

//! These tests link the trusted adapters without qcore-compiler or any provider.
use qcore_core::cfg::ExecutionOutcome;
use qcore_core::chunk_lifecycle::ChunkOperation;
use qcore_core::scoped_tasks::TaskOutcome;
use qcore_core::task_execution::{TaskExecutionScript, execute_task_module};
use qcore_schemas::profile::OperationKind;
use qcore_verification::driver::{
    CompileOptions, SourceExecutionProfile, SourceFile, compile_files_with_registry,
};
use qcore_verification::library_import::*;
use qcore_verifier_host::VerifierRegistry;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn sequential_source_compiles_with_only_an_explicit_empty_registry() {
    let module = compile_files_with_registry(
        &[
            SourceFile::new("helper.qcore", "fn helper()->i32 { return 1; }"),
            SourceFile::new(
                "application.qcore",
                "fn main()->i32 { helper(); return 23; }",
            ),
        ],
        &CompileOptions::default(),
        &VerifierRegistry::default(),
    )
    .unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::SequentialCleanupV1
    );
    assert_eq!(module.checked.function_count(), 2);
    assert_eq!(
        module.reference_observation.unwrap().outcome,
        ExecutionOutcome::Ok {
            value: 23,
            payload: vec![]
        }
    );
    assert!(module.c_source.unwrap().contains("application.qcore"));
    assert!(
        module
            .source_map
            .iter()
            .all(|entry| entry.source.path == "application.qcore")
    );
}

#[test]
fn owned_task_source_uses_the_same_checked_kernel_without_compiler_adapters() {
    let source = "#import std.concurrency; concurrency checked; fn worker(v:i32)->own<Buffer> effects {alloc,throws<AllocError>} {let b=qcore_buffer_new(v)?;return move b;} fn main()->i32 effects {throws<AllocError>,throws<SpawnError>,spawn,join,may_block} {let v=42;thread scope s {let h=spawn s worker(copy v)?;let b=join move h?;let value=qcore_buffer_read(b);drop b;return value;}}";
    let result = compile_files_with_registry(
        &[SourceFile::new("job.qcore", source)],
        &CompileOptions::default(),
        &VerifierRegistry::default(),
    );
    if !cfg!(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows"
    )) {
        assert!(result.is_err());
        return;
    }
    let module = result.unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ScopedOwnedTasksV1
    );
    assert!(module.checked.as_sequential().is_none());
    let observed = execute_task_module(
        module.checked.as_scoped_tasks().unwrap(),
        &TaskExecutionScript::default(),
    )
    .unwrap();
    assert_eq!(observed.root_outcome, TaskOutcome::I32(42));
    assert_eq!(
        observed.tasks[1].outcome,
        TaskOutcome::OwnedBuffer(4294967297)
    );
    assert!(observed.tasks[1].joined);
    assert!(module.c_source.unwrap().contains("thrd_create"));
}

#[test]
fn explicit_registry_never_skips_requested_verification_or_unchecked_contracts() {
    for (source, code) in [
        (
            "verify scope check { plugin example.absent profile default capabilities {resource_contracts}; } fn main()->i32 {return 0;}",
            "Q700",
        ),
        (
            "fn main()->i32 contract { quant normal wall_clock <= 5; } {return 0;}",
            "Q021",
        ),
    ] {
        let errors = compile_files_with_registry(
            &[SourceFile::new("rejected.qcore", source)],
            &CompileOptions::default(),
            &VerifierRegistry::default(),
        )
        .unwrap_err();
        assert!(errors.iter().any(|error| error.code == code), "{errors:?}");
    }
}

#[test]
fn generic_model_import_retains_declared_mechanism_and_revision_gates() {
    let program = ImportedLibraryCustodyProgram {
        profile: LibrarySummaryProfile {
            source_revision: "declared-revision".into(),
            library: "example.cache".into(),
            summaries: BTreeSet::from([LibraryMechanismSummary::ReferenceCount]),
        },
        custody: LibraryCustodyProgram {
            operations: vec![
                LibraryCustodyOperation::Acquire {
                    resource: 1,
                    holder: ExternalHolder::Local(0),
                },
                LibraryCustodyOperation::Retain {
                    resource: 1,
                    from: ExternalHolder::Local(0),
                    retained: ExternalHolder::Local(0),
                },
                LibraryCustodyOperation::Release {
                    resource: 1,
                    holder: ExternalHolder::Local(0),
                },
                LibraryCustodyOperation::Release {
                    resource: 1,
                    holder: ExternalHolder::Local(0),
                },
            ],
            ..Default::default()
        },
    };
    let permitted = program.profile.summaries.clone();
    let certificate =
        check_imported_library_custody(&program, &permitted, "declared-revision").unwrap();
    assert_eq!(certificate.library, "example.cache");
    assert_eq!(
        certificate.custody.discharged_resources,
        BTreeSet::from([1])
    );
    assert!(
        check_imported_library_custody(&program, &BTreeSet::new(), "declared-revision").is_err()
    );
    assert!(check_imported_library_custody(&program, &permitted, "different-revision").is_err());
    let mut incomplete = program;
    incomplete.profile.summaries.clear();
    assert!(
        check_imported_library_custody(&incomplete, &permitted, "declared-revision")
            .unwrap_err()
            .contains(&LibraryImportDiagnostic::MissingSummary(
                LibraryMechanismSummary::ReferenceCount
            ))
    );
}

#[test]
fn generic_lifecycle_import_requires_a_complete_declared_operation_inventory() {
    let table = BTreeMap::from([
        (OperationKind::CreateContext, "example.create".into()),
        (OperationKind::DestroyContext, "example.destroy".into()),
    ]);
    let mut program = ImportedChunkProgram {
        profile: ChunkImportProfile {
            source_revision: "declared-revision".into(),
            control_flow_complete: true,
            summaries: table.values().cloned().collect(),
        },
        operations: vec![
            ChunkOperation::CreateContext { context: 1 },
            ChunkOperation::DestroyContext { context: 1 },
        ],
    };
    assert!(check_imported_chunk_program(&program, &table, "declared-revision").is_ok());
    assert!(
        check_imported_chunk_program(&program, &BTreeMap::new(), "declared-revision")
            .unwrap_err()
            .iter()
            .any(|error| matches!(error, ChunkImportDiagnostic::UnmodeledOperation(_)))
    );
    program.profile.control_flow_complete = false;
    assert!(
        check_imported_chunk_program(&program, &table, "declared-revision")
            .unwrap_err()
            .contains(&ChunkImportDiagnostic::IncompleteControlFlow)
    );
}
