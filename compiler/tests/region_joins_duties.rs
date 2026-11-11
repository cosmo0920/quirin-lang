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

//! Acquisitions in the composed region body (card `REGION-JOINS-DUTIES`,
//! wave 29) — the remainder card `REGION-CREDIT-JOINS` named, and CT25's duty
//! half: *"Cleanup destroys owner after joins"*, expected **Accept**.
//!
//! The composed region body now admits the task route's **own** acquisition
//! and release — `let NAME = qcore_buffer_new(INT)?;`, `drop NAME;` — so the
//! sealed pack carries real cleanup duties (`CleanupAction::Destroy`) beside
//! the credit remainder, and the opening's `discharge` destroys the remaining
//! owners in reverse acquisition order, **after every join**. This
//! deliberately supersedes standing constraint 15m's duty-free premise; the
//! proposed amendment is in `docs/reports/region-joins-duties/README.md`.
//!
//! The emission story is the sequential route's precedent read at the text
//! level: an admitted region has one path, so it **erases into its
//! straight-line twin** — the split keeps the acquisitions at their own byte
//! offsets in the task half and writes one `drop NAME;` per remaining duty
//! after them, and the **unedited** scoped-task route compiles,
//! custody-checks, emits and reference-executes the result. Three checkers
//! run and none is edited: the scoped-task route, `check_flat_epoch` +
//! `check_epoch_credit_recovery`, and
//! `qcore_core::error_region::analyze_error_regions`.
//!
//! Every refusal test asserts the exact code, never `is_err()`.
use std::path::{Path, PathBuf};

use qcore_compiler::{
    CompileOptions, CompiledModule, ComposedEpochModule, SourceFile, compile_files,
};
use qcore_core::error_region::RegionOp;

/// The CT25 fixtures. They live **outside** `compiler/tests/fixtures/ct/` on
/// purpose: `ct_corpus.rs` fails closed when an unmapped case already owns a
/// fixture directory there, and this card does not edit the manifest. A CT
/// lane that maps the case moves this directory — the CT22/CT23 precedent.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/epoch_credit/CT25")
}

fn fixture(name: &str) -> SourceFile {
    let path = fixtures().join(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixture {} is readable: {error}", path.display()));
    SourceFile {
        path: format!("compiler/tests/fixtures/epoch_credit/CT25/{name}"),
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

/// The CT25 shape as one in-memory module with the region's eight lines
/// substituted. Every variant has the same line count and the same path, so
/// emitted C, source maps and reference observations are comparable byte for
/// byte with nothing excused by a filename.
fn with_region(region: &str) -> SourceFile {
    const SHAPE: &str = r#"#import std.concurrency;
concurrency checked;
error Failure { Failed; }
fn worker(value: i32) -> own<Buffer> effects { alloc, throws<AllocError> } {
    let buffer = qcore_buffer_new(value)?;
    return move buffer;
}
fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    thread scope plan epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope worker {
            copy;
            peak 1024;
            effort 10;
            credit 4096 effort 100;
            retained 64;
            join(drop);
        }
    }
    let value = 42;
    thread scope s {
        let handle = spawn s worker(copy value)?;
        let returned = join move handle?;
        let answer = qcore_buffer_read(returned);
        drop returned;
REGION
        return answer;
    }
}
"#;
    SourceFile {
        path: "region_joins_duties/shape.qcore".to_owned(),
        source: SHAPE.replace("REGION", region),
    }
}

const OWNED: &str = r#"        error region R {
            let owner = qcore_buffer_new(7)?;
            credit 5;
            effort 2;
            qcore_check(false, Failure);
        } catch (Failure) quant open R as pack {
            discharge pack;
        }"#;

/// The hand-written straight-line twin of [`OWNED`]: the acquisition at the
/// same byte offsets, the discharge's release written where the erasure has
/// room for it, every other byte of the statement blank and every line the
/// same width as the region line it replaces. Computed here by hand — this
/// constant is the test's independent oracle, not a readback of the split.
fn owned_twin() -> String {
    [
        " ".repeat(24),
        "            let owner = qcore_buffer_new(7)?;".to_owned(),
        format!("{:<21}", "drop owner;"),
        " ".repeat(21),
        " ".repeat(40),
        " ".repeat(48),
        " ".repeat(27),
        " ".repeat(9),
    ]
    .join("\n")
}

