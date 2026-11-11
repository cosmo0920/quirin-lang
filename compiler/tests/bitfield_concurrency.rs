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

//! Source-to-concurrency coverage for adjacent C bit-fields.
//!
//! These are deliberately outside `fixtures/ct/CT10`: the CT harness forbids a
//! fixture directory while the manifest row remains unmapped. The integrator
//! can map CT10 only after reconciling this card's evidence with the ledger.

use qcore_compiler::{
    BitfieldTaskAssumption, CheckedSourceModule, CompileOptions, SourceExecutionProfile,
    SourceFile, compile_files,
};
use qcore_core::disjoint_footprint::{
    DerivedFootprint, DisjointnessJudgement, FootprintAssumption, FootprintExport,
};

const ADJACENT: &str = include_str!("fixtures/bitfield_concurrency/adjacent.qr");
const ADDRESSABLE: &str = include_str!("fixtures/bitfield_concurrency/addressable.qr");
fn addressable_fixture() -> String {
    ADDRESSABLE.replace("\r\n", "\n")
}
const DISTINCT_UNITS: &str = include_str!("fixtures/bitfield_concurrency/distinct_units.qr");

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("bitfield_concurrency.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

#[test]
fn adjacent_bitfields_cannot_be_written_by_independent_workers() {
    let diagnostics = compile(ADJACENT).expect_err("one memory location cannot be split");
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        ["Q661"]
    );
    assert_eq!(
        diagnostics[0].pass,
        qcore_frontend::frontend::SemanticPass::Concurrency
    );
    assert!(diagnostics[0].message.contains("SharedBitfieldUnit"));
}

#[test]
fn refusal_is_symmetric_in_the_two_worker_captures() {
    let reversed = ADJACENT
        .replace("writer(write flags.left)?;", "writer(write flags.__swap)?;")
        .replace("writer(write flags.right)?;", "writer(write flags.left)?;")
        .replace(
            "writer(write flags.__swap)?;",
            "writer(write flags.right)?;",
        );
    let diagnostics = compile(&reversed).expect_err("operand order changes no storage unit");
    assert_eq!(diagnostics[0].code, "Q661");
}

fn checked(source: &str) -> qcore_compiler::CompiledModule {
    compile(source).unwrap_or_else(|diagnostics| panic!("expected acceptance, got {diagnostics:?}"))
}

#[test]
fn ordinary_disjoint_fields_admit_independent_workers() {
    let module = checked(ADDRESSABLE);
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::BitfieldConcurrencyV1
    );
    assert!(
        module.c_source.is_none(),
        "the route is check-only at this card"
    );
    assert_eq!(
        module.checked.function_count(),
        0,
        "a footprint property is not function-check coverage"
    );
    assert!(
        module.checked.as_scoped_tasks().is_none(),
        "the property certificate cannot be exported as a scoped-task certificate"
    );
    let CheckedSourceModule::BitfieldTasks(checked) = &module.checked else {
        panic!("expected bit-field task evidence");
    };
    assert_eq!(checked.lowered.writes.len(), 2);
    assert_eq!(checked.certificate.admitted_disjointness.len(), 1);
    assert!(matches!(
        checked.certificate.admitted_disjointness[0],
        DisjointnessJudgement::Memory { .. }
    ));
    assert!(
        checked
            .certificate
            .assumptions
            .contains(&FootprintAssumption::DeclaredRecordLayout)
    );
    assert_eq!(
        checked.assumptions,
        std::collections::BTreeSet::from([BitfieldTaskAssumption::SpawnAndJoinSucceed])
    );
}

