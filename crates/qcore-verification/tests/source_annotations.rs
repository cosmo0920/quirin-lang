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

use qcore_core::{ExecutionOutcome, TraceEvent};
use qcore_frontend::frontend::SemanticPass;
use qcore_verification::driver::{
    CompileOptions, SourceExecutionProfile, SourceFile, compile_files_with_registry,
};
use qcore_verifier_host::VerifierRegistry;

fn compile(
    source: &str,
) -> Result<
    qcore_verification::driver::CompiledModule,
    Vec<qcore_verification::driver::FileDiagnostic>,
> {
    compile_files_with_registry(
        &[SourceFile::new("annotation.qcore", source)],
        &CompileOptions::default(),
        &VerifierRegistry::default(),
    )
}

fn refusal(source: &str) -> qcore_verification::driver::FileDiagnostic {
    compile(source)
        .expect_err("the annotated source must be refused")
        .remove(0)
}

#[test]
fn a_matching_i32_annotation_is_checked_and_erases_from_emission() {
    let annotated = compile("error Failure { Failed; } fn main() -> i32 { let n: i32 = 3; let r = qcore_acquire(1, true, Failure); drop r; return 3; }").unwrap();
    let inferred = compile("error Failure { Failed; } fn main() -> i32 { let n = 3; let r = qcore_acquire(1, true, Failure); drop r; return 3; }").unwrap();
    assert_eq!(
        annotated.execution_profile,
        SourceExecutionProfile::SequentialCleanupV1
    );
    assert_eq!(
        annotated.reference_observation,
        inferred.reference_observation
    );
    assert_eq!(annotated.c_source, inferred.c_source);
}

#[test]
fn scalar_mismatch_and_unsupported_annotation_refuse_at_ordinary_types() {
    for source in [
        "error Failure { Failed; } fn main() -> i32 { let n: unit = 3; let r = qcore_acquire(1, true, Failure); drop r; return 3; }",
        "error Failure { Failed; } fn main() -> i32 { let n: Widget = 3; let r = qcore_acquire(1, true, Failure); drop r; return 3; }",
        "error Failure { Failed; } fn main() -> i32 { let n Widget = 3; let r = qcore_acquire(1, true, Failure); drop r; return 3; }",
    ] {
        let diagnostic = refusal(source);
        assert_eq!(diagnostic.code, "Q020", "{diagnostic:?}");
        assert_eq!(diagnostic.pass, SemanticPass::OrdinaryTypes);
    }
}

#[test]
fn scalar_call_annotation_behavior_remains_compatible() {
    let module = compile("fn main() -> i32 { let n: i32 = 3; return n; }").unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ScalarCallsV1
    );
}

#[test]
fn acquisition_annotations_cannot_claim_an_unpublished_nominal_type() {
    for annotation in ["i32", "Widget", "own<File>"] {
        let source = format!(
            "error Failure {{ Failed; }} fn main() -> i32 {{ let r: {annotation} = qcore_acquire(1, true, Failure); drop r; return 0; }}"
        );
        let diagnostic = refusal(&source);
        assert_eq!(diagnostic.code, "Q020", "{diagnostic:?}");
        assert_eq!(diagnostic.pass, SemanticPass::OrdinaryTypes);
    }
    compile("error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(1, true, Failure); drop r; return 0; }").unwrap();
}

#[test]
fn region_acquisitions_cannot_bypass_the_annotation_gate() {
    let source = |head: &str| {
        format!(
            "error Failure {{ Failed; }} fn main() -> i32 {{ error region R {{ let r{head} = qcore_acquire(1, true, Failure); qcore_check(false, Failure); }} catch (Failure) quant open R as pack {{ discharge pack; }} return 0; }}"
        )
    };
    let untyped = compile(&source("")).unwrap();
    let observation = untyped.reference_observation.unwrap();
    assert_eq!(
        observation.outcome,
        ExecutionOutcome::Ok {
            value: 0,
            payload: Vec::new()
        }
    );
    assert_eq!(
        observation.trace,
        vec![TraceEvent::Acquire(1), TraceEvent::Release(1)]
    );
    assert!(untyped.c_source.unwrap().contains("q_release(&q_res_1"));
    for head in [": i32", ": Widget", ": own<File>"] {
        let diagnostic = refusal(&source(head));
        assert_eq!(diagnostic.code, "Q020", "{diagnostic:?}");
        assert_eq!(diagnostic.pass, SemanticPass::OrdinaryTypes);
    }
}

#[test]
fn post_desugaring_tokens_do_not_bypass_annotation_checking() {
    let diagnostic = refusal(
        "error Failure { Failed; } fn main() -> i32 { let r: i32 = 1 |> qcore_acquire(true, Failure); drop r; return 0; }",
    );
    assert_eq!(diagnostic.code, "Q036", "{diagnostic:?}");
    assert_eq!(diagnostic.pass, SemanticPass::Pipe);
}

#[test]
fn usage_annotations_keep_their_own_diagnostic() {
    let diagnostic = refusal(
        "error Failure { Failed; } fn main() -> i32 { let r: own<File> use zero = qcore_acquire(1, true, Failure); drop r; return 0; }",
    );
    assert_eq!(diagnostic.code, "Q100");
    assert_eq!(diagnostic.pass, SemanticPass::Usage);
}
