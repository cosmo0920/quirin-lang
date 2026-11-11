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

//! Read-loan captures and owner adoption on the composed epoch task route
//! (card `REGION-JOINS-LOAN`, wave 30) — CT25's loan half, completing the
//! scenario whose duty half card `REGION-JOINS-DUTIES` landed: *"Cleanup
//! destroys owner after joins **and loan expiry**"*, expected **Accept**.
//!
//! Two admissions, one deliberate discharge:
//!
//! * A **read-loan capture** — the task route's own `read PLACE share N` at a
//!   spawn argument — is admitted, declared in the plan as the place-free
//!   `read;` clause. This deliberately discharges `T08-COMPOSE-LOAN`: the
//!   refusal's stated reason ("a read loan's residency is shared with the
//!   lender, which this flat accounting does not represent") dissolves on
//!   this route, because the lender is always the **creator** — a spawned
//!   worker is a leaf function and lends nothing — and creator-scope bytes
//!   are already outside the epoch's allocation (`REGION-JOINS-EPOCH-AXIS`).
//! * An **adoption** — `move NAME;` in the composed region body — takes a
//!   live entry owner's cleanup duty into the pack. The statement erases
//!   completely; the discharge's `drop NAME;` is written into the blanks
//!   after every join, where the **unedited** task route custody-checks it.
//!
//! No custody rule is re-implemented beside the erasure (constraint 15m):
//! "destroying the owner while a share is live refuses" is the task ledger's
//! own `LoanOutstanding` over the erased (or written) drop, and "the loan
//! expires at the join" is that ledger's own share return, ordered before the
//! erased drop by the wave-28 after-joins rule (`Q600`).
//!
//! Every refusal test asserts the exact code, never `is_err()`.
use std::path::{Path, PathBuf};

use qcore_compiler::{
    CompileOptions, CompiledModule, ComposedEpochModule, SourceFile, compile_files,
};
use qcore_core::error_region::RegionOp;
use qcore_frontend::source_epoch_tasks::ComposedCapture;

/// The CT25 loan fixtures. Card `REGION-JOINS-LOAN` parked them under
/// `compiler/tests/fixtures/epoch_credit/CT25_LOAN/` because `ct_corpus.rs`
/// fails closed when an unmapped case owns a fixture directory under
/// `fixtures/ct/`; card `CT-WIRE-LANE-3` (wave 31) mapped CT25 and moved the
/// directory to `compiler/tests/fixtures/ct/CT25/` — the CT22/CT23 precedent —
/// so one program is the evidence in both places, and this suite keeps running
/// on the moved bytes.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ct/CT25")
}

fn fixture(name: &str) -> SourceFile {
    let path = fixtures().join(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixture {} is readable: {error}", path.display()))
        .replace("\r\n", "\n");
    SourceFile {
        path: format!("compiler/tests/fixtures/ct/CT25/{name}"),
        source,
    }
}

fn compile(file: SourceFile) -> Result<CompiledModule, Vec<String>> {
    compile_files(&[file], &CompileOptions::default()).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.code.to_string())
            .collect()
    })
}

fn accept(name: &str) -> CompiledModule {
    compile(fixture(name)).unwrap_or_else(|codes| panic!("{name} is admitted, got {codes:?}"))
}

fn refuse(name: &str) -> Vec<String> {
    compile(fixture(name))
        .err()
        .unwrap_or_else(|| panic!("{name} is refused"))
}

fn message(file: SourceFile) -> String {
    compile_files(&[file], &CompileOptions::default())
        .expect_err("the module is refused")
        .remove(0)
        .message
}

fn composed(module: &CompiledModule) -> &ComposedEpochModule {
    module
        .checked
        .as_composed_epoch_tasks()
        .expect("the composed route publishes both halves")
}

