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

//! End-to-end route selection and refusals for `source.epoch.bound.v1`
//! (card T08-EPOCH-SRC), the source form of T08's `FlatEpochBoundV1`.
//!
//! `compile_files` must reach the flat-epoch profile from ordinary source,
//! publish a certificate it independently rechecked, and route every refusal of
//! the profile to the core's own `Q630`-`Q641` code — never to a fallback, and
//! never to another route. The checker itself is untouched by this card: every
//! assertion below is about which program the lowering hands it and which code
//! comes back.
//!
//! The four fixtures under `tests/fixtures/ct/CT3*/` are the CT35, CT36, CT38
//! and CT39 mappings (card T08-EPOCH-SRC proposed them; card T08-SRC-CT mapped
//! them and moved the directories into the harness's layout, see
//! `docs/reports/wave8-epoch-ct/README.md`). `compiler/tests/ct_corpus.rs` now
//! also runs all four through the manifest's stage-derived route; the tests
//! here are the independent second measurement of the same fixture bytes and
//! are deliberately kept.
//!
//! **This route is check-only.** An admitted epoch is an accounting statement
//! about declared shapes; there is no reference execution, no emitted C and no
//! native run, so no test below claims one.
use std::path::{Path, PathBuf};

use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ct")
}

fn fixture(case: &str, file: &str) -> SourceFile {
    let path = fixture_root().join(case).join(file);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixture {} is readable: {error}", path.display()));
    SourceFile {
        path: path.display().to_string(),
        source,
    }
}

fn compile_source(file: SourceFile) -> Result<qcore_compiler::CompiledModule, Vec<String>> {
    compile_files(&[file], &CompileOptions::default()).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.code.to_string())
            .collect()
    })
}

fn compile(source: &str) -> Result<qcore_compiler::CompiledModule, Vec<String>> {
    compile_source(SourceFile {
        path: "epoch.qcore".into(),
        source: source.into(),
    })
}

fn compile_case(case: &str, file: &str) -> Result<qcore_compiler::CompiledModule, Vec<String>> {
    compile_source(fixture(case, file))
}

/// The certificate of an accepted module, with the route asserted.
fn certificate_of(
    module: &qcore_compiler::CompiledModule,
) -> &qcore_core::epoch_accounting::EpochBoundCertificate {
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::FlatEpochBoundV1,
        "an epoch module selects the flat-epoch route"
    );
    assert!(
        module.c_source.is_none() && module.reference_observation.is_none(),
        "the flat-epoch route is check-only: it emits no C and runs no reference execution"
    );
    assert!(module.checked.as_sequential().is_none());
    assert!(module.checked.as_scoped_tasks().is_none());
    assert!(module.checked.as_checked_mutex_guards().is_none());
    &module
        .checked
        .as_checked_epoch_bound()
        .expect("the checked artifact variant is explicit for this route")
        .certificate
}

fn memory(quantity: &qcore_core::resource_flow::Quantity) -> u64 {
    quantity.get("memory")
}

fn work(quantity: &qcore_core::resource_flow::Quantity) -> u64 {
    quantity.get("effort")
}

// ---------------------------------------------------------------------------
// CT35 — one shared 4096-byte allocation read by two tasks: count 4096 once.
// ---------------------------------------------------------------------------

#[test]
fn ct35_counts_one_shared_allocation_once_however_many_tasks_read_it() {
    let module = compile_case("CT35", "case.qr").expect("CT35 is an admitted epoch");
    let certificate = certificate_of(&module);
    assert_eq!(
        memory(&certificate.shared_entry_union),
        4096,
        "two readers of one 4096-byte cell contribute 4096, not 8192"
    );
    // The bound is exactly union + summed peaks + declared overhead, so the
    // union rule is visible in the published figure and not just in a field.
    assert_eq!(memory(&certificate.epoch_peak), 4096 + 2048 + 512 + 2 * 128);
    assert_eq!(certificate.task_count, 2);

    // Control: two *distinct* cells each read by both tasks are two footprints.
    // The union counts identities, so this is 8192 — a different epoch, not a
    // different reading of the same one.
    let two_cells = compile_case("CT35", "two_cells.qr").expect("the control is admitted");
    let two_cells = certificate_of(&two_cells);
    assert_eq!(memory(&two_cells.shared_entry_union), 8192);

    // Control: one task naming the same cell twice is refused, not silently
    // deduplicated — a repeated entry is evidence the plan is summing.
    assert_eq!(
        compile_case("CT35", "repeated_entry.qr").unwrap_err(),
        vec!["Q630"]
    );
}

// ---------------------------------------------------------------------------
// CT36 — concurrent incremental peaks 3072 and 5120: at least 8192 plus
// modeled overhead.
// ---------------------------------------------------------------------------

