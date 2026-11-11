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

//! Card `P04-COMPOSE-MEMCLOSE`: the mutation matrix for the memory-close
//! composition.
//!
//! Every forged reading is asserted to actually differ from the honest one
//! **before** it is submitted, so no row passes vacuously, and every row count
//! is pinned so a row cannot silently vanish.
//!
//! The decisive new class is the **derived-text** group: an honest recipe over
//! substituted base text, and a substituted recipe over honest text, are both
//! refused — and the two directions are refused at different layers, which the
//! rows record rather than blur.

#[path = "fixtures/memory_close_composition.rs"]
mod fixture;
use fixture::*;

use qcore_schemas::composition_summary::{
    CalleeIdentityRecord, CompositionProposalRecord, MemoryCloseCalleeRecord,
    MemoryCloseCompositionRecord, MemoryClosePrerequisites,
};
use qcore_schemas::{SemanticWitness, call_use_summary::PrerequisiteIdentity};

/// The honest reading, plus a mutated copy asserted to really differ.
fn mutated(
    task: &qcore_verification::composition_source::MemoryCloseCompositionSourceTask,
    label: &str,
    change: impl FnOnce(&mut MemoryCloseCompositionRecord),
) -> SemanticWitness {
    let honest = core_record(task);
    let mut forged = honest.clone();
    change(&mut forged);
    assert_ne!(
        forged, honest,
        "the `{label}` mutation must really differ from the honest reading"
    );
    witness_of(task, forged)
}

