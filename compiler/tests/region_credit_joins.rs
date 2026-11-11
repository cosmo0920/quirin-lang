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

//! End-to-end evidence for the composition of a flat-epoch credit plan with
//! the quantitative error region (card `REGION-CREDIT-JOINS`, wave 28).
//!
//! This is the seam two wave-27 reports named from opposite sides —
//! `CT23-T07-SEAM` (`docs/reports/ct23-credit-error-path/README.md` §9) and
//! `REGION-CREDIT-WITH-JOINS` (`docs/reports/quant-region-source/README.md` §5)
//! — landed as the minimal honest composition: a `source.epoch.tasks.v1` entry
//! whose task scope carries, **after every spawn and join**, one
//! `error region … catch (E) quant open … { discharge …; }` whose body holds
//! the parent's own quantities. Three checkers run and **none is edited**: the
//! scoped-task route, `check_flat_epoch` + `check_epoch_credit_recovery`, and
//! `qcore_core::error_region::analyze_error_regions`.
//!
//! What this file measures is CT22's scenario — *"Quant opening discards
//! parent-local unused credit after joins"*, expected **Accept** — and the
//! refusals that bound it. Every refusal test asserts the **exact ordered code
//! vector**, never `is_err()`.
use std::path::{Path, PathBuf};

use qcore_compiler::{
    CompileOptions, CompiledModule, ComposedEpochModule, SourceFile, compile_files,
};
use qcore_core::error_region::RegionOp;
use qcore_core::resource_flow::Quantity;

/// The CT22 fixtures. Card `REGION-CREDIT-JOINS` parked them under
/// `tests/fixtures/epoch_credit/CT22/` because `ct_corpus.rs` fails closed
/// when an unmapped case owns a directory under `fixtures/ct/`; the wave-29
/// CT lane (card `CT22-WIRE`) mapped the case and moved the directory here —
/// the T08-EPOCH-SRC and CT23 precedent. `ct_corpus.rs` now runs the case and
/// its controls too; this file remains the capability card's own end-to-end
/// evidence on the same bytes.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ct/CT22")
}