#[test]
fn ct36_sums_concurrent_private_peaks_and_adds_the_declared_overhead() {
    let module = compile_case("CT36", "case.qr").expect("CT36 is an admitted epoch");
    let certificate = certificate_of(&module);
    assert_eq!(
        memory(&certificate.concurrent_private_peak),
        8192,
        "independent tasks may run at once, so their peaks are summed"
    );
    assert_eq!(memory(&certificate.runtime_overhead), 512 + 2 * 128);
    assert_eq!(memory(&certificate.epoch_peak), 8192 + 768);
    assert!(
        memory(&certificate.epoch_peak) >= 8192 + memory(&certificate.runtime_overhead),
        "the bound is at least the summed peaks plus the modeled overhead"
    );

    // The two controls straddle the summed figure: 8192 is admitted and 8191 is
    // refused. A maximized reading would have called 5120 the peak and admitted
    // both, so the refusal is what makes the summing falsifiable.
    compile_case("CT36", "summed_allocation.qr").expect("the summed peak fits exactly 8192");
    assert_eq!(
        compile_case("CT36", "maximized_allocation.qr").unwrap_err(),
        vec!["Q637", "Q638"],
        "one byte short of the summed peak refuses on the credits and on the epoch"
    );
}

// ---------------------------------------------------------------------------
// CT38 — child credits splitting the total to more than the original credit.
// ---------------------------------------------------------------------------

#[test]
fn ct38_refuses_child_credits_that_sum_beyond_the_epoch_allocation() {
    assert_eq!(
        compile_case("CT38", "case.qr").unwrap_err(),
        vec!["Q637"],
        "6144 + 6144 granted out of an 8192 allocation is refused, and only that"
    );
    // Control: the same epoch with credits that really do split the total.
    let module = compile_case("CT38", "split_within_total.qr").expect("4096 + 4096 splits 8192");
    let certificate = certificate_of(&module);
    assert_eq!(memory(&certificate.granted_credits), 8192);
    assert_eq!(memory(&certificate.allocation), 8192);
}

// ---------------------------------------------------------------------------
// CT39 — a concurrent work bound read as a wall-clock bound.
// ---------------------------------------------------------------------------

#[test]
fn ct39_refuses_reading_the_work_bound_as_a_wall_clock_bound() {
    assert_eq!(
        compile_case("CT39", "case.qr").unwrap_err(),
        vec!["Q639"],
        "the epoch is well accounted; it is the time claim that does not follow"
    );
    // Control: the same epoch without the claim is admitted, and its work total
    // is published as work — a quantity, never a duration. The profile records
    // that it has no schedule model on the certificate itself, so a consumer
    // cannot drop the premise and read the total as a finish time.
    let module = compile_case("CT39", "work_bound_only.qr").expect("the epoch itself is fine");
    let certificate = certificate_of(&module);
    assert_eq!(work(&certificate.work_total), 800);
    assert!(
        certificate
            .assumptions
            .contains(&qcore_core::epoch_accounting::EpochAssumption::NoScheduleModel)
    );
    assert!(
        qcore_core::epoch_accounting::epoch_wall_clock_bound(certificate).is_err(),
        "no admitted epoch ever yields a wall-clock bound"
    );
}

// ---------------------------------------------------------------------------
// The rest of the core's band, reached from source.
// ---------------------------------------------------------------------------

/// A complete epoch with one clause replaced, so each refusal below differs
/// from the admitted module by exactly what its name says.
fn epoch(workers: &str) -> String {
    format!(
        "#import std.concurrency;\nfn main() -> unit {{\n\
         thread scope e epoch {{\n\
         let page = share 4096;\n\
         bound 65536 effort 1000;\n\
         overhead scope 512 thread 128;\n\
         {workers}\n\
         }}\n\
         }}\n"
    )
}

#[test]
fn the_admitted_baseline_of_the_refusal_family_checks() {
    let module = compile(&epoch(
        "thread scope w { read page; peak 1024; effort 10; credit 4096 effort 100; join(drop); }",
    ))
    .expect("the baseline is admitted");
    let certificate = certificate_of(&module);
    assert_eq!(certificate.task_count, 1);
    assert_eq!(memory(&certificate.exit_residency), 4096 + 512);
}

#[test]
fn a_result_that_outlives_its_task_must_fit_the_peak_it_was_accounted_for() {
    assert_eq!(
        compile(&epoch(
            "thread scope w { peak 128; retained 256; credit 4096 effort 100; join(move); }"
        ))
        .unwrap_err(),
        vec!["Q633"]
    );
    // Control: a result inside the peak is admitted, and a transferred result
    // is still live after the join.
    let module = compile(&epoch(
        "thread scope w { peak 1024; retained 256; credit 4096 effort 100; join(move); }",
    ))
    .expect("a result inside the peak is admitted");
    assert_eq!(memory(&certificate_of(&module).exit_residency), 512 + 256);
}