#[test]
fn every_figure_of_a_proposed_reading_is_refused() {
    if !clang_available() {
        return;
    }
    let task = task();
    assert!(
        accept(&task, &witness_for(&task)).is_ok(),
        "the honest reading is admitted"
    );

    type Row = (&'static str, fn(&mut MemoryCloseCompositionRecord));
    let rows: Vec<Row> = vec![
        ("function", |r| r.function = "cio_chunk_open".into()),
        ("revision", |r| r.source_revision = "0".repeat(40)),
        ("signature", |r| {
            r.signature = "void (struct cio_chunk *)".into()
        }),
        ("memory kind inflated", |r| r.memory_kind += 1),
        ("memory kind zeroed", |r| r.memory_kind = 0),
        ("cases emptied", |r| r.cases.clear()),
        ("a case dropped", |r| {
            r.cases.pop();
        }),
        ("a case duplicated", |r| {
            let first = r.cases[0].clone();
            r.cases.push(first);
        }),
        ("cases reordered", |r| r.cases.reverse()),
        ("the null entry claimed nonnull", |r| {
            r.cases[0].nonnull = true
        }),
        ("the nonnull entry claimed null", |r| {
            r.cases[1].nonnull = false
        }),
        ("a selected path retargeted", |r| {
            r.cases[1].source_path += 1
        }),
        ("a call count inflated", |r| r.cases[1].calls += 1),
        ("a call count erased", |r| r.cases[1].calls = 0),
        ("backend discharge claimed on the null entry", |r| {
            r.cases[0].backend_discharged = true
        }),
        ("backend discharge erased", |r| {
            r.cases[1].backend_discharged = false
        }),
        ("primary unlink erased", |r| {
            r.cases[1].primary_unlinked = false
        }),
        ("state unlink erased", |r| r.cases[1].state_unlinked = false),
        ("name release erased", |r| r.cases[1].name_released = false),
        ("chunk release erased", |r| {
            r.cases[1].chunk_released = false
        }),
        ("counter decrement erased", |r| {
            r.cases[1].counter_decremented = false
        }),
        ("counter decrement claimed on the null entry", |r| {
            r.cases[0].counter_decremented = true
        }),
        ("checked paths emptied", |r| r.checked_paths.clear()),
        ("checked paths widened", |r| r.checked_paths.push(99)),
        ("checked paths reordered", |r| r.checked_paths.reverse()),
        ("uncovered paths emptied", |r| r.uncovered_paths.clear()),
        ("uncovered paths widened", |r| r.uncovered_paths.push(99)),
        ("uncovered paths claimed checked", |r| {
            r.uncovered_paths = r.checked_paths.clone()
        }),
        ("profile assumptions invented", |r| {
            r.profile_assumptions
                .insert("invented".into(), "true".into());
        }),
    ];
    assert_eq!(rows.len(), 29, "twenty-nine proposed-figure rows");
    for (label, change) in rows {
        let witness = mutated(&task, label, change);
        assert!(
            accept(&task, &witness).is_err(),
            "the `{label}` mutation was admitted"
        );
    }
}

/// The proposal is tagged by family, so one family's reading can never be read
/// as another's — and a leaf-family proposal against a memory-close task is
/// refused before any figure is compared.
#[test]
fn another_familys_proposal_and_another_witness_kind_are_refused() {
    if !clang_available() {
        return;
    }
    let task = task();
    let honest = core_record(&task);
    let foreign = SemanticWitness::Composition(Box::new(
        qcore_schemas::composition_summary::CompositionWitness {
            task_digest: task.task().task_digest(),
            summary: CompositionProposalRecord::UnmapWrapper(
                qcore_schemas::composition_summary::CompositionRecord {
                    function: honest.function.clone(),
                    source_revision: honest.source_revision.clone(),
                    signature: honest.signature.clone(),
                    cases: Vec::new(),
                    profile_assumptions: honest.profile_assumptions.clone(),
                },
            ),
        },
    ));
    assert!(
        accept(&task, &foreign).is_err(),
        "a leaf-family proposal is refused"
    );

    // A witness bound to another task digest.
    let rebound = SemanticWitness::Composition(Box::new(
        qcore_schemas::composition_summary::CompositionWitness {
            task_digest: "0".repeat(64),
            summary: CompositionProposalRecord::MemoryClose(honest.clone()),
        },
    ));
    assert!(
        accept(&task, &rebound).is_err(),
        "a rebound witness is refused"
    );

    // Another witness kind entirely.
    let other =
        SemanticWitness::ContractObservation(qcore_schemas::witness::ContractObservationWitness {
            task_digest: task.task().task_digest(),
            runs: Vec::new(),
        });
    assert!(
        accept(&task, &other).is_err(),
        "another witness kind is refused"
    );
}

/// The four prerequisite identities, forged every way the set allows.
#[test]
fn prerequisite_forgeries_are_refused() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites(SOURCE);
    let honest = identities(&tasks);
    // A foreign identity: another counter body's own task, over the same unit.
    let foreign = counter_task(SOURCE, "cio_chunk_counter_total_up_sub", COUNTER_CONTRACT)
        .prerequisite_identity();
    assert_ne!(foreign, honest.counter);

    let empty = PrerequisiteIdentity {
        semantic_profile: String::new(),
        task_digest: String::new(),
        obligation_id: String::new(),
    };

    let mut rows: Vec<(String, MemoryClosePrerequisites)> = Vec::new();
    // Each of the four emptied.
    for (label, slot) in [
        ("reset emptied", 0usize),
        ("backend emptied", 1),
        ("unlink emptied", 2),
        ("counter emptied", 3),
    ] {
        let mut set = honest.clone();
        *[
            &mut set.reset,
            &mut set.backend,
            &mut set.unlink,
            &mut set.counter,
        ][slot] = empty.clone();
        rows.push((label.into(), set));
    }
    // Two repeated, in two different pairs.
    for (label, from, to) in [
        ("reset repeated as backend", 0usize, 1usize),
        ("counter repeated as unlink", 3, 2),
    ] {
        let mut set = honest.clone();
        let source = [
            set.reset.clone(),
            set.backend.clone(),
            set.unlink.clone(),
            set.counter.clone(),
        ][from]
            .clone();
        *[
            &mut set.reset,
            &mut set.backend,
            &mut set.unlink,
            &mut set.counter,
        ][to] = source;
        rows.push((label.into(), set));
    }
    // A foreign identity in each of the four slots.
    for (label, slot) in [
        ("foreign reset", 0usize),
        ("foreign backend", 1),
        ("foreign unlink", 2),
        ("foreign counter", 3),
    ] {
        let mut set = honest.clone();
        *[
            &mut set.reset,
            &mut set.backend,
            &mut set.unlink,
            &mut set.counter,
        ][slot] = foreign.clone();
        rows.push((label.into(), set));
    }
    // Two real identities exchanged.
    {
        let mut set = honest.clone();
        std::mem::swap(&mut set.unlink, &mut set.counter);
        rows.push(("two real identities exchanged".into(), set));
    }
    assert_eq!(rows.len(), 11, "eleven prerequisite-identity rows");
    for (label, set) in rows {
        assert_ne!(set, honest, "the `{label}` row really differs");
        let result = task_with_identities(SOURCE, set).and_then(|task| task.evaluate());
        assert!(result.is_err(), "the `{label}` row was admitted");
    }
}