#[test]
fn distinct_bitfield_units_are_an_admitting_attribution_control() {
    let module = checked(DISTINCT_UNITS);
    let CheckedSourceModule::BitfieldTasks(checked) = &module.checked else {
        panic!("expected bit-field task evidence");
    };
    let exports = checked
        .certificate
        .derived_footprints
        .values()
        .filter_map(|footprint| match footprint {
            DerivedFootprint::Memory { unit: Some(_), .. } => Some(footprint.export()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        exports,
        [
            FootprintExport::SharedMemoryLocation { unit: 0 },
            FootprintExport::SharedMemoryLocation { unit: 1 },
        ]
    );
    assert_eq!(checked.certificate.admitted_disjointness.len(), 1);
}

#[test]
fn byte_encoding_is_not_used_in_either_direction() {
    // Same unit, disjoint declared bit ranges, and the same byte extent: still
    // refused by unit identity rather than byte overlap.
    assert_eq!(compile(ADJACENT).unwrap_err()[0].code, "Q661");
    // Addressable fields in distinct bytes are admitted without inventing a
    // bit-field unit. This is the converse control from CT10-BYTE-ENCODING-UNSOUND.
    assert!(compile(ADDRESSABLE).is_ok());
}

#[test]
fn missing_concurrency_import_is_refused_before_route_admission() {
    let source = ADJACENT
        .replace("\r\n", "\n")
        .replace("#import std.concurrency;\n", "");
    let diagnostics = compile(&source).expect_err("concurrency stays opt-in");
    assert_eq!(diagnostics[0].code, "Q015");
}

#[test]
fn a_malformed_selected_layout_fails_closed() {
    let source = addressable_fixture().replace("field<4, 4>", "field<4>");
    let diagnostics = compile(&source).expect_err("selected syntax is total-or-reject");
    assert_eq!(diagnostics[0].code, "Q600");
    assert!(diagnostics[0].message.contains("layout fields must be"));
}

#[test]
fn a_worker_that_does_not_write_is_refused() {
    let source = addressable_fixture().replace("    write value;\n", "");
    let diagnostics = compile(&source).expect_err("capture attribution cannot replace a write");
    assert_eq!(diagnostics[0].code, "Q600");
    assert!(
        diagnostics[0]
            .message
            .contains("exactly `write PARAM; return;`")
    );
}

#[test]
fn an_unjoined_writer_is_refused() {
    let source = addressable_fixture().replace("        let right_done = join move right?;\n", "");
    let diagnostics = compile(&source).expect_err("the write authority must return at join");
    assert_eq!(diagnostics[0].code, "Q600");
    assert!(diagnostics[0].message.contains("join each handle once"));
}

#[test]
fn sequentialized_writers_do_not_count_as_concurrent() {
    let source = addressable_fixture().replace(
        "        let right = spawn workers writer(write pair.right)?;\n        let left_done = join move left?;",
        "        let left_done = join move left?;\n        let right = spawn workers writer(write pair.right)?;",
    );
    let diagnostics = compile(&source).expect_err("second spawn is after the first join");
    assert_eq!(diagnostics[0].code, "Q600");
    assert!(
        diagnostics[0]
            .message
            .contains("spawned before any is joined")
    );
}

#[test]
fn an_extra_unsafe_statement_is_not_silently_ignored() {
    let source = addressable_fixture().replace(
        "    thread scope workers {",
        "    unsafe { return; }\n    thread scope workers {",
    );
    let diagnostics = compile(&source).expect_err("bounded route rejects unmodelled statements");
    assert_eq!(diagnostics[0].code, "Q600");
    assert!(diagnostics[0].message.contains("bounded entry is exactly"));
}

#[test]
fn the_object_must_be_declared_before_the_writing_scope() {
    let source = addressable_fixture()
        .replace("    let pair = Pair;\n", "")
        .replace(
            "    }\n    return;\n}\n",
            "    }\n    let pair = Pair;\n    return;\n}\n",
        );
    let diagnostics = compile(&source).expect_err("use before declaration stays refused");
    assert_eq!(diagnostics[0].code, "Q013");
}

#[test]
fn an_unrelated_declaration_is_not_silently_ignored() {
    let source =
        addressable_fixture().replace("fn main()", "fn idle() -> unit { return; }\n\nfn main()");
    let diagnostics = compile(&source).expect_err("bounded profile owns every declaration");
    assert_eq!(diagnostics[0].code, "Q600");
    assert!(diagnostics[0].message.contains("unrelated declaration"));
}

#[test]
fn the_entry_cannot_hide_unmodelled_parameters() {
    let source = addressable_fixture().replace("fn main()", "fn main(hidden: i32)");
    let diagnostics = compile(&source).expect_err("entry parameters are outside the grammar");
    assert_eq!(diagnostics[0].code, "Q600");
    assert!(diagnostics[0].message.contains("takes no parameters"));
}

#[test]
fn duplicate_write_captures_are_refused() {
    let source = addressable_fixture().replace(
        "writer(write pair.left)?;",
        "writer(write pair.left, write pair.left)?;",
    );
    let diagnostics = compile(&source).expect_err("one spawn cannot duplicate write authority");
    assert_eq!(diagnostics[0].code, "Q013");
}

#[test]
fn duplicate_joins_are_refused() {
    let source = addressable_fixture().replace("join move right?;", "join move left?;");
    let diagnostics = compile(&source).expect_err("one handle cannot satisfy both joins");
    assert_eq!(diagnostics[0].code, "Q600");
    assert!(diagnostics[0].message.contains("join each handle once"));
}

#[test]
fn a_spawn_binding_cannot_shadow_the_layout_object() {
    let source = addressable_fixture().replace(
        "let left = spawn workers writer(write pair.left)?;",
        "let pair = spawn workers writer(write pair.left)?;",
    );
    let diagnostics = compile(&source).expect_err("the second capture resolves to a task handle");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "Q013")
            || diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "Q600"),
        "shadowing must fail closed: {diagnostics:?}"
    );
}

#[test]
fn inconsistent_declared_layout_arithmetic_is_refused_by_the_core_checker() {
    let source = ADJACENT.replace("bits<0, 3, 5, 0, 4>", "bits<0, 31, 5, 0, 4>");
    let diagnostics = compile(&source).expect_err("bit extent exceeds its declared storage");
    assert_eq!(diagnostics[0].code, "Q657");
    assert!(
        diagnostics[0]
            .message
            .contains("BitfieldBitsOutsideStorage")
    );
}

#[test]
fn propagated_spawn_failure_is_an_explicit_unproved_premise() {
    let module = checked(ADDRESSABLE);
    let CheckedSourceModule::BitfieldTasks(checked) = &module.checked else {
        panic!("expected property-only bit-field evidence");
    };
    assert!(
        checked
            .assumptions
            .contains(&BitfieldTaskAssumption::SpawnAndJoinSucceed)
    );
    assert!(module.c_source.is_none());
    assert!(module.reference_observation.is_none());
}

#[test]
fn a_layout_word_without_a_layout_constructor_does_not_select_this_route() {
    let source = addressable_fixture()
        .replace("left: field<0, 4>", "left: field")
        .replace("right: field<4, 4>", "right: field");
    let diagnostics = compile(&source).expect_err("ordinary record typing owns this source");
    assert!(diagnostics.iter().all(|diagnostic| {
        !diagnostic.message.contains("layout fields must be")
            && !diagnostic
                .message
                .contains("source.bitfield-concurrency.v1")
    }));
}