/// **CT25's duty half.** The plan grants the worker's credit, the parent
/// spawns and joins its one child, and only then does the parent's own region
/// acquire `owner`, credit 5, spend 2 and take its decided edge. The sealed
/// pack carries one real duty and remainder 3; the discharge destroys the
/// owner — after every join — and abandons exactly the parent-local credit.
/// Both published figures are cross-checked by the driver against the
/// reader's independent folds before publication.
#[test]
fn ct25_the_cleanup_destroys_the_owner_after_every_join() {
    let module = accept("case.qr");
    let evidence = composed(&module)
        .region
        .as_ref()
        .expect("the entry's region is published");
    assert_eq!(evidence.released, vec!["owner".to_owned()]);
    assert_eq!(evidence.abandoned_credit, 3);

    // The operations handed to the unedited model carry the real acquisition.
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

    // "After joins" is a fact of the checked task IR: the one creation is
    // joined, and both task facts precede the region statement.
    let ledger = &composed(&module).composed.ledger;
    assert_eq!(ledger.len(), 1);
    assert!(ledger[0].span.end < evidence.span.start);

    // The destruction is emitted: the erased task half carries the owner's
    // allocation and its release, compiled by the unedited task route.
    let c = module.c_source.as_deref().expect("the module emits");
    assert!(c.contains("qcore_buffer_new(INT32_C(7)"));
}

/// **The erasure, byte for byte, against a hand-written twin.** One module,
/// one path, one line count: the emitted C, the source map, the entry and the
/// reference observation of the region variant all equal the twin's — the
/// straight-line program the region *is*, written by hand at the same byte
/// offsets. The region buys checking, not code (the sequential route's
/// `an_admitted_region_emits_what_its_hand_written_twin_emits`, reached from
/// this route at the text level).
#[test]
fn the_region_erases_into_its_straight_line_twin_byte_for_byte() {
    let with = compile(with_region(OWNED)).expect("the region variant is admitted");
    let twin = compile(with_region(&owned_twin())).expect("the twin is admitted");
    assert_eq!(with.c_source, twin.c_source);
    assert_eq!(with.entry, twin.entry);
    assert_eq!(with.reference_observation, twin.reference_observation);
    // The source maps carry identical positions row for row. Exactly one row's
    // rendered `line_source` may differ: the synthesized release's position
    // lies inside the region statement's own extent, so the region variant
    // renders the original file's line there while the twin renders the
    // written `drop owner;` — the same attribution rule every erased span of
    // this route has (offsets always point at the original file).
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

/// A body release erases its duty — the model's frame rule, measured end to
/// end: the pack is duty-free again, the discharge destroys nothing, and only
/// the remainder is abandoned.
#[test]
fn a_body_release_erases_its_duty() {
    let module = accept("dropped_owner.qr");
    let evidence = composed(&module)
        .region
        .as_ref()
        .expect("the entry's region is published");
    assert_eq!(evidence.released, Vec::<String>::new());
    assert_eq!(evidence.abandoned_credit, 3);
}

/// Two owners are destroyed last-first when both duties survive to the pack —
/// here through the admitted sequenced spelling, where the pack's one
/// remaining duty is the second owner; and the simultaneous two-owner shape
/// is a **measured boundary**: the second acquisition's `?` failure edge
/// discards the first owner, which is the erased program's real leak path,
/// refused by the unedited task custody checker at its own `Q600`
/// (`REGION-JOINS-MULTI-OWNER`).
#[test]
fn duty_order_and_the_multi_owner_boundary() {
    let module = accept("sequenced_owners.qr");
    let evidence = composed(&module)
        .region
        .as_ref()
        .expect("the entry's region is published");
    assert_eq!(evidence.released, vec!["second".to_owned()]);

    assert_eq!(refuse("two_owners.qr"), vec!["Q600"]);
    assert!(
        message(fixture("two_owners.qr")).contains("discards"),
        "the refusal is the task route's own custody rule"
    );
}

/// Every pack obligation stays the unedited model's own, now reachable with a
/// **real duty** in the pack: a handler that walks away from a held owner is
/// `Q305` (CT21's shape with positive access permission), and a pack made
/// nonempty by a duty alone — zero credit — still requires its opening
/// (`Q306`).
#[test]
fn the_models_own_refusals_hold_with_a_real_duty() {
    assert_eq!(refuse("no_discharge_duty.qr"), vec!["Q305"]);
    assert_eq!(refuse("no_open_duty.qr"), vec!["Q306"]);
}

/// The duty reader's own shape refusals, each at `Q302` with the rule named:
/// a release of a resource the region did not acquire, an owner colliding
/// with an entry binding, and the sequential route's `qcore_acquire`
/// spelling, which names no task-route resource and stays refused.
#[test]
fn the_duty_reader_refuses_its_own_shapes() {
    let refused = refuse("outer_drop.qr");
    assert_eq!(refused, vec!["Q302"]);
    assert!(message(fixture("outer_drop.qr")).contains("REGION-OUTER-LIVE-RESOURCES"));

    assert_eq!(refuse("colliding_owner.qr"), vec!["Q302"]);
    assert!(message(fixture("colliding_owner.qr")).contains("collides with a binding"));

    // The sequential spelling stays refused, with the admitted set named.
    let acquiring = with_region(
        r#"        error region R {
            let outer = qcore_acquire(1, true, Failure);
            credit 5;
            qcore_check(false, Failure);
        } catch (Failure) quant open R as pack {
            discharge pack;
        }"#,
    );
    let refused = compile_files(std::slice::from_ref(&acquiring), &CompileOptions::default())
        .expect_err("the sequential spelling is refused here");
    assert_eq!(refused[0].code, "Q302");
    assert!(refused[0].message.contains("qcore_buffer_new"));
}