/// **The decisive prerequisite rows.** Every transported identity is the one
/// the core derives, the derivation is honest and the composition's own
/// contract is untouched — and the route still refuses, because the core
/// re-runs each prerequisite's own contract evaluation.
#[test]
fn an_honest_identity_over_a_falsified_prerequisite_contract_discharges_nothing() {
    if !clang_available() {
        return;
    }
    let falsified_frame = falsify_all_ensures(FRAME_CONTRACT);
    let falsified_backend = falsify_all_ensures(BACKEND_CONTRACT);
    let falsified_unlink = falsify_all_ensures(UNLINK_CONTRACT);
    let falsified_counter = falsify_all_ensures(COUNTER_CONTRACT);
    let rows: [(&str, [&str; 4]); 4] = [
        (
            "frame",
            [
                &falsified_frame,
                BACKEND_CONTRACT,
                UNLINK_CONTRACT,
                COUNTER_CONTRACT,
            ],
        ),
        (
            "backend",
            [
                FRAME_CONTRACT,
                &falsified_backend,
                UNLINK_CONTRACT,
                COUNTER_CONTRACT,
            ],
        ),
        (
            "unlink",
            [
                FRAME_CONTRACT,
                BACKEND_CONTRACT,
                &falsified_unlink,
                COUNTER_CONTRACT,
            ],
        ),
        (
            "counter",
            [
                FRAME_CONTRACT,
                BACKEND_CONTRACT,
                UNLINK_CONTRACT,
                &falsified_counter,
            ],
        ),
    ];
    assert_eq!(rows.len(), 4, "one decisive row per prerequisite");
    for (label, [frame, backend, unlink, counter]) in rows {
        assert!(
            frame != FRAME_CONTRACT
                || backend != BACKEND_CONTRACT
                || unlink != UNLINK_CONTRACT
                || counter != COUNTER_CONTRACT,
            "the `{label}` row really falsifies a contract"
        );
        // The prerequisite tasks are built under the falsified contract, so the
        // transported identities are exactly the ones the core derives, and the
        // composition's own contract selection is untouched.
        let tasks = prerequisites_with(SOURCE, frame, backend, unlink, counter);
        let result = task_from(SOURCE, tasks, &contracts()).and_then(|task| task.evaluate());
        assert!(
            result.is_err(),
            "a falsified `{label}` contract discharged the composition"
        );
    }
}