#[test]
fn a_completed_child_result_stays_live_until_the_creator_actually_takes_it() {
    // `join(close)` is the claim that completion reclaimed the result.
    assert_eq!(
        compile(&epoch(
            "thread scope w { peak 1024; retained 256; credit 4096 effort 100; join(close); }"
        ))
        .unwrap_err(),
        vec!["Q634"]
    );
    // No join at all, with a result: the result is never transferred.
    assert_eq!(
        compile(&epoch(
            "thread scope w { peak 1024; retained 256; credit 4096 effort 100; }"
        ))
        .unwrap_err(),
        vec!["Q635"]
    );
    // No join at all, without a result: the task's own overhead still leaks.
    assert_eq!(
        compile(&epoch(
            "thread scope w { peak 1024; credit 4096 effort 100; }"
        ))
        .unwrap_err(),
        vec!["Q635"]
    );
    // A retained result the creator releases is gone at exit; one it takes is
    // not. Both are admitted; the difference is visible in the residency.
    let released = compile(&epoch(
        "thread scope w { peak 1024; retained 256; credit 4096 effort 100; join(drop); }",
    ))
    .expect("released at join");
    let taken = compile(&epoch(
        "thread scope w { peak 1024; retained 256; credit 4096 effort 100; join(move); }",
    ))
    .expect("taken at join");
    assert_eq!(memory(&certificate_of(&released).exit_residency), 512);
    assert_eq!(memory(&certificate_of(&taken).exit_residency), 512 + 256);
    // Completion itself reclaims nothing on either module: with every task
    // finished and none joined, the result and the per-task overhead are both
    // still live. (These workers read no shared cell, and a cell no task reads
    // is not part of the entry allocation, so the union contributes nothing.)
    for module in [&released, &taken] {
        assert_eq!(
            memory(&certificate_of(module).completed_residency),
            256 + 512 + 128
        );
    }
}

#[test]
fn a_task_beyond_its_own_credit_is_refused_on_the_axis_it_exceeds() {
    // memory: 1024 peak + 128 per-task overhead > 1024 credit.
    assert_eq!(
        compile(&epoch(
            "thread scope w { peak 1024; effort 10; credit 1024 effort 100; join(drop); }"
        ))
        .unwrap_err(),
        vec!["Q636"]
    );
    // work: the work axis is checked exactly like the memory axis.
    assert_eq!(
        compile(&epoch(
            "thread scope w { peak 1024; effort 101; credit 4096 effort 100; join(drop); }"
        ))
        .unwrap_err(),
        vec!["Q636"]
    );
}

#[test]
fn an_epoch_beyond_its_allocation_is_refused_even_when_every_credit_fits() {
    // Shared storage and the epoch's own overhead belong to no task's credit,
    // so credits fitting is not by itself an epoch that fits.
    let source = "#import std.concurrency;\nfn main() -> unit {\n\
         thread scope e epoch {\n\
         let page = share 4096;\n\
         bound 4200 effort 1000;\n\
         overhead scope 0 thread 0;\n\
         thread scope w { read page; peak 128; credit 128 effort 0; join(drop); }\n\
         }\n\
         }\n";
    assert_eq!(compile(source).unwrap_err(), vec!["Q638"]);
}

#[test]
fn an_unsupported_epoch_shape_is_a_represented_refusal_not_a_silent_omission() {
    // A worker that writes shared storage: this profile admits shared reads.
    assert_eq!(
        compile(&epoch(
            "thread scope w { write page; peak 1024; credit 4096 effort 100; join(drop); }"
        ))
        .unwrap_err(),
        vec!["Q632"]
    );
    // A worker that creates a worker: the epoch would not be flat.
    assert_eq!(
        compile(&epoch(
            "thread scope w { peak 1024; credit 4096 effort 100; join(drop); \
             thread scope inner { peak 1; } }"
        ))
        .unwrap_err(),
        vec!["Q632"]
    );
}

#[test]
fn a_measured_run_offered_as_the_declared_overhead_is_refused() {
    let source = "#import std.concurrency;\nfn main() -> unit {\n\
         thread scope e epoch {\n\
         bound 65536 effort 1000;\n\
         overhead scope 512 thread 128 measured 30;\n\
         thread scope w { peak 1024; credit 4096 effort 100; join(drop); }\n\
         }\n\
         }\n";
    assert_eq!(compile(source).unwrap_err(), vec!["Q640"]);
}