fn fixture(name: &str) -> SourceFile {
    let path = fixtures().join(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixture {} is readable: {error}", path.display()));
    SourceFile {
        path: format!("compiler/tests/fixtures/ct/CT22/{name}"),
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

/// The CT22 shape as one in-memory module with the region's seven lines
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
fn main() -> i32 effects { throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
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
        path: "region_credit_joins/shape.qcore".to_owned(),
        source: SHAPE.replace("REGION", region),
    }
}

const DISCHARGED: &str = r#"        error region R {
            credit 5;
            effort 2;
            qcore_check(false, Failure);
        } catch (Failure) quant open R as pack {
            discharge pack;
        }"#;

const BLANK: &str = "\n\n\n\n\n\n";

fn both(memory: u64, work: u64) -> Quantity {
    Quantity::from_axes([("memory".to_owned(), memory), ("effort".to_owned(), work)])
        .expect("two distinct axes never overflow")
}

/// **The CT22 case.** The plan grants the worker's credit, the parent spawns
/// and joins its one child, and only then does the parent's own region credit
/// 5, spend 2 and take its decided edge. The opening binds the sealed pack and
/// the discharge abandons exactly the parent-local remainder 3 — an explicit,
/// published quantity, cross-checked by the driver against an independent fold
/// before publication.
#[test]
fn ct22_the_opening_discards_the_parents_unused_credit_after_all_joins() {
    let module = accept("case.qr");
    let evidence = composed(&module)
        .region
        .as_ref()
        .expect("the entry's region is published");
    assert_eq!(evidence.abandoned_credit, 3);

    // The operations handed to the unedited model are exactly the written
    // statements: the parent's quantities, the decided edge, one clause that
    // opens this region's pack, one discharge.
    let [
        RegionOp::ErrorRegion {
            region,
            body,
            catches,
        },
    ] = evidence.operations.as_slice()
    else {
        panic!("one region operation is published");
    };
    assert_eq!(region, "R");
    assert_eq!(
        body.as_slice(),
        [
            RegionOp::AddStrictCredit(5),
            RegionOp::SpendStrictCredit(2),
            RegionOp::Raise("Failure".to_owned()),
        ]
    );
    let [clause] = catches.as_slice() else {
        panic!("one catch clause");
    };
    assert_eq!(clause.error, "Failure");
    let open = clause.quant_open.as_ref().expect("the clause opens a pack");
    assert_eq!((open.region.as_str(), open.binding.as_str()), ("R", "pack"));
    assert_eq!(
        clause.handler.as_slice(),
        [RegionOp::Discharge("pack".to_owned())]
    );

    // The pack the opening binds carries **no duty**: the body admits no
    // acquisition, so the one thing the discharge can discard is the credit.
    assert!(
        body.iter()
            .all(|op| !matches!(op, RegionOp::Acquire { .. })),
        "the composed region body holds quantities only"
    );

    // "After joins" is a fact of the checked task IR, not a reading of the
    // fixture: the one creation is joined, and both task facts precede the
    // region statement.
    let ledger = &composed(&module).composed.ledger;
    assert_eq!(ledger.len(), 1);
    assert!(ledger[0].span.end < evidence.span.start);
}

/// The boundary in the other direction: a parent that spends its whole credit
/// abandons nothing, and the published figure says so — the quantity is a real
/// fold of the written statements, not a flag.
#[test]
fn the_boundary_spends_everything_and_abandons_nothing() {
    let module = accept("spent_out.qr");
    let evidence = composed(&module)
        .region
        .as_ref()
        .expect("the entry's region is published");
    assert_eq!(evidence.abandoned_credit, 0);
}

/// **Nothing is relaxed and nothing is re-attributed.** The epoch bound and the
/// credit-recovery split a module publishes are byte-identical with and without
/// the region, and a module without a region publishes no region evidence.
#[test]
fn the_epoch_certificates_are_unmoved_by_the_region() {
    let with = accept("case.qr");
    let without = accept("no_region.qr");
    assert_eq!(composed(&with).certificate, composed(&without).certificate);
    assert_eq!(
        composed(&with).credit_recovery,
        composed(&without).credit_recovery
    );
    assert!(composed(&without).region.is_none());
}

/// **The erasure, byte for byte.** One module, one path, one line count: the
/// emitted C, the source map, the entry and the reference observation are all
/// equal with the region written and with its seven lines blank. The region
/// buys checking, not code (constraint 15l reached from a second route).
#[test]
fn the_emitted_c_is_unchanged_by_the_region_byte_for_byte() {
    let with = compile(with_region(DISCHARGED)).expect("the region variant is admitted");
    let without = compile(with_region(BLANK)).expect("the blank variant is admitted");
    assert_eq!(with.c_source, without.c_source);
    assert_eq!(with.source_map, without.source_map);
    assert_eq!(with.entry, without.entry);
    assert_eq!(with.reference_observation, without.reference_observation);
}

/// The "after joins" half of the scenario, refused when false — in both
/// directions, and derived from the checked task IR rather than read from the
/// plan. `Q600` is the composed route's own reconciliation band.
#[test]
fn a_spawn_or_join_after_the_region_is_refused() {
    assert_eq!(refuse("before_join.qr"), vec!["Q600"]);
    assert!(
        message(fixture("before_join.qr")).contains("REGION-CREDIT-JOINS"),
        "the refusal names the rule"
    );

    // The spawn direction: the region written before the creation itself.
    let spawn_after = with_region(
        r#"        error region R {
            credit 5;
            effort 2;
            qcore_check(false, Failure);
        } catch (Failure) quant open R as pack {
            discharge pack;
        }"#,
    );
    let mut source = spawn_after.source;
    // Move the region above the spawn by swapping the two blocks of lines.
    let region_start = source.find("        error region R {").expect("region");
    let spawn_start = source
        .find("        let handle = spawn")
        .expect("spawn line");
    let region_text = source[region_start..source.find("        return answer;").unwrap()]
        .trim_end()
        .to_owned();
    let spawn_text = source[spawn_start..region_start].trim_end().to_owned();
    source.replace_range(
        spawn_start..source.find("        return answer;").unwrap(),
        &format!("{region_text}\n{spawn_text}\n"),
    );
    let refused = compile_files(
        &[SourceFile {
            path: "region_credit_joins/spawn_after.qr".to_owned(),
            source,
        }],
        &CompileOptions::default(),
    )
    .expect_err("a spawn after the region is refused");
    assert_eq!(
        refused
            .into_iter()
            .map(|diagnostic| diagnostic.code.to_string())
            .collect::<Vec<_>>(),
        vec!["Q600"]
    );
}

/// Every pack obligation is the unedited model's own, at the code that pass
/// already allocates — the composition re-implements no rule and re-attributes
/// no refusal (constraint 15l).
#[test]
fn the_models_own_refusals_arrive_at_their_own_codes() {
    // CT21's shape from this route: the handler walks away with the pack live.
    assert_eq!(refuse("no_discharge.qr"), vec!["Q305"]);
    // A nonempty pack caught without being opened.
    assert_eq!(refuse("no_open.qr"), vec!["Q306"]);
    // The opening names the wrong region.
    assert_eq!(refuse("wrong_region.qr"), vec!["Q303"]);

    // A second discharge consumes one capability twice.
    let doubled = with_region(
        r#"        error region R {
            credit 5;
            effort 2;
            qcore_check(false, Failure);
        } catch (Failure) quant open R as pack {
            discharge pack; discharge pack;
        }"#,
    );
    let refused = compile_files(&[doubled], &CompileOptions::default())
        .expect_err("a doubled discharge is refused");
    assert_eq!(refused[0].code, "Q304");

    // Overspending refuses with the model's own words; the quantity never
    // wraps.
    let overspent = with_region(
        r#"        error region R {
            credit 5;
            effort 7;
            qcore_check(false, Failure);
        } catch (Failure) quant open R as pack {
            discharge pack;
        }"#,
    );
    let refused = compile_files(&[overspent], &CompileOptions::default())
        .expect_err("an overspend is refused");
    assert_eq!(refused[0].code, "Q302");
    assert!(refused[0].message.contains("insufficient strict credit"));
}