/// The "after joins" rule holds unchanged with a duty in the region: a join
/// after a duty-carrying region is refused at `Q600` in exactly the wave-28
/// way.
#[test]
fn after_joins_is_still_refused_with_a_duty() {
    let mut file = with_region(OWNED);
    // Move the join below the region: swap the join/read/drop block with the
    // region statement.
    let source = &mut file.source;
    let join_start = source.find("        let returned").expect("join line");
    let region_start = source.find("        error region R {").expect("region");
    let tail_start = source.find("        return answer;").expect("return");
    let joins = source[join_start..region_start].trim_end().to_owned();
    let region = source[region_start..tail_start].trim_end().to_owned();
    source.replace_range(join_start..tail_start, &format!("{region}\n{joins}\n"));
    let refused = compile_files(&[file], &CompileOptions::default())
        .expect_err("a join after the duty region is refused");
    assert_eq!(
        refused
            .into_iter()
            .map(|diagnostic| diagnostic.code.to_string())
            .collect::<Vec<_>>(),
        vec!["Q600"]
    );
}

/// The epoch's certificates are unmoved by a duty-carrying region: the bound
/// and the credit-recovery split are byte-identical between the owner region
/// and the duty-free wave-28 shape — the parent's owner is outside the
/// epoch's allocation exactly as its strict credit is
/// (`REGION-JOINS-EPOCH-AXIS` unchanged).
#[test]
fn the_epoch_certificates_are_unmoved_by_a_duty() {
    let owned = compile(with_region(OWNED)).expect("the owner region is admitted");
    let duty_free = compile(with_region(
        r#"        error region R {
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
        composed(&owned).certificate,
        composed(&duty_free).certificate
    );
    assert_eq!(
        composed(&owned).credit_recovery,
        composed(&duty_free).credit_recovery
    );
    // The duty-free region publishes wave-28 evidence unchanged: no owner
    // released, and the same abandoned credit.
    let free = composed(&duty_free).region.as_ref().expect("published");
    assert_eq!(free.released, Vec::<String>::new());
    assert_eq!(free.abandoned_credit, 3);
}

/// Native positive control (the family's `owned_source_tasks` pattern): the
/// CT25 case's emitted C, compiled with the platform C compiler and run with
/// counting allocator hooks, frees **every** buffer it allocates — the
/// worker's result and the region's owner — and exits successfully. A
/// discharge that lost the owner's duty would surface here as an unfreed
/// allocation, not only as a disagreement.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn the_native_run_frees_every_owner() {
    use std::process::Command;

    let module = accept("case.qr");
    let directory = std::env::temp_dir().join(format!(
        "qcore-region-duties-{}-{}",
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
    assert_eq!(
        allocated.len(),
        2,
        "worker result and region owner: {stderr}"
    );
    let mut allocated_sorted = allocated.clone();
    let mut freed_sorted = freed.clone();
    allocated_sorted.sort_unstable();
    freed_sorted.sort_unstable();
    assert_eq!(
        allocated_sorted, freed_sorted,
        "every allocation is freed: {stderr}"
    );
}