/// **CT25, both halves in one program.** The parent owns `owner`, lends the
/// worker a counted read share of it at the spawn, joins — the loan expires
/// there, under the unedited task ledger — and only then does the region
/// adopt the owner and its discharge destroy it. Asserted, word by word of
/// the case text: the capture really is a loan (`Read`, derived from the
/// checked IR and declared as `read;`); every task fact precedes the region
/// (*after joins*, and therefore after the share's return — *after loan
/// expiry*); and the discharge destroys the owner (`released = ["owner"]`,
/// abandoning exactly the parent-local remainder).
#[test]
fn ct25_the_cleanup_destroys_the_lent_owner_after_joins_and_loan_expiry() {
    let module = accept("case.qr");
    let evidence = composed(&module)
        .region
        .as_ref()
        .expect("the entry's region is published");
    assert_eq!(evidence.released, vec!["owner".to_owned()]);
    assert_eq!(evidence.abandoned_credit, 3);

    // The operations handed to the unedited model carry the adoption as the
    // pack acquisition it is.
    let [RegionOp::ErrorRegion { body, .. }] = evidence.operations.as_slice() else {
        panic!("one region operation is published");
    };
    assert_eq!(
        body.as_slice(),
        [
            RegionOp::Acquire {
                resource: 1,
                destructor_noexcept: true,
            },
            RegionOp::AddStrictCredit(5),
            RegionOp::SpendStrictCredit(2),
            RegionOp::Raise("Failure".to_owned()),
        ]
    );

    // The capture is a loan, derived from the checked task IR and reconciled
    // against the declared `read;` clause.
    let composed_module = composed(&module);
    assert_eq!(composed_module.composed.ledger.len(), 1);
    assert_eq!(
        composed_module.composed.ledger[0].captures,
        vec![ComposedCapture::Read]
    );
    assert_eq!(
        composed_module.composed.rows[0].captures,
        vec![ComposedCapture::Read]
    );

    // "After joins and loan expiry" is a fact of the checked task IR plus the
    // unedited ledger: the share returns at the join, and the one creation's
    // span ends before the region begins.
    assert!(composed_module.composed.ledger[0].span.end < evidence.span.start);

    // The destruction is emitted: the erased task half carries the owner's
    // allocation and the release the discharge decided.
    let c = module.c_source.as_deref().expect("the module emits");
    assert!(c.contains("qcore_buffer_new(INT32_C(42)"));
}

/// The CT25 loan shape as one in-memory module with the region's eight lines
/// substituted. Every variant has the same line count and the same path, so
/// emitted C, source maps and reference observations are comparable byte for
/// byte with nothing excused by a filename.
fn with_region(region: &str) -> SourceFile {
    const SHAPE: &str = r#"#import std.concurrency;
concurrency checked;
error Failure { Failed; }
fn reader(payload: borrow<Buffer>) -> i32 {
    let seen = qcore_buffer_read(payload);
    return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    thread scope plan epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope reader {
            read;
            peak 1024;
            effort 10;
            credit 4096 effort 100;
        }
    }
    let owner = qcore_buffer_new(42)?;
    thread scope s {
        match spawn s reader(read owner share FULL) {
            Result::Ok { value: handle } => {
                let seen = join move handle?;
            }
            Result::Err { error: failure } => {
                drop owner;
                return 0;
            }
        }
REGION
        return 1;
    }
}
"#;
    SourceFile {
        path: "region_joins_loan/shape.qcore".to_owned(),
        source: SHAPE.replace("REGION", region),
    }
}

const ADOPTED: &str = r#"        error region R {
            move owner;
            credit 5;
            effort 2;
            qcore_check(false, Failure);
        } catch (Failure) quant open R as pack {
            discharge pack;
        }"#;

/// The hand-written straight-line twin of [`ADOPTED`]: the adoption erases
/// completely — the owner's acquisition already stands in the entry's own
/// text — and the discharge's release is written into the first blank run of
/// the erased statement, which is the region's own first line. Computed here
/// by hand — this constant is the test's independent oracle, not a readback
/// of the split.
fn adopted_twin() -> String {
    [
        format!("        {:<16}", "drop owner;"),
        " ".repeat(23),
        " ".repeat(21),
        " ".repeat(21),
        " ".repeat(40),
        " ".repeat(48),
        " ".repeat(27),
        " ".repeat(9),
    ]
    .join("\n")
}