/// The composed reader's own shape refusals, each at `Q302` (the `ErrorRegion`
/// pass's generic presentation code, reached from a second route) except the
/// undeclared error type, which is the typed-error band's own `Q300`.
#[test]
fn the_composed_reader_refuses_its_own_shapes() {
    // A region acquisition has no task-route resource to hold.
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
        .expect_err("an acquiring region is refused here");
    assert_eq!(refused[0].code, "Q302");
    assert!(refused[0].message.contains("REGION-JOINS-DUTIES"));

    // A check decided to pass has no edge to model.
    let passing = with_region(
        r#"        error region R {
            credit 5;
            effort 5;
            qcore_check(true, Failure);
        } catch (Failure) quant open R as pack {
            discharge pack;
        }"#,
    );
    let refused = compile_files(&[passing], &CompileOptions::default())
        .expect_err("a passing check is refused");
    assert_eq!(refused[0].code, "Q302");

    // A name that is not a declared error type is the typed-error band's own
    // condition. (`value` resolves — it is the entry's own binding — so name
    // resolution admits it and the region reader is the gate that refuses it;
    // a name that resolves nowhere is the existing `Q013` before any of this
    // runs, exactly as everywhere else.)
    let undeclared = with_region(
        r#"        error region R {
            credit 5;
            effort 2;
            qcore_check(false, value);
        } catch (Failure) quant open R as pack {
            discharge pack;
        }"#,
    );
    let refused = compile_files(&[undeclared], &CompileOptions::default())
        .expect_err("an undeclared error is refused");
    assert_eq!(refused[0].code, "Q300");

    // A region in a worker: a spawned worker is a first-order leaf function.
    let mut in_worker = acquiring;
    in_worker.source = with_region(BLANK).source.replace(
        "    let buffer = qcore_buffer_new(value)?;",
        "    error region W {\n        credit 1;\n        qcore_check(false, Failure);\n    } \
         catch (Failure) quant open W as pack {\n        discharge pack;\n    }\n    let buffer \
         = qcore_buffer_new(value)?;",
    );
    let refused = compile_files(&[in_worker], &CompileOptions::default())
        .expect_err("a worker region is refused");
    assert_eq!(refused[0].code, "Q302");
    assert!(refused[0].message.contains("first-order leaf"));

    // A second region in the entry.
    let doubled = with_region(&format!("{DISCHARGED}\n{DISCHARGED}"));
    let refused = compile_files(&[doubled], &CompileOptions::default())
        .expect_err("a second region is refused");
    assert_eq!(refused[0].code, "Q302");
    assert!(refused[0].message.contains("REGION-MULTIPLE"));
}

/// Both compositions in one module: the epoch's credit-recovery split over the
/// creator's derived error edges and the parent's own region epilogue. Two
/// unedited models, two published quantities, one entry.
#[test]
fn both_models_fire_in_one_module() {
    let module = accept("both_models.qr");
    let evidence = composed(&module);
    // The CT23 half: `error(move);` restores the whole grant on the creator's
    // derived error edges.
    assert_eq!(evidence.credit_recovery.restored_credits, both(4096, 100));
    assert_eq!(evidence.credit_recovery.spent_credits, Quantity::zero());
    // The CT22 half: the parent-local remainder is abandoned by the opening on
    // the success path.
    assert_eq!(
        evidence
            .region
            .as_ref()
            .expect("the region is published")
            .abandoned_credit,
        3
    );
}

/// The route boundary in the other direction, measured rather than assumed: a
/// **pure** epoch module (declarative workers, no task region) that writes a
/// region keeps a refusal — the declarative route gains no error path from
/// this card, exactly as constraint 15j records it gained none from CT23.
#[test]
fn the_pure_epoch_route_keeps_its_refusal() {
    let source = r#"#import std.concurrency;
error Failure { Failed; }
fn main() -> i32 {
    thread scope plan epoch {
        bound 65536 effort 1000;
        overhead scope 512 thread 128;
        thread scope worker {
            peak 1024;
            effort 10;
            credit 4096 effort 100;
        }
    }
    error region R {
        credit 5;
        effort 2;
        qcore_check(false, Failure);
    } catch (Failure) quant open R as pack {
        discharge pack;
    }
    return 0;
}
"#;
    let refused = compile_files(
        &[SourceFile {
            path: "region_credit_joins/pure_epoch.qcore".to_owned(),
            source: source.to_owned(),
        }],
        &CompileOptions::default(),
    )
    .expect_err("the pure epoch route refuses a region");
    assert_eq!(refused[0].code, "Q600");
}