/// **The card's decisive new class**: the derived text.
#[test]
fn derived_text_substitution_is_refused_in_both_directions() {
    if !clang_available() {
        return;
    }
    let honest = task();
    let honest_witness = witness_for(&honest);

    // (1) An honest recipe over substituted base text. The base text moves, so
    //     the derived text moves with it: the derived import identity, the
    //     derivation record and the projection all move, and neither context
    //     accepts the other's witness.
    let substituted_base = SOURCE.replace("#define CIO_STORE_MEM 1", "#define CIO_STORE_MEM 4");
    assert_ne!(substituted_base, SOURCE);
    let other = task_over(&substituted_base);
    assert_ne!(
        other.task().task_digest(),
        honest.task().task_digest(),
        "substituted base text is a different task"
    );
    assert_ne!(
        other.derivation().base_source_digest,
        honest.derivation().base_source_digest
    );
    assert_ne!(
        other.derivation().derived_source_digest,
        honest.derivation().derived_source_digest
    );
    assert!(
        accept(&other, &honest_witness).is_err(),
        "the honest context's witness is refused by the substituted-base context"
    );
    assert!(
        accept(&honest, &witness_for(&other)).is_err(),
        "the substituted-base context's witness is refused by the honest context"
    );

    // (2) A substituted recipe over honest text. The core's own recipe constant
    //     decides: a projection derived under any other recipe is refused at
    //     derivation, before a task exists.
    for recipe in [
        "\nint qcore_cio_memory_store_value(void) { return CIO_STORE_FS; }\n",
        "\nint qcore_cio_memory_store_value(void) { return 1; }\n",
        "\n\nint qcore_cio_memory_store_value(void) { return CIO_STORE_MEM; }\n",
    ] {
        assert_ne!(recipe, PROBE, "the substituted recipe really differs");
        let result = task_with_recipe(SOURCE, recipe, &contracts());
        assert!(
            result.is_err(),
            "a projection derived under a substituted recipe was admitted: {recipe:?}"
        );
    }

    // (3) The core's own recipe moved instead: the contracts name a different
    //     recipe than the projection was derived under, in both directions.
    let other_recipe: &'static str =
        "\nint qcore_cio_memory_store_value(void) { return CIO_STORE_FS; }\n";
    let result = task_with_recipe(
        SOURCE,
        PROBE,
        &contracts_with(COMPOSITION_CONTRACT, other_recipe),
    );
    assert!(
        result.is_err(),
        "an honest projection under a moved core recipe was admitted"
    );

    // (4) A recipe that derives nothing at all is refused by the importer, so a
    //     "derivation" that is not a derivation cannot exist.
    let empty = task_with_recipe(SOURCE, "", &contracts_with(COMPOSITION_CONTRACT, ""));
    assert!(empty.is_err(), "an empty recipe was admitted");
}

/// The transported callee identities are the core's own reading of its
/// prerequisites' bodies; a substituted pair is refused.
#[test]
fn substituted_callee_identities_are_refused() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites(SOURCE);
    let honest = callees(&tasks);
    let rename = |identity: &CalleeIdentityRecord| CalleeIdentityRecord {
        function: format!("{}_other", identity.function),
        signature: identity.signature.clone(),
    };
    let resign = |identity: &CalleeIdentityRecord| CalleeIdentityRecord {
        function: identity.function.clone(),
        signature: "void (void)".into(),
    };
    let rows: Vec<(&str, MemoryCloseCalleeRecord)> = vec![
        (
            "reset renamed",
            MemoryCloseCalleeRecord {
                reset: rename(&honest.reset),
                ..honest.clone()
            },
        ),
        (
            "backend renamed",
            MemoryCloseCalleeRecord {
                backend: rename(&honest.backend),
                ..honest.clone()
            },
        ),
        (
            "unlink renamed",
            MemoryCloseCalleeRecord {
                unlink: rename(&honest.unlink),
                ..honest.clone()
            },
        ),
        (
            "counter renamed",
            MemoryCloseCalleeRecord {
                counter: rename(&honest.counter),
                ..honest.clone()
            },
        ),
        (
            "reset resigned",
            MemoryCloseCalleeRecord {
                reset: resign(&honest.reset),
                ..honest.clone()
            },
        ),
        (
            "counter resigned",
            MemoryCloseCalleeRecord {
                counter: resign(&honest.counter),
                ..honest.clone()
            },
        ),
        (
            "two callees collapsed onto one frame",
            MemoryCloseCalleeRecord {
                unlink: honest.backend.clone(),
                ..honest.clone()
            },
        ),
    ];
    assert_eq!(rows.len(), 7, "seven callee-substitution rows");
    for (label, record) in rows {
        assert_ne!(record, honest, "the `{label}` row really differs");
        let result = task_with_callees(SOURCE, record).and_then(|task| task.evaluate());
        assert!(result.is_err(), "the `{label}` row was admitted");
    }
}