#[test]
fn structural_epoch_refusals_precede_every_accounting_figure() {
    // An epoch with no tasks.
    let empty = "#import std.concurrency;\nfn main() -> unit {\n\
         thread scope e epoch { bound 65536 effort 1000; overhead scope 512 thread 128; }\n\
         }\n";
    assert_eq!(compile(empty).unwrap_err(), vec!["Q630"]);
    // A retained result with no footprint is not a retained result.
    assert_eq!(
        compile(&epoch(
            "thread scope w { peak 1024; retained 0; credit 4096 effort 100; join(move); }"
        ))
        .unwrap_err(),
        vec!["Q630"]
    );
    // A shared entry that names something other than a declared cell keeps its
    // own identity, so the core reports the unknown cell rather than this
    // grammar deciding what an unmodeled name meant.
    assert_eq!(
        compile(&epoch(
            "thread scope w { read main; peak 1024; credit 4096 effort 100; join(drop); }"
        ))
        .unwrap_err(),
        vec!["Q631"]
    );
}

#[test]
fn quantitative_overflow_is_refused_rather_than_wrapped() {
    let source = format!(
        "#import std.concurrency;\nfn main() -> unit {{\n\
         thread scope e epoch {{\n\
         bound {max} effort {max};\n\
         overhead scope 0 thread 0;\n\
         thread scope a {{ peak {max}; credit {max} effort 0; join(drop); }}\n\
         thread scope b {{ peak {max}; credit {max} effort 0; join(drop); }}\n\
         }}\n\
         }}\n",
        max = u64::MAX
    );
    assert_eq!(compile(&source).unwrap_err(), vec!["Q641"]);
}

// ---------------------------------------------------------------------------
// Route boundaries.
// ---------------------------------------------------------------------------

#[test]
fn an_epoch_module_needs_the_explicit_concurrency_import() {
    let source = epoch("thread scope w { peak 1; credit 4096 effort 100; join(drop); }")
        .replace("#import std.concurrency;\n", "");
    assert_eq!(compile(&source).unwrap_err(), vec!["Q015"]);
}

#[test]
fn forms_outside_the_epoch_grammar_are_refused_on_this_route() {
    // A clause word this profile does not have.
    assert_eq!(
        compile(&epoch(
            "thread scope w { peak 1024; credit 4096 effort 100; join(drop); open page; }"
        ))
        .unwrap_err(),
        vec!["Q600"]
    );
    // A worker with no credit clause: its share of the allocation is not a
    // figure this lowering may invent.
    assert_eq!(
        compile(&epoch("thread scope w { peak 1024; join(drop); }")).unwrap_err(),
        vec!["Q600"]
    );
    // Two join clauses in one worker.
    assert_eq!(
        compile(&epoch(
            "thread scope w { peak 1024; credit 4096 effort 100; join(drop); join(move); }"
        ))
        .unwrap_err(),
        vec!["Q600"]
    );
    // `join(close)` with nothing retained claims a result that does not exist.
    assert_eq!(
        compile(&epoch(
            "thread scope w { peak 1024; credit 4096 effort 100; join(close); }"
        ))
        .unwrap_err(),
        vec!["Q600"]
    );
    // The task profile's spawn/join expressions are a different model: their
    // captures, owned values and completion capsules have no representation in
    // an accounting statement about declared shapes.
    let spawned = "#import std.concurrency;\nfn worker() -> unit { }\n\
         fn main() -> unit {\n\
         thread scope e epoch {\n\
         bound 65536 effort 1000;\n\
         overhead scope 512 thread 128;\n\
         let h = spawn e worker();\n\
         }\n\
         }\n";
    assert_eq!(compile(spawned).unwrap_err(), vec!["Q600"]);
    // A missing allocation or overhead header is a refusal, not a default.
    let no_allocation = "#import std.concurrency;\nfn main() -> unit {\n\
         thread scope e epoch {\n\
         overhead scope 512 thread 128;\n\
         thread scope w { peak 1; credit 4096 effort 100; join(drop); }\n\
         }\n\
         }\n";
    assert_eq!(compile(no_allocation).unwrap_err(), vec!["Q600"]);
    let no_overhead = "#import std.concurrency;\nfn main() -> unit {\n\
         thread scope e epoch {\n\
         bound 65536 effort 1000;\n\
         thread scope w { peak 1; credit 4096 effort 100; join(drop); }\n\
         }\n\
         }\n";
    assert_eq!(compile(no_overhead).unwrap_err(), vec!["Q600"]);
}

#[test]
fn a_module_without_the_epoch_marker_keeps_its_existing_route() {
    // The same `thread scope` without `epoch` is the scoped task route, which
    // refuses this body on its own terms. The point is only that the epoch
    // route did not claim it.
    let source = "#import std.concurrency;\nfn main() -> unit {\n\
         thread scope s {\n\
         let buf = protected 4;\n\
         let m = mutex guards buf;\n\
         with lock m as g { open g; write buf with g; close g; }\n\
         }\n\
         }\n";
    let module = compile(source).expect("the mutex route still checks its own fixture");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::CheckedMutexGuardsV1
    );
}