fn compile_shape(file: SourceFile) -> Result<CompiledModule, Vec<String>> {
    compile(file)
}

/// **The erasure, byte for byte, against a hand-written twin.** One module,
/// one path, one line count: the emitted C, the entry and the reference
/// observation of the adopted-owner region variant all equal the twin's — the
/// straight-line program the region *is*, written by hand at the same byte
/// offsets, with the loan and its expiry standing unchanged in both. The
/// source maps carry identical positions row for row, at most the synthesized
/// release's rendering apart (its position lies inside the region statement's
/// own extent, so the region variant renders the original file's line there —
/// the same attribution rule every erased span of this route has).
#[test]
fn the_adopted_region_erases_into_its_straight_line_twin_byte_for_byte() {
    let with = compile_shape(with_region(ADOPTED)).expect("the region variant is admitted");
    let twin = compile_shape(with_region(&adopted_twin())).expect("the twin is admitted");
    assert_eq!(with.c_source, twin.c_source);
    assert_eq!(with.entry, twin.entry);
    assert_eq!(with.reference_observation, twin.reference_observation);
    let positions = |maps: &[qcore_compiler::EmittedSourceMap]| {
        maps.iter()
            .map(|map| {
                (
                    map.generated_start_line,
                    map.generated_end_line,
                    map.source.path.clone(),
                    map.source.line,
                    map.source.column,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(positions(&with.source_map), positions(&twin.source_map));
    let renderings_apart = with
        .source_map
        .iter()
        .zip(&twin.source_map)
        .filter(|(ours, theirs)| ours.source.line_source != theirs.source.line_source)
        .count();
    assert!(
        renderings_apart <= 1,
        "only the synthesized release's rendering may differ"
    );
}

/// The epoch's certificates are unmoved by the loan and the adoption: the
/// bound and the credit-recovery split are byte-identical between the
/// adopted-owner region and the duty-free wave-28 shape over the same lending
/// program — the lent owner's bytes are the creator's, outside the epoch's
/// allocation exactly as its strict credit is (`REGION-JOINS-EPOCH-AXIS`
/// unchanged).
#[test]
fn the_epoch_certificates_are_unmoved_by_a_loan_and_an_adoption() {
    let adopted = compile_shape(with_region(ADOPTED)).expect("the adopted region is admitted");
    // The duty-free control releases the owner in written text — a duty-free
    // region blanks completely, so the program must do its own cleanup.
    let duty_free = compile_shape(with_region(
        r#"        drop owner;
        error region R {
            credit 5;
            effort 2;
            qcore_check(false, Failure);
        } catch (Failure) quant open R as pack {
            discharge pack;
        }
"#
        .trim_end(),
    ))
    .expect("the duty-free region is admitted");
    assert_eq!(
        composed(&adopted).certificate,
        composed(&duty_free).certificate
    );
    assert_eq!(
        composed(&adopted).credit_recovery,
        composed(&duty_free).credit_recovery
    );
    let free = composed(&duty_free).region.as_ref().expect("published");
    assert_eq!(free.released, Vec::<String>::new());
    assert_eq!(free.abandoned_credit, 3);
}

/// **Destroying the owner while a share is live refuses**, at the task
/// route's own custody rule over the written drop — no rule of this card, and
/// exactly the refusal the erased drop would meet if the region could precede
/// a join (which the wave-28 `Q600` ordering rule already forbids).
#[test]
fn a_live_share_refuses_the_destruction() {
    assert_eq!(refuse("live_share_drop.qr"), vec!["Q600"]);
    assert!(
        message(fixture("live_share_drop.qr")).contains("LoanOutstanding"),
        "the refusal is the task ledger's own custody rule, naming the loan"
    );
}

/// The wave-28 after-joins rule, re-verified over an adopted owner: a region
/// moved before the join is refused. The erased `drop owner;` would stand
/// while the share is out, and the composition refuses before emitting any
/// such program — here at the task route's own custody `Q600`, the erased
/// program's real defect.
#[test]
fn the_region_cannot_precede_the_loan_expiry() {
    let mut file = fixture("case.qr");
    // Move the whole region statement above the match: the drop it erases to
    // would then run while the share is still out.
    let source = &mut file.source;
    let match_start = source.find("        match spawn").expect("match");
    let region_start = source.find("        error region R {").expect("region");
    let tail_start = source.find("        return 1;").expect("return");
    let joins = source[match_start..region_start].trim_end().to_owned();
    let region = source[region_start..tail_start].trim_end().to_owned();
    source.replace_range(match_start..tail_start, &format!("{region}\n{joins}\n"));
    let refused = compile_files(&[file], &CompileOptions::default())
        .expect_err("a region before the join is refused");
    assert_eq!(
        refused
            .iter()
            .map(|diagnostic| diagnostic.code.to_string())
            .collect::<Vec<_>>(),
        vec!["Q600"]
    );
}

/// An adopted owner released in the body erases its duty — the model's frame
/// rule through the adoption shape: the pack is duty-free again, the kept
/// `drop owner;` stands at its own offsets in the task half, and only the
/// remainder is abandoned.
#[test]
fn an_adopted_owner_released_in_the_body_erases_its_duty() {
    let module = accept("adopted_then_dropped.qr");
    let evidence = composed(&module)
        .region
        .as_ref()
        .expect("the entry's region is published");
    assert_eq!(evidence.released, Vec::<String>::new());
    assert_eq!(evidence.abandoned_credit, 3);
}

/// The adoption reader's own shape refusals, each at `Q302` with the rule
/// named: a name the entry does not bind, and one owner taken twice.
#[test]
fn the_adoption_reader_refuses_its_own_shapes() {
    assert_eq!(refuse("adopt_unknown.qr"), vec!["Q302"]);
    assert!(message(fixture("adopt_unknown.qr")).contains("which the entry does not bind"));

    assert_eq!(refuse("adopt_twice.qr"), vec!["Q302"]);
    assert!(message(fixture("adopt_twice.qr")).contains("takes `owner` twice"));
}

/// An adopted name whose binding is not an owned Buffer in the erased program
/// is the **task route's** question, not this reader's: the reader admits the
/// shape and the unedited lowering refuses the erased `drop` at its own code
/// (`Q500`, the executable-subset gate) — custody is not re-implemented
/// beside the erasure (constraint 15m).
#[test]
fn an_adopted_scalar_is_refused_by_the_unedited_task_route() {
    let refused = refuse("adopt_scalar.qr");
    assert_eq!(refused, vec!["Q500"]);
    assert!(
        message(fixture("adopt_scalar.qr")).contains("drop requires an owned buffer"),
        "the refusal is the task route's own"
    );
}

/// A region acquisition beside an adopted owner is the multi-owner boundary
/// reached through this card's shape: the acquisition's `?` failure edge
/// discards the live adopted owner in the erased program, refused at the task
/// route's existing custody `Q600` (`REGION-JOINS-MULTI-OWNER`, unchanged).
#[test]
fn an_acquisition_beside_an_adopted_owner_keeps_the_multi_owner_boundary() {
    let refused = refuse("adopt_and_acquire.qr");
    assert_eq!(refused, vec!["Q600"]);
    assert!(
        message(fixture("adopt_and_acquire.qr")).contains("discards"),
        "the refusal is the task route's own custody rule"
    );
}

/// The declared and the derived capture list are still compared elementwise:
/// a `copy;` clause against a loan creation, and a `read;` clause against a
/// copy creation, both refuse at `Q600` naming both lists.
#[test]
fn the_capture_reconciliation_reads_loans_in_both_directions() {
    let mut copy_for_loan = fixture("case.qr");
    copy_for_loan.source = copy_for_loan
        .source
        .replace("            read;\n", "            copy;\n");
    let refused = compile_files(&[copy_for_loan.clone()], &CompileOptions::default())
        .expect_err("a copy clause against a loan creation is refused");
    assert_eq!(refused[0].code, "Q600");
    assert!(refused[0].message.contains("declares captures [copy]"));
    assert!(refused[0].message.contains("passes [read]"));

    let mut read_for_copy = fixture("case.qr");
    read_for_copy.source = read_for_copy
        .source
        .replace("reader(read owner share FULL)", "reader_copy(copy value)")
        .replace(
            "fn reader(payload: borrow<Buffer>) -> i32 {\n    let seen = qcore_buffer_read(payload);\n    return seen;\n}",
            "fn reader_copy(value: i32) -> i32 {\n    return value;\n}",
        )
        .replace("thread scope reader {", "thread scope reader_copy {")
        .replace(
            "    let owner = qcore_buffer_new(42)?;\n",
            "    let owner = qcore_buffer_new(42)?;\n    let value = 7;\n",
        );
    let refused = compile_files(&[read_for_copy], &CompileOptions::default())
        .expect_err("a read clause against a copy creation is refused");
    assert_eq!(refused[0].code, "Q600");
    assert!(refused[0].message.contains("declares captures [read]"));
    assert!(refused[0].message.contains("passes [copy]"));
}

/// Native positive control (the family's `owned_source_tasks` pattern): the
/// CT25 loan case's emitted C, compiled with the platform C compiler and run
/// with counting allocator hooks, frees **every** buffer it allocates — here
/// exactly one, the lent owner, destroyed by the discharge's erased release —
/// and exits successfully. A discharge that lost the adopted duty would
/// surface here as an unfreed allocation, not only as a disagreement.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn the_native_run_frees_the_lent_owner() {
    use std::process::Command;

    let module = accept("case.qr");
    let directory = std::env::temp_dir().join(format!(
        "qcore-region-loan-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let c = directory.join("module.c");
    let executable = directory.join("module");
    std::fs::write(&c, module.c_source.as_deref().unwrap()).unwrap();
    let hooks = directory.join("hooks.h");
    std::fs::write(
        &hooks,
        r#"
#include <stdio.h>
#include <stdlib.h>
static void *counting_alloc(size_t size) { void *p = malloc(size); fprintf(stderr, "BUFFER_ALLOC %p\n", p); return p; }
static void counting_free(void *p) { fprintf(stderr, "BUFFER_FREE %p\n", p); free(p); }
#define QCORE_BUFFER_MALLOC counting_alloc
#define QCORE_BUFFER_FREE counting_free
"#,
    )
    .unwrap();
    let mut compiler =
        Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()));
    compiler.args(["-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror"]);
    if cfg!(target_os = "macos") {
        compiler.arg("-I").arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/apple-threads"
        ));
    }
    let compiled = compiler
        .arg("-include")
        .arg(&hooks)
        .arg(&c)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "C compiler rejected the erased region:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let run = Command::new(&executable).output().unwrap();
    let _ = std::fs::remove_dir_all(&directory);
    assert!(run.status.success(), "{run:?}");
    let stderr = String::from_utf8_lossy(&run.stderr);
    let allocated: Vec<&str> = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("BUFFER_ALLOC "))
        .collect();
    let freed: Vec<&str> = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("BUFFER_FREE "))
        .collect();
    assert_eq!(allocated.len(), 1, "exactly the lent owner: {stderr}");
    let mut allocated_sorted = allocated.clone();
    let mut freed_sorted = freed.clone();
    allocated_sorted.sort_unstable();
    freed_sorted.sort_unstable();
    assert_eq!(
        allocated_sorted, freed_sorted,
        "every allocation is freed: {stderr}"
    );
}