/// Every clause of the composition's own pinned contract, falsified two ways.
#[test]
fn every_clause_of_the_composition_contract_is_load_bearing() {
    if !clang_available() {
        return;
    }
    let tasks = prerequisites(SOURCE);
    let honest = task_from(SOURCE, tasks, &contracts()).unwrap();
    assert!(honest.evaluate().is_ok());

    let mut rows = 0usize;
    for (index, _) in COMPOSITION_CONTRACT.match_indices("ensures normal ") {
        let end = index + COMPOSITION_CONTRACT[index..].find(';').unwrap();
        for predicate in ["false", "unsupported_predicate"] {
            let changed = format!(
                "{}ensures normal {predicate}{}",
                &COMPOSITION_CONTRACT[..index],
                &COMPOSITION_CONTRACT[end..]
            );
            assert_ne!(changed, COMPOSITION_CONTRACT);
            let tasks = prerequisites(SOURCE);
            let leaked: &'static str = Box::leak(changed.into_boxed_str());
            let result = task_from(SOURCE, tasks, &contracts_with(leaked, PROBE))
                .and_then(|task| task.evaluate());
            assert!(result.is_err(), "a falsified `ensures normal` was admitted");
            rows += 1;
        }
    }
    for premise in [
        "requires exclusive_access;",
        "requires context_counter_covers_live_chunk;",
        "requires free_resolves_to_libc;",
    ] {
        assert!(COMPOSITION_CONTRACT.contains(premise), "{premise}");
        let changed = COMPOSITION_CONTRACT.replace(premise, "");
        let tasks = prerequisites(SOURCE);
        let leaked: &'static str = Box::leak(changed.into_boxed_str());
        let result = task_from(SOURCE, tasks, &contracts_with(leaked, PROBE))
            .and_then(|task| task.evaluate());
        assert!(
            result.is_err(),
            "a withdrawn premise `{premise}` was admitted"
        );
        rows += 1;
    }
    assert_eq!(
        rows, 19,
        "eight `ensures normal` clauses falsified two ways, plus three withdrawn premises"
    );
}

/// The **isolation** row for the two import keys. An unused `-D` leaves the
/// preprocessed text — and therefore the projected bodies, the derivation
/// record and every callee identity — byte-identical, and moves only the
/// captured configuration. Without the separate base-import key the two would
/// be one task.
#[test]
fn a_build_change_invisible_to_the_projection_still_retires_the_task() {
    if !clang_available() {
        return;
    }
    let honest = task();
    let tasks = prerequisites(SOURCE);
    let selection = selection(identities(&tasks), callees(&tasks), PROBE, PROBE_FUNCTION);
    let with_definition = import(
        SOURCE,
        vec!["QCORE_UNUSED_MEMCLOSE=1".to_owned()],
        &selection,
    );
    let other = qcore_verification::composition_source::MemoryCloseCompositionSourceTask::derive(
        &with_definition,
        prerequisites(SOURCE),
        UNIT,
        &contracts(),
        target(),
    )
    .unwrap();
    // The whole transported question is byte-identical...
    assert_eq!(
        serde_json::to_string(
            &honest
                .request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap())
                .composition_source
        )
        .unwrap(),
        serde_json::to_string(
            &other
                .request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap())
                .composition_source
        )
        .unwrap(),
        "an unused definition leaves the projection untouched"
    );
    assert_eq!(honest.derivation(), other.derivation());
    // ...and the tasks are still different, because the build moved.
    assert_ne!(
        honest.task().task_digest(),
        other.task().task_digest(),
        "a build change invisible to the projection still retires the task"
    );
    assert!(accept(&other, &witness_for(&honest)).is_err());
    assert!(accept(&honest, &witness_for(&other)).is_err());
}
