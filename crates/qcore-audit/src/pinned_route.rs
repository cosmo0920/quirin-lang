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

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};

use crate::compile_db::{
    CompileEntry, CompilerOptions, compile_entries,
    compiler_options_compatibility as compiler_options,
};
use crate::current_route::{
    is_monkey_unit, monkey_source_root, source_weight, verify_monkey_source_copy,
};
use crate::input_paths::{Placement, place, relative_within, select_exact, select_under};
use crate::parallel::run_ordered;
use crate::projection::{AuditCounts, project_unit_into_with_cross_unit};
use crate::report::AuditReport;
use qcore_verification::cleanup_outcome::{CleanupCertificate, GuardedDeleteCertificate};
use qcore_verification::counter_contract::CounterCertificate;
use qcore_verification::field_destructor::FieldDestructorCertificate;
use qcore_verification::file_close_composition::FileCloseDeletionEvidence;
use qcore_verification::filesystem_chunk_close_composition::FilesystemChunkCloseEvidence;
use qcore_verification::memory_close_composition::MemoryCloseCertificate;
use quirin_source_analysis::chunkio::parent_teardown::ParentSource;
use quirin_source_analysis::chunkio::traversal::TraversalSource;
use serde_json::Value;

use crate::source_contract::{
    FrameHelperPolicy, SourceContractPolicies, SourceContractRegistry,
    import_configured_frame_helpers_compatibility,
};

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, CrossUnitHomeEvidence, LegacyFunctionScope, ResourceContracts,
    enumerate_source_definitions, prepare_translation_unit,
};
use qcore_contract::contract_loader::load_resource_contract_manifest_stamped;
use qcore_contract::profile_loader::{LoadedProfile, load_audited_bundle, verify_pin};
use qcore_verification::external_source::derive_library_summary_profile;
use quirin_source_analysis::chunk_contract::check_chunk_source_contract;
use quirin_source_analysis::chunkio::{
    ChunkFrameSummary, ChunkSourceResult, import_chunk_source_with_helpers,
};
use quirin_source_analysis::libevent::LIBEVENT_SUBMODULE_PATH;
use quirin_source_analysis::libevent::monkey_source::{
    MonkeySourceResult, import_monkey_callbacks,
};
use quirin_source_analysis::libevent::{classify_libevent_symbol, verify_pinned_libevent_source};
use quirin_source_analysis::source_profiles::{
    FLUENT_BIT_SOURCE_SET, LIBEVENT_SOURCE_SET, load_profile,
};

/// W2: the pinned revision, the clean-path list and the audited-file
/// inventories below come from `contract/profiles/<source-set>/profile.json`.
/// An unloadable profile audits nothing; it never falls back to a built-in
/// table. The ChunkIO provider modules still carry their own revision
/// constant (their migration is not part of this change), so the checks that
/// take a revision string take it from there; a test asserts the profile pin
/// and that constant agree.
use quirin_source_analysis::chunkio::PINNED_FLUENT_BIT_REVISION as PINNED_FLUENT_BIT_COMMIT;

/// One console line of a unit, prelude or postlude, appended to that step's
/// buffer. The buffers are printed in the sequential order (card
/// W44-AUDIT-PARALLEL-UNITS), so a `String` sink, which cannot fail, replaces
/// `println!` everywhere a step may run off the main thread.
macro_rules! say {
    ($out:expr, $($argument:tt)*) => {{
        use std::fmt::Write as _;
        let _ = writeln!($out, $($argument)*);
    }};
}

mod checkpoints;
pub use checkpoints::{
    LAYOUT as CHECKPOINT_LAYOUT, Options as CheckpointOptions, RUN_RECORD as CHECKPOINT_RUN_RECORD,
};

pub fn run(
    root: &Path,
    monkey_database: Option<&Path>,
    monkey_source_report: Option<&Path>,
    diagnostic_report: Option<&Path>,
    binding_directory: Option<&Path>,
    contract_manifest: Option<&Path>,
    jobs: usize,
) -> Result<bool, String> {
    run_with_checkpoints(
        root,
        monkey_database,
        monkey_source_report,
        diagnostic_report,
        binding_directory,
        contract_manifest,
        jobs,
        None,
    )
}

/// Card CORPUS-GROUP-CHECKPOINTS: the pinned replay, or, with `checkpoints`,
/// the same replay whose library groups are sealed into `.qo` checkpoints at
/// every phase and reused on `--resume` when their identity is unchanged
/// ([`checkpoints`]). Without `checkpoints` this is exactly [`run`].
#[allow(clippy::too_many_arguments)]
pub fn run_with_checkpoints(
    root: &Path,
    monkey_database: Option<&Path>,
    monkey_source_report: Option<&Path>,
    diagnostic_report: Option<&Path>,
    binding_directory: Option<&Path>,
    contract_manifest: Option<&Path>,
    jobs: usize,
    checkpoints: Option<&CheckpointOptions>,
) -> Result<bool, String> {
    // Card CORPUS-GROUP-CHECKPOINTS: an environment a checkpoint cannot stand
    // for is refused before any work.
    if checkpoints.is_some() {
        checkpoints::refuse_unsupported()?;
    }
    let cross_unit_links = crate::cross_unit_links::LinkMap::load_selected()?;
    // Card LINK-STANDING (rule 40): a derived resolution is read by the
    // current route only; a pinned route keeps its stated premise.
    if std::env::var_os(crate::cross_unit_links::RESOLUTION_SELECTION).is_some() {
        return Err(format!(
            "{} is read by the current route only; the pinned route states its link resolution",
            crate::cross_unit_links::RESOLUTION_SELECTION
        ));
    }
    // Explicit process selection is checked before any expensive source audit.
    let source_contracts = source_contract_configuration()?;
    // Validate the external build input before running any expensive audit.
    let monkey_input = monkey_database.map(|path| {
        let database = fs::read_to_string(path)
            .map_err(|error| format!("could not read Monkey compile database {}: {error}", path.display()))?;
        if database.trim().is_empty() {
            return Err(format!("Monkey compile database {} is empty; generate persistent inputs with quirin-audit prepare monkey", path.display()));
        }
        let entries = compile_entries(&database)
            .map_err(|error| format!("invalid Monkey compile database {}: {error}", path.display()))?;
        let source_root = monkey_source_root(&entries)
            .map_err(|error| format!("Monkey compile database {}: {error}", path.display()))?;
        // Card AUDIT-CANONICAL-PATHS: two spellings of one Monkey unit are
        // refused before any audit, never merged by the dedup below.
        crate::input_paths::refuse_aliases(
            entries
                .iter()
                .filter(|entry| is_monkey_unit(&source_root, &entry.file)),
        )
        .map_err(|error| format!("Monkey compile database {}: {error}", path.display()))?;
        Ok::<_, String>((entries, source_root))
    }).transpose()?;
    let root = root
        .canonicalize()
        .map_err(|error| format!("could not resolve {}: {error}", root.display()))?;
    let database_path = root.join("build/compile_commands.json");
    let database = fs::read_to_string(&database_path)
        .map_err(|error| format!("could not read {}: {error}", database_path.display()))?;
    let entries = compile_entries(&database)?;
    let profile = load_profile(FLUENT_BIT_SOURCE_SET)
        .map_err(|error| format!("Fluent Bit source-set profile unavailable: {error}"))?;
    let pin = profile
        .manifest()
        .pins
        .get(FLUENT_BIT_SOURCE_SET)
        .ok_or("the fluent-bit profile declares no `fluent-bit` pin")?;
    let revision = verify_pin(FLUENT_BIT_SOURCE_SET, pin, &root)
        .map_err(|error| format!("Fluent Bit pin check failed: {error}"))?;
    println!("Fluent Bit {revision}");
    println!(
        "source profile assumptions: {}",
        serde_json::to_string(&profile.assumptions())
            .map_err(|error| format!("could not serialize profile assumptions: {error}"))?
    );
    let mut report = AuditReport::new(&revision);
    let mut native_results = Vec::new();
    let mut teardown_results = Vec::new();
    audit_library_summary_profiles(&root, &revision, &profile)?;

    let default_manifest = profile.resource_contract_manifest_path();
    let contract_manifest = contract_manifest
        .or(default_manifest.as_deref())
        .ok_or("source-set profile supplies no resource contracts")?;
    let owners = crate::contract_revision::ModuleOwners::for_manifest(
        contract_manifest,
        &fs::read_to_string(contract_manifest).map_err(|e| e.to_string())?,
    )?;
    // Card CONTRACT-SUPERSESSION: a function two selected modules declare is
    // decided by the pin stamps of their locks (contract spec C06.6).
    let mut contracts = load_resource_contract_manifest_stamped(
        contract_manifest,
        &crate::contract_revision::LockStamps::new(&owners),
    )?;
    // Card W45-CONTRACT-REVISION-MODE: every pinned contract is compared with
    // its lock over the audited Fluent Bit source before any unit is analysed.
    let pins = crate::contract_revision::apply_pins(&mut contracts, &owners, &root)?;
    // Card MEASURE-LOOKUP-RECORD-REUSE: a run claiming the base side's
    // analyzer for a contract-only diff is the base build reading the tip's
    // contract inputs, checked before any unit is analysed.
    if let Some(acceptance) = checkpoints.and_then(|options| options.acceptance.as_ref()) {
        let locks = pins
            .lock_digests()
            .into_keys()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        let configuration = profile.directory().join("source-contracts.json");
        let mut inputs = vec![contract_manifest, profile.directory()];
        if configuration.exists() {
            inputs.push(configuration.as_path());
        }
        inputs.extend(locks.iter().map(PathBuf::as_path));
        acceptance
            .verify_run(option_env!("QCORE_AUDIT_BUILD_COMMIT"), &inputs)
            .map_err(|error| format!("analyzer acceptance refused: {error}"))?;
        println!(
            "analyzer accepted from the base side: {} -> {} ({} contract path(s))",
            acceptance.base,
            acceptance.tip,
            acceptance.paths.len()
        );
    }
    // Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: with the bounds property
    // selected, the bounds views of the manifest's load path, pinned over the
    // audited source, before any unit is analysed.
    crate::bounds_selection::select_for_route(
        "pinned",
        contract_manifest,
        &fs::read_to_string(contract_manifest).map_err(|e| e.to_string())?,
        &root,
        &contracts,
    )?;
    // Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): the pin-derived result
    // sets of the installed bounds environment, read by rule 38 while in force.
    let contracts = {
        let mut contracts = contracts;
        contracts.set_result_ranges(crate::bounds_selection::result_range_table());
        contracts
    };
    // Card OUTER-CONTRACT (rule 31): with the runtime projection selected,
    // the outer views of the same load path, pinned over the audited source.
    crate::outer_selection::select_for_route(
        "pinned",
        contract_manifest,
        &fs::read_to_string(contract_manifest).map_err(|e| e.to_string())?,
        &root,
    )?;
    // Card W45-CONTRACT-DEFERRED-REVIEW: kept so the pending-review section can
    // be added once the report it counts is complete.
    let mut contract_sidecar = None;
    if let Some(path) = diagnostic_report {
        let sidecar = path.with_extension("contracts.json");
        let mut inputs = serde_json::json!({
            "schema_version": 1,
            "trust": "declared resource-projection assumptions, not verified C behavior",
            "manifest": fs::read_to_string(contract_manifest).map_err(|e| e.to_string())?,
            "sources": contracts.source_assumptions(),
            "source_profile": profile.assumptions(),
            "audit_selection": fs::read_to_string(profile.directory().join("audit.json")).map_err(|error| error.to_string())?,
        });
        // Card CONTRACT-INHERIT-ADOPTION: an environment that inherited any
        // contract publishes every inheritance chain beside the elaborated
        // sources, as the configured audit's assumptions block does. Absent
        // otherwise, so a selection without inclusion writes the same bytes.
        if let Some(provenance) = qcore_contract::contract_loader::inclusion_provenance(&contracts)
        {
            inputs["contract_inclusion"] = provenance;
        }
        let bytes = serde_json::to_vec_pretty(&inputs).map_err(|e| e.to_string())?;
        fs::write(&sidecar, bytes).map_err(|e| format!("{}: {e}", sidecar.display()))?;
        contract_sidecar = Some((sidecar, inputs));
    }
    let mut failed = false;
    let selection_path = profile.directory().join("audit.json");
    // Card W44-AUDIT-PARALLEL-UNITS: plan every selected library first. The
    // selection is pure, and a planning failure is published at its sequential
    // position — after every library before it — by the fold below, which also
    // stops planning there, as the sequential loop stopped.
    let mut plans = Vec::new();
    let mut planning_failure = None;
    for selected_library in crate::load_library_selections(&selection_path)? {
        match plan_library(&selected_library, &profile, &root, &entries) {
            Ok(plan) => plans.push(plan),
            Err(error) => {
                planning_failure = Some(error);
                break;
            }
        }
    }
    let monkey_plan = monkey_input.map(|(monkey_entries, source_root)| {
        let mut selected = monkey_entries
            .into_iter()
            .filter(|entry| is_monkey_unit(&source_root, &entry.file))
            .collect::<Vec<_>>();
        selected.sort_by(|left, right| left.file.cmp(&right.file));
        selected.dedup_by(|left, right| left.file == right.file);
        LibraryPlan {
            library: "monkey-libevent".into(),
            source_root,
            selected,
        }
    });
    let fluent_bit_libraries = plans.len();
    if planning_failure.is_none() {
        plans.extend(monkey_plan);
    }
    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n; user decisions R1, F1):
    // before any unit is analysed, read the member facts of every unit of the
    // production compile database and of the Monkey units the run analyses,
    // so each unit decides a member invocation over the run's set.
    let prepass = {
        let mut prepass_entries = entries.clone();
        if let Some(plan) = plans.get(fluent_bit_libraries) {
            prepass_entries.extend(plan.selected.iter().cloned());
        }
        let scope = crate::member_prepass::load_scope(profile.directory())?;
        // Card OUTER-CONTRACT (rule 31): what each loader's binders reach,
        // read from the loader's own unit in the production database.
        crate::outer_selection::compute_reach(&prepass_entries)?;
        crate::member_prepass::run(&prepass_entries, &contracts, &scope, Some(&root), jobs)
    };
    if let Some(prepass) = &prepass {
        // Evidence only: the pre-pass section, written as soon as it is read
        // when `QCORE_MEMBER_PREPASS_SECTION` names a file. Card
        // SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 13l): a hand-off's `@`
        // name that Clang does not resolve in its candidate's own result type
        // rejects the contracts before any unit is analysed.
        let _ = crate::member_prepass::announce(prepass, &contracts)?;
    }
    let run_facts = prepass.as_ref().map(|prepass| prepass.facts.clone());
    // The steps in exactly the sequential order. Only `Prelude` and `Unit`
    // carry work that may run off this thread; the consumer below performs
    // every other step itself, at its sequential position.
    let mut steps = Vec::new();
    for (index, plan) in plans.iter().enumerate() {
        if index == fluent_bit_libraries {
            steps.push(Step::MonkeyGate);
        }
        steps.push(Step::Prelude(index));
        steps.extend(plan.selected.iter().map(|entry| Step::Unit(index, entry)));
        steps.push(Step::Finish(index));
    }
    match planning_failure {
        Some(error) => steps.push(Step::PlanningFailed(error)),
        None if plans.len() == fluent_bit_libraries => steps.push(Step::MonkeyGate),
        None => {}
    }
    // Start every library prelude first, so a unit's worker never waits long
    // on its library's context, then the largest units first. Dispatch moves
    // wall time only; consumption stays in step order.
    let weights = steps
        .iter()
        .map(|step| match step {
            Step::Prelude(_) => u64::MAX,
            Step::Unit(_, entry) => source_weight(entry),
            _ => 0,
        })
        .collect::<Vec<_>>();
    let provider = Mutex::new(());
    let mut library_counts = AuditCounts::default();
    let mut captures = ChunkioCaptures::default();
    // Card CORPUS-GROUP-CHECKPOINTS: the checkpointed phases produce the same
    // step outcomes, refutation table and cross-unit premise, each library
    // group reused or recomputed per phase; the fold below is shared.
    let (outcomes, applied, cross_unit_premise) = if let Some(options) = checkpoints {
        checkpoints::run_phases(
            &checkpoints::Route {
                plans: &plans,
                steps: &steps,
                planning_failed: steps
                    .iter()
                    .any(|step| matches!(step, Step::PlanningFailed(_))),
                root: &root,
                revision: &revision,
                profile: &profile,
                contract_manifest,
                contracts: &contracts,
                pins: &pins,
                links: cross_unit_links.as_ref(),
                run_facts: run_facts.as_ref(),
                source_contracts: &source_contracts,
                provider: &provider,
                jobs,
            },
            options,
        )?
    } else {
        let dispatch = crate::parallel::heaviest_first(&weights);
        let contexts = (0..plans.len())
            .map(|_| OnceLock::new())
            .collect::<Vec<OnceLock<Option<LibraryContext>>>>();
        // Card W45-REFUTED-CONTRACT-WITHDRAWAL: one step's work under a given
        // environment. Phase one runs every step under the loaded contracts;
        // when a body refutes its own declaration, phase two analyses every unit
        // again under the refutation table (each library's prelude context is
        // reused), and only then is every outcome folded, in step order.
        let work = |step: Step<'_>,
                    contracts: &ResourceContracts,
                    homes: &[CrossUnitHomeEvidence]| match step {
            Step::Prelude(index) => {
                let plan = &plans[index];
                let mut output = String::new();
                // Publishes `None` if the prelude fails or panics, so a unit
                // waiting on this library never blocks forever.
                let publish = PublishOnDrop(&contexts[index]);
                let result = prepare_library(plan, &source_contracts, &mut output)
                    .map(|context| publish.publish(context));
                StepOutcome::Prelude { output, result }
            }
            Step::Unit(index, entry) => {
                // Card DEEP-AST-TRANSPORTS-AND-WALL-TIME: measurement only.
                #[cfg(all(feature = "unit-timing", target_os = "linux"))]
                let _timing = crate::unit_timing::Span::start(&entry.file);
                let plan = &plans[index];
                let mut output = String::new();
                let result = match contexts[index].wait() {
                    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (R1): the unit is
                    // analysed with the run's member facts installed.
                    Some(context) => {
                        qcore_c_gateway::member_run_facts::scoped(run_facts.clone(), || {
                            audit_unit(
                                plan,
                                context,
                                entry,
                                &revision,
                                contracts,
                                homes,
                                &source_contracts,
                                &provider,
                                &mut output,
                            )
                        })
                    }
                    // Unreachable in the fold: the prelude's own failure is
                    // consumed first and stops it.
                    None => Err(format!("{}: library prelude failed", plan.library)),
                };
                StepOutcome::Unit {
                    output,
                    result: Box::new(result),
                }
            }
            Step::Finish(index) => StepOutcome::Finish(index),
            Step::MonkeyGate => StepOutcome::MonkeyGate,
            Step::PlanningFailed(error) => StepOutcome::PlanningFailed(error),
        };
        let stops = |outcome: &StepOutcome| match outcome {
            StepOutcome::Prelude { result, .. } => result.is_err(),
            StepOutcome::Unit { result, .. } => result.is_err(),
            StepOutcome::PlanningFailed(_) => true,
            StepOutcome::Finish(_)
            | StepOutcome::MonkeyGate
            | StepOutcome::StoredUnit(_)
            | StepOutcome::Finished { .. } => false,
        };
        let table_of = |outcomes: &[StepOutcome]| {
            crate::refuted_contracts::table(
                &outcomes
                    .iter()
                    .filter_map(|outcome| match outcome {
                        StepOutcome::Unit { result, .. } => result.as_ref().as_ref().ok(),
                        _ => None,
                    })
                    .flat_map(|products| products.report.unit_rows().iter().cloned())
                    .collect::<Vec<_>>(),
            )
        };
        let mut outcomes = Vec::new();
        let stopped = run_ordered(
            steps.clone(),
            jobs,
            Some(dispatch.clone()),
            |step| work(step, &contracts, &[]),
            |outcome| {
                let stop = stops(&outcome);
                outcomes.push(outcome);
                if stop { Err(()) } else { Ok(()) }
            },
        )
        .is_err();
        let found = table_of(&outcomes);
        // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14h): the
        // bounds declarations phase one's bodies refute also call for phase two.
        let bounds_found = crate::bounds_evidence::refutation_table(
            &outcomes
                .iter()
                .filter_map(|outcome| match outcome {
                    StepOutcome::Unit { result, .. } => result.as_ref().as_ref().ok(),
                    _ => None,
                })
                .flat_map(|products| products.report.bounds_evidence().iter().cloned())
                .collect::<Vec<_>>(),
        );
        let applied = if stopped || (found.is_empty() && bounds_found.is_empty()) {
            qcore_schemas::contract_refutation::RefutationTable::new()
        } else {
            crate::bounds_selection::apply_refutation_table(&bounds_found);
            let held = crate::refuted_contracts::with_table(&contracts, &found);
            let units = steps
                .iter()
                .enumerate()
                .filter(|(_, step)| matches!(step, Step::Unit(..)))
                .map(|(index, step)| (index, step.clone()))
                .collect::<Vec<_>>();
            let unit_count = units.len();
            let weights = units
                .iter()
                .map(|(_, step)| match step {
                    Step::Unit(_, entry) => source_weight(entry),
                    _ => 0,
                })
                .collect::<Vec<_>>();
            let mut replaced = Vec::new();
            let _ = run_ordered(
                units,
                jobs,
                Some(crate::parallel::heaviest_first(&weights)),
                |(index, step)| (index, work(step, &held, &[])),
                |(index, outcome)| {
                    let stop = stops(&outcome);
                    replaced.push((index, outcome));
                    if stop { Err(()) } else { Ok(()) }
                },
            );
            // Phase one ran to its end, so every unit has its slot; a phase-two
            // unit that failed ends the fold at its own position.
            let complete = replaced.len() == unit_count;
            let last = replaced.last().map(|(index, _)| *index);
            for (index, outcome) in replaced {
                outcomes[index] = outcome;
            }
            if !complete {
                outcomes.truncate(last.map_or(outcomes.len(), |index| index + 1));
            }
            found
        };
        // A link map is a declared program-resolution premise, never inferred
        // from the corpus. Export homes under the refutation table actually
        // applied, then re-run caller units with only uniquely named imports.
        let mut cross_unit_premise = None;
        if let Some(links) = &cross_unit_links
            && outcomes.len() == steps.len()
        {
            let held = crate::refuted_contracts::with_table(&contracts, &applied);
            let (imported, homes) = select_homes(links, &plans, &held)?;
            if !homes.is_empty() {
                let units = steps
                    .iter()
                    .enumerate()
                    .filter(|(_, step)| matches!(step, Step::Unit(..)))
                    .map(|(index, step)| (index, step.clone()))
                    .collect::<Vec<_>>();
                let unit_count = units.len();
                let weights = units
                    .iter()
                    .map(|(_, step)| match step {
                        Step::Unit(_, entry) => source_weight(entry),
                        _ => 0,
                    })
                    .collect::<Vec<_>>();
                let mut replaced = Vec::new();
                let stopped = run_ordered(
                    units,
                    jobs,
                    Some(crate::parallel::heaviest_first(&weights)),
                    |(index, step)| (index, work(step, &imported, &homes)),
                    |(index, outcome)| {
                        let stop = stops(&outcome);
                        replaced.push((index, outcome));
                        if stop { Err(()) } else { Ok(()) }
                    },
                )
                .is_err();
                let new_table = crate::refuted_contracts::table(
                    &replaced
                        .iter()
                        .filter_map(|(_, outcome)| match outcome {
                            StepOutcome::Unit { result, .. } => result.as_ref().as_ref().ok(),
                            _ => None,
                        })
                        .flat_map(|products| products.report.unit_rows().iter().cloned())
                        .collect::<Vec<_>>(),
                );
                if !stopped && replaced.len() == unit_count && new_table == applied {
                    for (index, outcome) in replaced {
                        outcomes[index] = outcome;
                    }
                    cross_unit_premise = Some(cross_unit_premise_of(links, &homes));
                } else {
                    println!(
                        "cross-unit import phase refused: incomplete run or changed refutation table"
                    );
                }
            }
        }
        (outcomes, applied, cross_unit_premise)
    };
    for outcome in outcomes {
        match outcome {
            StepOutcome::Prelude { output, result } => {
                print!("{output}");
                result?;
                library_counts = AuditCounts::default();
                captures = ChunkioCaptures::default();
            }
            outcome @ (StepOutcome::Unit { .. } | StepOutcome::StoredUnit(_)) => {
                // Card CORPUS-GROUP-CHECKPOINTS: a checkpointed unit is
                // restored here, one at a time, into the products the
                // whole run folds.
                let (output, result) = match outcome {
                    StepOutcome::Unit { output, result } => (output, *result),
                    StepOutcome::StoredUnit(unit) => {
                        let result = checkpoints::restore_unit(&revision, &unit);
                        (unit.output, result)
                    }
                    _ => unreachable!("matched a unit outcome"),
                };
                print!("{output}");
                let UnitProducts {
                    report: rows,
                    counts,
                    native_results: native,
                    teardown_results: teardown,
                    captures: captured,
                } = result?;
                report.absorb(rows)?;
                library_counts.absorb(counts);
                native_results.extend(native);
                teardown_results.extend(teardown);
                captures.absorb(captured);
            }
            StepOutcome::Finish(index) => {
                let plan = &plans[index];
                let mut output = String::new();
                let result = finish_library(
                    &plan.library,
                    std::mem::take(&mut captures),
                    &mut teardown_results,
                    &mut output,
                );
                print!("{output}");
                result?;
                let counts = std::mem::take(&mut library_counts);
                failed |= counts.gateway_failures != 0;
                println!("{}", library_tally(&plan.library, &counts));
            }
            // Card CORPUS-GROUP-CHECKPOINTS: the library postlude a
            // checkpointed phase ran when the group completed (the same
            // `finish_library` over the same captures, in unit order), with
            // the teardown results it appended.
            StepOutcome::Finished {
                index,
                output,
                result,
            } => {
                let plan = &plans[index];
                print!("{output}");
                teardown_results.extend(result?);
                captures = ChunkioCaptures::default();
                let counts = std::mem::take(&mut library_counts);
                failed |= counts.gateway_failures != 0;
                println!("{}", library_tally(&plan.library, &counts));
            }
            StepOutcome::MonkeyGate => {
                audit_monkey_libevent_contract(&root)?;
                if let Some(plan) = plans.get(fluent_bit_libraries) {
                    verify_monkey_source_copy(
                        &root.join("lib/monkey"),
                        &plan.source_root,
                        &plan.selected,
                    )?;
                    audit_monkey_callback_source(
                        &plan.selected,
                        monkey_source_report,
                        binding_directory,
                    )?;
                }
            }
            StepOutcome::PlanningFailed(error) => return Err(error),
        }
    }
    if plans.len() == fluent_bit_libraries {
        println!(
            "monkey-libevent: source contract checked; pass its forced-libevent \
             compile_commands.json to run the C gateway projection"
        );
    }
    if let Some(premise) = &cross_unit_premise {
        report.record_cross_unit_premise(premise);
    }
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): the run's closure
    // of every invoked member, folded over the rows before anything is
    // published; it only demotes. The per-library console lines above are the
    // analysis's own; the fold's demotions are tallied here.
    if let Some(prepass) = prepass {
        report.set_member_prepass(prepass.facts, prepass.section);
    }
    let (members, _, _) = report.fold_callback_members(&contracts);
    if !members.is_null() {
        println!(
            "callback members: {} demoted",
            members["demoted"].as_array().map_or(0, Vec::len)
        );
    }
    // Card W45-CONTRACT-DEFERRED-REVIEW: the pending reviews and what they
    // held, tallied on the console and added to the contract sidecar. Additive:
    // every other byte of the sidecar is the one written before the analysis.
    let pending = crate::pending_review::section(&contracts, &report);
    println!("{}", crate::pending_review::tally(&pending));
    // Card W45-CONTRACT-REVISION-MODE: revision modes, pin mismatches and
    // flexible assumptions, tallied and added the same additive way.
    let revision_section = crate::contract_revision::section(&contracts, &pins, &report);
    println!("{}", crate::contract_revision::tally(&revision_section));
    // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13h): the rows' dependencies
    // under that inventory, tallied and added the same additive way.
    let dependency_premises = crate::contract_dependencies::premises(&contracts)?;
    let dependency_section =
        crate::contract_dependencies::section(&revision_section, &dependency_premises, &report);
    println!(
        "{}",
        crate::contract_dependencies::tally(&dependency_section)
    );
    // Card W45-REFUTED-CONTRACT-WITHDRAWAL: the refutation table, its digest
    // and the calls it held, tallied and added the same additive way.
    let refuted_section = crate::refuted_contracts::section(&applied, &report);
    println!("{}", crate::refuted_contracts::tally(&refuted_section));
    if let Some((sidecar, mut inputs)) = contract_sidecar {
        if let Some(premise) = &cross_unit_premise {
            inputs["cross_unit_link_resolution"] = premise.clone();
        }
        inputs["pending_human_review"] = pending;
        inputs["contract_revision"] = revision_section;
        inputs["refuted_contracts"] = refuted_section;
        inputs[crate::contract_dependencies::SECTION_KEY] = dependency_section;
        if !members.is_null() {
            inputs[crate::callback_member_fold::SECTION_KEY] = members;
        }
        let bytes = serde_json::to_vec_pretty(&inputs).map_err(|e| e.to_string())?;
        fs::write(&sidecar, bytes).map_err(|e| format!("{}: {e}", sidecar.display()))?;
    }
    if let Some(path) = diagnostic_report {
        // Card AUDIT-COMPOSITION: the contract bundle the components rest on.
        report.record_contract_bundle(&contracts);
        report.write_with_revision_key(path, "fluent_bit_revision")?;
        report.write_bounds_selection()?;
        report.write_loop_measure_selection()?;
        let null_rows = report.write_null_selection()?;
        report.write_outer_projection(&contracts, null_rows.as_deref())?;
        let (provider_results, native_results): (Vec<_>, Vec<_>) = native_results
            .into_iter()
            .partition(|record| record["kind"] == "provider_source_effect");
        if !provider_results.is_empty() {
            fs::write(path.with_extension("source-effects.json"), serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "scope": "source-derived Boolean-field function contracts under explicit caller preconditions; no whole-unit promotion",
                "results": provider_results,
            })).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
        }
        let sidecar = path.with_extension("chunk-contracts.json");
        let inputs = serde_json::json!({
            "schema_version": 1, "source_revision": revision,
            "scope": "bounded field contracts; live/exclusive/Boolean domains and source/build/ABI identity assumed; no whole-unit lifecycle certificate",
            "results": native_results,
            "backend_teardown": teardown_results,
            "source_policy_contract": include_str!("../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"),
            "configured_frame_helper_policies": source_contracts.frame_helper_policy_inventory(),
            "frame_helper_contract": include_str!("../../../providers/qcore-provider-chunkio/contracts/chunkio/error_state.qv"),
        });
        fs::write(
            &sidecar,
            serde_json::to_vec_pretty(&inputs).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("Resource diagnostics: {}", path.display());
    }
    Ok(failed)
}

/// One library's console tally after its postlude.
fn library_tally(library: &str, counts: &AuditCounts) -> String {
    format!(
        "{}: {} unit(s), {} source function(s), {} projected; {} data-only, {} Safe, {} Pending, {} Unsafe, {} Unknown, {} gateway failure(s)",
        library,
        counts.units,
        counts.source_functions,
        counts.projected_functions,
        counts.data_only_units,
        counts.safe_units,
        counts.pending_units,
        counts.unsafe_units,
        counts.unknown_units,
        counts.gateway_failures
    )
}

/// The cross-unit homes the link map selects under `held` (the contracts
/// with the applied refutation table): every selected unit is prepared for
/// the definition census, and each requested home exports its summary. A
/// census that is not complete refuses the import phase (no homes).
fn select_homes(
    links: &crate::cross_unit_links::LinkMap,
    plans: &[LibraryPlan],
    held: &ResourceContracts,
) -> Result<(ResourceContracts, Vec<CrossUnitHomeEvidence>), String> {
    let mut exports = Vec::new();
    let mut definitions = Vec::new();
    let requested_names = links.requested();
    let mut census_complete = true;
    for plan in plans {
        for entry in &plan.selected {
            let file = relative_within(&entry.file, &plan.source_root)
                .ok_or("cross-unit home outside library root")?;
            let file = file.to_string_lossy();
            let key = format!("{}/{file}", plan.library);
            let requested = links.names_for(&key);
            let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
            let options = compiler_options(entry)?;
            let unit = C11TranslationUnit {
                source: &source,
                include_paths: options.include_paths,
                definitions: options.definitions,
                preprocessor_options: options.preprocessor_options,
                dialect: options.dialect,
                function_scope: LegacyFunctionScope::MainSourceDefinitions,
                input: options.input,
            };
            let prepared = prepare_translation_unit(&unit);
            census_complete &= prepared.inventory().is_complete();
            definitions.extend(
                prepared
                    .inventory()
                    .definitions()
                    .iter()
                    .filter(|name| requested_names.contains(*name))
                    .map(|name| (key.clone(), name.clone())),
            );
            if !requested.is_empty() {
                exports.extend(
                    qcore_c_gateway::legacy_c::export_cross_unit_homes(
                        &prepared,
                        held,
                        &requested,
                        &links.digest,
                    )?
                    .into_iter()
                    .map(|export| (key.clone(), export)),
                );
            }
        }
    }
    Ok(if census_complete {
        links.select(exports, definitions, held)
    } else {
        println!("cross-unit import phase refused: incomplete definition census");
        (held.clone(), Vec::new())
    })
}

/// The report's record of the cross-unit premise the import phase applied.
fn cross_unit_premise_of(
    links: &crate::cross_unit_links::LinkMap,
    homes: &[CrossUnitHomeEvidence],
) -> Value {
    serde_json::json!({
        "scope": "conditional: each external call to an imported symbol resolves to its named home definition with compatible function type and calling convention in the actual link and interposition environment",
        // Card CURRENT-RUN-WORKER-SPIN (rule 4c, amended): the stated
        // work bound; a caller reaching more homes holds none.
        "replay_budget_per_unit": qcore_c_gateway::legacy_c::CROSS_UNIT_REPLAY_BUDGET_PER_UNIT,
        "premise": links.premise,
        "map_sha256": links.digest,
        "imported": homes.iter().map(|home| serde_json::json!({
            "function": home.function,
            "home_unit": links.home_for(&home.function),
            "home_raw_source_sha256": home.input_identity.raw_source_sha256,
            "home_preprocessed_source_sha256": home.input_identity.preprocessed_source_sha256,
            "home_environment_sha256": home.environment_sha256,
        })).collect::<Vec<_>>(),
    })
}

fn audit_library_summary_profiles(
    root: &Path,
    revision: &str,
    profile: &LoadedProfile,
) -> Result<(), String> {
    for name in ["librdkafka", "cfl", "chunkio"] {
        let library = profile
            .library(name)
            .ok_or_else(|| format!("the fluent-bit profile declares no `{name}` library"))?;
        let bundle = load_audited_bundle(library, root, revision).map_err(|diagnostics| {
            format!("{name} audited source inventory failed: {diagnostics:?}")
        })?;
        let expected_revision = &profile
            .pin_for(name)
            .ok_or("library has no declared pin")?
            .git_commit;
        let certificate = derive_library_summary_profile(name, library, expected_revision, &bundle)
            .map_err(|diagnostics| {
                format!("{name} source-summary contract failed: {diagnostics:?}")
            })?;
        println!(
            "{name} source summaries: {} contract(s), {} file(s), {:?}",
            certificate.checked_contracts.len(),
            certificate.checked_files.len(),
            certificate.profile.summaries
        );
    }
    Ok(())
}

/// Card AUDIT-CANONICAL-PATHS: the route's existing outside-root diagnostic,
/// now also for a record spelled under the root that resolves outside it.
fn outside_audit_root(entry: &CompileEntry) -> String {
    format!("source is outside audit root: {}", entry.file.display())
}

fn is_c_unit(entry: &CompileEntry) -> bool {
    entry
        .file
        .extension()
        .is_some_and(|extension| extension == "c")
}

/// Membership by located path (card AUDIT-CANONICAL-PATHS,
/// [`crate::input_paths`]); every selected record keeps its spelling, and the
/// order and the merge of repeated spellings are as before.
fn select_recursive_units(
    entries: &[CompileEntry],
    source_root: &Path,
) -> Result<Vec<CompileEntry>, String> {
    let mut selected = select_under(
        entries,
        source_root,
        |entry, _| is_c_unit(entry),
        outside_audit_root,
    )?
    .into_iter()
    .cloned()
    .collect::<Vec<_>>();
    selected.sort_by(|left, right| left.file.cmp(&right.file));
    selected.dedup_by(|left, right| left.file == right.file);
    Ok(selected)
}

/// The C records directly in the source root, sorted, repeated spellings kept
/// as before.
fn select_top_level_units(
    entries: &[CompileEntry],
    source_root: &Path,
) -> Result<Vec<CompileEntry>, String> {
    let mut selected = select_under(
        entries,
        source_root,
        |entry, relative| relative.components().count() == 1 && is_c_unit(entry),
        outside_audit_root,
    )?
    .into_iter()
    .cloned()
    .collect::<Vec<_>>();
    selected.sort_by(|left, right| left.file.cmp(&right.file));
    Ok(selected)
}

/// The first record of the exact file, as before; the file must locate under
/// the inputs root, and two spellings of it are refused.
fn select_exact_unit(
    entries: &[CompileEntry],
    root: &Path,
    source: &Path,
) -> Result<Vec<CompileEntry>, String> {
    if place(source, root) == Placement::Escapes {
        return Err(format!(
            "source is outside audit root: {}",
            source.display()
        ));
    }
    Ok(select_exact(entries, source)?
        .into_iter()
        .next()
        .cloned()
        .into_iter()
        .collect())
}

/// One selected library, planned before any of its units is analysed.
struct LibraryPlan {
    library: String,
    source_root: PathBuf,
    selected: Vec<CompileEntry>,
}

/// Select one library's units exactly as the sequential loop did.
fn plan_library(
    selected_library: &crate::LibrarySelection,
    profile: &LoadedProfile,
    root: &Path,
    entries: &[CompileEntry],
) -> Result<LibraryPlan, String> {
    let declared = profile.library(&selected_library.library).ok_or_else(|| {
        format!(
            "profile declares no selected library `{}`",
            selected_library.library
        )
    })?;
    let [selection] = declared.source_roots.as_slice() else {
        return Err(
            "the compatibility adapter requires one source root per selected library".into(),
        );
    };
    let source_root = qcore_contract::profile_loader::source_root_path(root, &selection.root);
    let selected = match &selection.selection {
        qcore_schemas::profile::UnitSelection::TopLevel => {
            select_top_level_units(entries, &source_root)?
        }
        qcore_schemas::profile::UnitSelection::Recursive => {
            select_recursive_units(entries, &source_root)?
        }
        qcore_schemas::profile::UnitSelection::Exact { file } => {
            select_exact_unit(entries, root, &root.join(file))?
        }
    };
    let library = selected_library
        .label
        .as_deref()
        .unwrap_or(&selected_library.library);
    if selected.is_empty() {
        return Err(format!(
            "compile database contains no selected {library} sources"
        ));
    }
    Ok(LibraryPlan {
        library: library.to_owned(),
        source_root,
        selected,
    })
}

/// One step of the pinned audit, in sequential order.
#[derive(Clone)]
enum Step<'p> {
    /// The library's own checks before its units (frame helpers; ChunkIO's
    /// memory backend and filesystem dependencies).
    Prelude(usize),
    Unit(usize, &'p CompileEntry),
    /// The library's checks after its units, and its summary line.
    Finish(usize),
    /// The Monkey/Libevent contract check, Monkey source-copy check and
    /// callback source audit, between the Fluent Bit libraries and Monkey.
    MonkeyGate,
    PlanningFailed(String),
}

enum StepOutcome {
    Prelude {
        output: String,
        result: Result<(), String>,
    },
    Unit {
        output: String,
        result: Box<Result<UnitProducts, String>>,
    },
    /// Card CORPUS-GROUP-CHECKPOINTS: a unit a checkpointed phase sealed or
    /// reused, restored by the fold.
    StoredUnit(Box<checkpoints::StoredUnit>),
    Finish(usize),
    /// Card CORPUS-GROUP-CHECKPOINTS: a postlude a checkpointed phase ran,
    /// with the teardown results it appended.
    Finished {
        index: usize,
        output: String,
        result: Result<Vec<Value>, String>,
    },
    MonkeyGate,
    PlanningFailed(String),
}

/// What a library prelude computes for its units.
struct LibraryContext {
    chunk_helpers: Vec<ChunkFrameSummary>,
    memory_backend: Option<FieldDestructorCertificate>,
    filesystem_deps: Option<(
        CleanupCertificate,
        CounterCertificate,
        CleanupCertificate,
        GuardedDeleteCertificate,
    )>,
    filesystem_close: Option<FileCloseDeletionEvidence>,
}

/// Publishes a library context to the units waiting on it; publishes `None`
/// when dropped unpublished, so a failed or panicking prelude releases them.
struct PublishOnDrop<'a>(&'a OnceLock<Option<LibraryContext>>);

impl PublishOnDrop<'_> {
    fn publish(self, context: LibraryContext) {
        let _ = self.0.set(Some(context));
    }
}

impl Drop for PublishOnDrop<'_> {
    fn drop(&mut self) {
        let _ = self.0.set(None);
    }
}

/// ChunkIO unit evidence the library postlude composes. The sequential loop
/// assigned each when its unit was audited, so a later unit's value replaced
/// an earlier one's; [`Self::absorb`] keeps exactly that rule in unit order.
#[derive(Default)]
struct ChunkioCaptures {
    nested_memory: Option<MemoryCloseCertificate>,
    chunk_traversal: Option<TraversalSource>,
    stream_traversal: Option<TraversalSource>,
    stream_parent: Option<ParentSource>,
    context_parent: Option<ParentSource>,
    filesystem_chunk: Option<FilesystemChunkCloseEvidence>,
}

impl ChunkioCaptures {
    fn absorb(&mut self, later: Self) {
        let Self {
            nested_memory,
            chunk_traversal,
            stream_traversal,
            stream_parent,
            context_parent,
            filesystem_chunk,
        } = later;
        fn replace<T>(slot: &mut Option<T>, later: Option<T>) {
            if later.is_some() {
                *slot = later;
            }
        }
        replace(&mut self.nested_memory, nested_memory);
        replace(&mut self.chunk_traversal, chunk_traversal);
        replace(&mut self.stream_traversal, stream_traversal);
        replace(&mut self.stream_parent, stream_parent);
        replace(&mut self.context_parent, context_parent);
        replace(&mut self.filesystem_chunk, filesystem_chunk);
    }
}

/// Everything one unit contributes, folded in unit order by the consumer.
struct UnitProducts {
    report: AuditReport,
    counts: AuditCounts,
    native_results: Vec<Value>,
    teardown_results: Vec<Value>,
    captures: ChunkioCaptures,
}

/// The library prelude: every check the sequential `audit_library` made
/// before its unit loop.
fn prepare_library(
    plan: &LibraryPlan,
    source_contracts: &SourceContractConfiguration,
    out: &mut String,
) -> Result<LibraryContext, String> {
    let library = plan.library.as_str();
    let source_root = plan.source_root.as_path();
    let entries = plan.selected.as_slice();
    let mut chunk_helpers = Vec::new();
    let mut memory_backend = None;
    let mut filesystem_deps = None;
    let mut filesystem_close = None;
    let helper_policies = source_contracts.frame_helpers_for_library(library);
    if !helper_policies.is_empty() {
        let profile = load_profile(FLUENT_BIT_SOURCE_SET).map_err(|error| error.to_string())?;
        for checked in import_configured_frame_helpers_compatibility(
            helper_policies,
            source_root,
            entries,
            PINNED_FLUENT_BIT_COMMIT,
            &profile,
            library,
            include_str!(
                "../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"
            ),
        )
        .map_err(|error| error.to_string())?
        {
            let function = checked.summary.function();
            let postconditions = checked.checked_postconditions;
            say!(
                out,
                "qcore-contract {function}: Checked ({postconditions} postconditions; complete constant-field helper body, live/exclusive premises assumed)"
            );
            say!(
                out,
                "chunk-source dependency {function}: source-validated constant field writes; revision/build/ABI supplied by pinned audit"
            );
            chunk_helpers.push(checked.summary);
        }
    }
    if library == "chunkio" {
        let entry = entries
            .iter()
            .find(|entry| entry.file.ends_with("cio_memfs.c"))
            .ok_or("missing compiled memory backend")?;
        let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
        let options = compiler_options(entry)?;
        let unit = C11TranslationUnit {
            source: &source,
            include_paths: options.include_paths,
            definitions: options.definitions,
            preprocessor_options: options.preprocessor_options,
            dialect: options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            input: options.input,
        };
        memory_backend = Some(
            quirin_source_analysis::chunkio::teardown_source::check_memory_backend_source(
                &unit,
                PINNED_FLUENT_BIT_COMMIT,
            )?,
        );
    }
    if library == "chunkio" {
        let load = |file: &str| -> Result<(String, CompilerOptions), String> {
            let entry = entries
                .iter()
                .find(|entry| entry.file.ends_with(file))
                .ok_or_else(|| format!("missing compiled dependency {file}"))?;
            Ok((
                fs::read_to_string(&entry.file).map_err(|error| error.to_string())?,
                compiler_options(entry)?,
            ))
        };
        let (native_source, native_options) = load("cio_file_unix.c")?;
        let native_unit = C11TranslationUnit {
            source: &native_source,
            include_paths: native_options.include_paths,
            definitions: native_options.definitions,
            preprocessor_options: native_options.preprocessor_options,
            dialect: native_options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            input: native_options.input,
        };
        let native = quirin_source_analysis::chunkio::file_outcomes::check_file_outcome_sources(
            &native_unit,
            PINNED_FLUENT_BIT_COMMIT,
        )?
        .into_iter()
        .find(|c| c.source().function() == "cio_file_native_unmap")
        .ok_or("missing native unmap certificate")?;
        let close = quirin_source_analysis::chunkio::file_outcomes::check_file_outcome_sources(
            &native_unit,
            PINNED_FLUENT_BIT_COMMIT,
        )?
        .into_iter()
        .find(|c| c.source().function() == "cio_file_native_close")
        .ok_or("missing native close certificate")?;
        let (counter_source, counter_options) = load("cio_chunk.c")?;
        let counter_unit = C11TranslationUnit {
            source: &counter_source,
            include_paths: counter_options.include_paths,
            definitions: counter_options.definitions,
            preprocessor_options: counter_options.preprocessor_options,
            dialect: counter_options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            input: counter_options.input,
        };
        let counter = quirin_source_analysis::chunkio::teardown_source::check_counter_sources(
            &counter_unit,
            PINNED_FLUENT_BIT_COMMIT,
        )?
        .into_iter()
        .find(|c| c.field() == "total_chunks_up" && c.delta() == -1)
        .ok_or("missing up counter certificate")?;
        let delete = quirin_source_analysis::chunkio::file_delete::check_native_delete_source(
            &native_unit,
            PINNED_FLUENT_BIT_COMMIT,
        )?;
        let (file_source, file_options) = load("cio_file.c")?;
        let file_unit = C11TranslationUnit {
            source: &file_source,
            include_paths: file_options.include_paths,
            definitions: file_options.definitions,
            preprocessor_options: file_options.preprocessor_options,
            dialect: file_options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            input: file_options.input,
        };
        let unmap = quirin_source_analysis::chunkio::unmap_wrapper::check_unmap_wrapper_source(
            &file_unit,
            PINNED_FLUENT_BIT_COMMIT,
            &native,
            &counter,
        )?;
        filesystem_close = Some(
            quirin_source_analysis::chunkio::file_close::check_file_close_deletion_source(
                &file_unit,
                PINNED_FLUENT_BIT_COMMIT,
                &unmap,
                &close,
                &delete,
            )?,
        );
        filesystem_deps = Some((native, counter, close, delete));
    }
    Ok(LibraryContext {
        chunk_helpers,
        memory_backend,
        filesystem_deps,
        filesystem_close,
    })
}

/// One unit: the body of the sequential `audit_library` loop, recording into
/// its own report fragment, tally and result lists.
#[allow(clippy::too_many_arguments)]
fn audit_unit(
    plan: &LibraryPlan,
    context: &LibraryContext,
    entry: &CompileEntry,
    revision: &str,
    contracts: &ResourceContracts,
    homes: &[CrossUnitHomeEvidence],
    source_contracts: &SourceContractConfiguration,
    provider: &Mutex<()>,
    out: &mut String,
) -> Result<UnitProducts, String> {
    qcore_c_gateway::with_audit_unit_lifetimes(|| {
        audit_unit_unscoped(
            plan,
            context,
            entry,
            revision,
            contracts,
            homes,
            source_contracts,
            provider,
            out,
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn audit_unit_unscoped(
    plan: &LibraryPlan,
    context: &LibraryContext,
    entry: &CompileEntry,
    revision: &str,
    contracts: &ResourceContracts,
    homes: &[CrossUnitHomeEvidence],
    source_contracts: &SourceContractConfiguration,
    provider: &Mutex<()>,
    out: &mut String,
) -> Result<UnitProducts, String> {
    let library = plan.library.as_str();
    let source_root = plan.source_root.as_path();
    let LibraryContext {
        chunk_helpers,
        memory_backend,
        filesystem_deps,
        filesystem_close,
    } = context;
    let mut report = AuditReport::new(revision);
    let mut counts = AuditCounts::default();
    let mut native_results = Vec::new();
    let mut teardown_results = Vec::new();
    let mut nested_memory = None;
    let mut chunk_traversal = None;
    let mut stream_traversal = None;
    let mut stream_parent = None;
    let mut context_parent = None;
    let mut filesystem_chunk = None;
    let source = fs::read_to_string(&entry.file)
        .map_err(|error| format!("could not read {}: {error}", entry.file.display()))?;
    let file =
        relative_within(&entry.file, source_root).ok_or_else(|| outside_audit_root(entry))?;
    let file = file.to_string_lossy();
    let options = compiler_options(entry)?;
    let unit = C11TranslationUnit {
        source: &source,
        include_paths: options.include_paths,
        definitions: options.definitions,
        preprocessor_options: options.preprocessor_options,
        dialect: options.dialect,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        input: options.input,
    };
    // Unit E01-F1: the pinned route reports the same enumerated
    // denominator the configured driver does, so macro-expanded and K&R
    // definitions are counted here too.
    let inventory = enumerate_source_definitions(&unit);
    if library == "chunkio" && matches!(file.as_ref(), "cio_file.c" | "cio_file_unix.c") {
        let evidence = quirin_source_analysis::chunkio::file_attempts::check_file_attempt_sources(
            &unit,
            PINNED_FLUENT_BIT_COMMIT,
            file == "cio_file_unix.c",
        )?;
        for function in &evidence {
            say!(
                out,
                "qcore-result-use {}: Checked ({} source paths; OS cleanup success and custody Unknown)",
                function.source().function(),
                function.source().paths().len()
            );
        }
        teardown_results.push(
            serde_json::json!({"kind":"filesystem_call_results", "file":file, "evidence":evidence}),
        );
    }
    if library == "chunkio" && file == "cio_file.c" {
        let (native, counter, close, delete) = filesystem_deps
            .as_ref()
            .ok_or("missing filesystem dependencies")?;
        let certificate =
            quirin_source_analysis::chunkio::unmap_wrapper::check_unmap_wrapper_source(
                &unit,
                PINNED_FLUENT_BIT_COMMIT,
                native,
                counter,
            )?;
        say!(
            out,
            "qcore-outcomes munmap_file: Checked ({} source cases; sync/log frames assumed, OS custody conditional)",
            certificate.cases().len()
        );
        let close_evidence = quirin_source_analysis::chunkio::file_close::check_file_close_source(
            &unit,
            PINNED_FLUENT_BIT_COMMIT,
            &certificate,
            close,
        )?;
        say!(
            out,
            "qcore-custody cio_file_close: {} conditional no-delete cases, {} unresolved nested cleanup cases; deletion paths uncovered",
            close_evidence.cases().len(),
            close_evidence
                .cases()
                .iter()
                .enumerate()
                .filter(|(i, _)| close_evidence.require_nested_discharge(*i).is_err())
                .count()
        );
        let deleting =
            quirin_source_analysis::chunkio::file_close::check_file_close_deletion_source(
                &unit,
                PINNED_FLUENT_BIT_COMMIT,
                &certificate,
                close,
                delete,
            )?;
        say!(
            out,
            "qcore-custody cio_file_close delete: {} conditional cases, {} uncovered source paths across both modes",
            deleting.deleting_cases().len(),
            deleting.uncovered_paths().len()
        );
        teardown_results
            .push(serde_json::json!({"kind":"filesystem_close_with_delete", "evidence":deleting}));
        teardown_results.push(
            serde_json::json!({"kind":"filesystem_close_no_delete", "evidence":close_evidence}),
        );
        teardown_results.push(
            serde_json::json!({"kind":"filesystem_unmap_wrapper", "certificate":certificate}),
        );
    }
    if library == "chunkio" && file == "cio_file_unix.c" {
        let deletion = quirin_source_analysis::chunkio::file_delete::check_native_delete_source(
            &unit,
            PINNED_FLUENT_BIT_COMMIT,
        )?;
        say!(
            out,
            "qcore-outcomes cio_file_native_delete: Checked ({} eligibility/outcome cases; unlink framing assumed)",
            deletion.cases().len()
        );
        teardown_results
            .push(serde_json::json!({"kind":"filesystem_delete_outcomes", "certificate":deletion}));
        let delete = quirin_source_analysis::chunkio::file_attempts::check_file_delete_source(
            &unit,
            PINNED_FLUENT_BIT_COMMIT,
        )?;
        say!(
            out,
            "qcore-result-use cio_file_native_delete: Checked ({} short-circuit source paths; deletion custody Unknown)",
            delete.source().paths().len()
        );
        teardown_results
            .push(serde_json::json!({"kind":"filesystem_delete_attempt", "evidence":delete}));
        let certificates =
            quirin_source_analysis::chunkio::file_outcomes::check_file_outcome_sources(
                &unit,
                PINNED_FLUENT_BIT_COMMIT,
            )?;
        for certificate in &certificates {
            say!(
                out,
                "qcore-outcomes {}: Checked ({} null/absent/failure/success cases; OS custody conditional)",
                certificate.source().function(),
                certificate.cases().len()
            );
        }
        teardown_results.push(
            serde_json::json!({"kind":"filesystem_native_outcomes", "certificates":certificates}),
        );
    }
    if library == "chunkio" && file == "cio_chunk.c" {
        let unlink = quirin_source_analysis::chunkio::teardown_source::check_list_unlink_source(
            &unit,
            PINNED_FLUENT_BIT_COMMIT,
        )?;
        say!(
            out,
            "qcore-contract mk_list_del + __mk_list_del: Checked ({} neighbor alias cases; live reciprocal/exclusive links assumed)",
            unlink.cases().len()
        );
        teardown_results.push(serde_json::json!({"kind":"list_unlink", "status":"checked_source_links", "certificate":unlink}));
        let certificates = quirin_source_analysis::chunkio::teardown_source::check_counter_sources(
            &unit,
            PINNED_FLUENT_BIT_COMMIT,
        )?;
        say!(
            out,
            "qcore-contract ChunkIO counters: Checked ({} complete unsigned unit updates; nonwrapping entry bounds and ABI binding assumed)",
            certificates.len()
        );
        teardown_results.push(serde_json::json!({"kind": "unsigned_counters", "status": "checked_source_updates", "certificates": certificates}));
        let dependencies = quirin_source_analysis::chunkio::memory_close::MemoryCloseDependencies {
            reset: chunk_helpers[0].clone(),
            backend: memory_backend
                .as_ref()
                .ok_or("missing memory backend dependency")?
                .clone(),
            unlink: unlink.clone(),
            counter: certificates
                .iter()
                .find(|certificate| {
                    certificate.field() == "total_chunks" && certificate.delta() == -1
                })
                .ok_or("missing total decrement dependency")?
                .clone(),
        };
        let close = quirin_source_analysis::chunkio::memory_close::check_memory_close_source(
            &unit,
            PINNED_FLUENT_BIT_COMMIT,
            &dependencies,
        )?;
        say!(
            out,
            "qcore-contract cio_chunk_close memory scope: Checked ({} null/memory paths; {} source paths uncovered; enclosing stream/context teardown open)",
            close.checked_paths().len(),
            close.uncovered_paths().len()
        );
        nested_memory = Some(close.clone());
        teardown_results.push(serde_json::json!({"kind":"memory_chunk_close", "status":"checked_null_and_memory_paths", "certificate":close}));
        let file = quirin_source_analysis::chunkio::filesystem_chunk_close::check_filesystem_chunk_close_source(
            &unit, PINNED_FLUENT_BIT_COMMIT, &dependencies,
            filesystem_close.as_ref().ok_or("missing filesystem close dependency")?)?;
        say!(
            out,
            "qcore-custody cio_chunk_close filesystem: {} conditional cases, {} unresolved nested cases, {} combined uncovered paths (unsupported backend kinds)",
            file.cases().len(),
            (0..file.cases().len())
                .filter(|i| file.require_nested_discharge(*i).is_err())
                .count(),
            file.combined_uncovered_paths().len()
        );
        filesystem_chunk = Some(file.clone());
        let unsupported =
            quirin_source_analysis::chunkio::unsupported_close::check_unsupported_close_source(
                &file,
            )?;
        if file.combined_uncovered_paths() != [unsupported.source_path()] {
            return Err("chunk close path partition has additional unclassified paths".into());
        }
        say!(
            out,
            "qcore-custody cio_chunk_close unsupported backend: Unknown (complete symbolic int domain excluding {:?}; outer release preserves unresolved backend custody)",
            unsupported.excluded_kinds()
        );
        teardown_results
            .push(serde_json::json!({"kind":"unsupported_chunk_close", "evidence":unsupported}));
        teardown_results
            .push(serde_json::json!({"kind":"filesystem_chunk_close", "evidence":file}));
    }
    if library == "chunkio" && matches!(file.as_ref(), "cio_chunk.c" | "cio_stream.c") {
        use quirin_source_analysis::chunkio::traversal::{TraversalKind, check_traversal_source};
        let kind = if file == "cio_chunk.c" {
            TraversalKind::Chunks
        } else {
            TraversalKind::Streams
        };
        let evidence = check_traversal_source(&unit, PINNED_FLUENT_BIT_COMMIT, kind)?;
        match kind {
            TraversalKind::Chunks => chunk_traversal = Some(evidence.clone()),
            TraversalKind::Streams => stream_traversal = Some(evidence.clone()),
        }
        say!(
            out,
            "qcore-traversal {}: complete typed reference body checked; ring provenance and child custody remain premises",
            evidence.source().function()
        );
        teardown_results.push(
            serde_json::json!({"kind":"teardown_traversal", "file":file, "evidence":evidence}),
        );
    }
    if library == "chunkio" && matches!(file.as_ref(), "cio_stream.c" | "chunkio.c") {
        use quirin_source_analysis::chunkio::parent_teardown::{ParentKind, check_parent_source};
        let kind = if file == "cio_stream.c" {
            ParentKind::Stream
        } else {
            ParentKind::Context
        };
        let evidence = check_parent_source(&unit, PINNED_FLUENT_BIT_COMMIT, kind)?;
        match kind {
            ParentKind::Stream => stream_parent = Some(evidence.clone()),
            ParentKind::Context => context_parent = Some(evidence.clone()),
        }
        say!(
            out,
            "qcore-parent {}: complete allocation-release body checked; child effects and allocation provenance remain premises",
            evidence.source().function()
        );
        teardown_results
            .push(serde_json::json!({"kind":"parent_teardown", "file":file, "evidence":evidence}));
    }
    if library == "chunkio" && file == "cio_memfs.c" {
        let certificate = memory_backend
            .as_ref()
            .ok_or("missing memory backend certificate")?;
        say!(
            out,
            "qcore-contract cio_memfs_close: Checked ({} nullable custody states; borrowed root/provenance/libc premises assumed; enclosing chunk teardown open)",
            certificate.cases().len()
        );
        teardown_results.push(serde_json::json!({"function": "cio_memfs_close", "status": "checked_field_custody", "certificate": certificate}));
    }
    if library == "fluent-bit-chunk" || (library == "chunkio" && file == "cio_chunk.c") {
        audit_chunk_source(
            &unit,
            library,
            &file,
            chunk_helpers,
            &mut native_results,
            source_contracts,
            provider,
            out,
        )?;
    }
    project_unit_into_with_cross_unit(
        library,
        &file,
        &unit,
        &inventory,
        contracts,
        homes,
        &mut report,
        &mut counts,
        out,
    )?;
    Ok(UnitProducts {
        report,
        counts,
        native_results,
        teardown_results,
        captures: ChunkioCaptures {
            nested_memory,
            chunk_traversal,
            stream_traversal,
            stream_parent,
            context_parent,
            filesystem_chunk,
        },
    })
}

/// The library postlude: every check the sequential `audit_library` made
/// after its unit loop.
fn finish_library(
    library: &str,
    captures: ChunkioCaptures,
    teardown_results: &mut Vec<Value>,
    out: &mut String,
) -> Result<(), String> {
    let ChunkioCaptures {
        nested_memory,
        chunk_traversal,
        stream_traversal,
        stream_parent,
        context_parent,
        filesystem_chunk,
    } = captures;
    if library == "chunkio" {
        let certificate =
            quirin_source_analysis::chunkio::nested_memory::check_nested_memory_source(
                chunk_traversal.as_ref().ok_or("missing chunk traversal")?,
                stream_traversal
                    .as_ref()
                    .ok_or("missing stream traversal")?,
                stream_parent.as_ref().ok_or("missing stream parent")?,
                context_parent.as_ref().ok_or("missing context parent")?,
                nested_memory.as_ref().ok_or("missing memory close")?,
            )?;
        say!(
            out,
            "qcore-nested-memory: complete source dependency chain checked; native caller/ring provenance remains Unknown; filesystem children rejected"
        );
        teardown_results
            .push(serde_json::json!({"kind":"nested_memory_source", "certificate":certificate}));
        use quirin_source_analysis::chunkio::nested_cleanup::{
            ChildCase, ConditionalStream, check_nested_cleanup_source,
        };
        let filesystem = filesystem_chunk
            .as_ref()
            .ok_or("missing filesystem chunk close")?;
        let cleanup = check_nested_cleanup_source(&certificate, filesystem)?;
        let mut conditional_examples = Vec::new();
        for (case, c) in filesystem
            .cases()
            .iter()
            .enumerate()
            .filter(|(_, c)| c.chunk_present && !c.delete)
        {
            conditional_examples.push(cleanup.project(
                1,
                &[ConditionalStream {
                    stream: 1,
                    chunks: vec![
                        ChildCase::Memory { chunk: 1 },
                        ChildCase::Filesystem { chunk: 2, case },
                    ],
                }],
                2,
                c.up_counter_decrements,
            )?);
        }
        say!(
            out,
            "qcore-nested-cleanup: {} mixed-backend conditional cases, {} retain child OS/counter obligations after parent free; native entry/outcome binding remains Unknown",
            conditional_examples.len(),
            conditional_examples
                .iter()
                .filter(|c| c.require_conditional_discharge().is_err())
                .count()
        );
        teardown_results.push(serde_json::json!({"kind":"nested_cleanup_source", "source":cleanup, "conditional_examples":conditional_examples}));
        let unsupported =
            quirin_source_analysis::chunkio::unsupported_close::check_unsupported_close_source(
                filesystem,
            )?;
        let extended = cleanup.with_unsupported(&unsupported)?;
        let mut examples = Vec::new();
        for kind in [i32::MIN, -1, 2, i32::MAX] {
            let case = extended.project(
                1,
                &[
                    ConditionalStream {
                        stream: 10,
                        chunks: vec![ChildCase::Memory { chunk: 11 }],
                    },
                    ConditionalStream {
                        stream: 20,
                        chunks: vec![ChildCase::Unsupported { chunk: 21, kind }],
                    },
                ],
                2,
                0,
            )?;
            if case.require_conditional_discharge().is_ok() || case.opaque_residuals().len() != 1 {
                return Err("unsupported parent teardown erased opaque child custody".into());
            }
            examples.push(case);
        }
        say!(
            out,
            "qcore-nested-unsupported: {} conditional examples retain exact opaque child identities after parent release; constructor reachability Unknown",
            examples.len()
        );
        teardown_results.push(serde_json::json!({"kind":"nested_unsupported_source", "source":extended, "conditional_examples":examples}));
    }
    Ok(())
}

/// Both modes consume the same snapshotted policy configuration. Explicit
/// process selection requires verified packages and never falls back to direct
/// acceptance. The old environment variable remains a package-path alias only.
enum SourceContractConfiguration {
    Direct(SourceContractPolicies),
    Process(SourceContractRegistry),
}

impl SourceContractConfiguration {
    fn frame_helper_policy_inventory(
        &self,
    ) -> std::collections::BTreeMap<&str, &[FrameHelperPolicy]> {
        match self {
            Self::Direct(policies) => policies.frame_helper_policy_inventory(),
            Self::Process(registry) => registry.frame_helper_policy_inventory(),
        }
    }

    fn frame_helpers_for_library(&self, library: &str) -> &[FrameHelperPolicy] {
        match self {
            Self::Direct(policies) => policies.frame_helpers_for_library(library),
            Self::Process(registry) => registry.frame_helpers_for_library(library),
        }
    }
}

fn source_contract_configuration() -> Result<SourceContractConfiguration, String> {
    let configured = std::env::var_os("QCORE_SOURCE_CONTRACTS");
    let legacy_package = std::env::var_os("QCORE_CHUNKIO_PACKAGE");
    if configured.is_some() && legacy_package.is_some() {
        return Err("QCORE_SOURCE_CONTRACTS and QCORE_CHUNKIO_PACKAGE cannot both select source-contract packages".into());
    }
    let path = configured.as_ref().map(PathBuf::from).unwrap_or_else(|| {
        quirin_source_analysis::source_profiles::profiles_root()
            .join(FLUENT_BIT_SOURCE_SET)
            .join("source-contracts.json")
    });
    if let Some(package) = legacy_package {
        if package.is_empty() {
            return Err("QCORE_CHUNKIO_PACKAGE cannot be empty".into());
        }
        let package = PathBuf::from(package);
        let package = if package.is_absolute() {
            package
        } else {
            std::env::current_dir()
                .map_err(|error| error.to_string())?
                .join(package)
        };
        Ok(SourceContractConfiguration::Process(
            SourceContractRegistry::load_with_packages(&path, &[package])
                .map_err(|error| error.to_string())?,
        ))
    } else if configured.is_some() {
        Ok(SourceContractConfiguration::Process(
            SourceContractRegistry::load(&path).map_err(|error| error.to_string())?,
        ))
    } else {
        Ok(SourceContractConfiguration::Direct(
            SourceContractPolicies::load(&path).map_err(|error| error.to_string())?,
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn audit_chunk_source(
    unit: &C11TranslationUnit<'_>,
    library: &str,
    file: &str,
    helpers: &[ChunkFrameSummary],
    native_results: &mut Vec<serde_json::Value>,
    source_contracts: &SourceContractConfiguration,
    provider: &Mutex<()>,
    out: &mut String,
) -> Result<(), String> {
    let process_mode = matches!(source_contracts, SourceContractConfiguration::Process(_));
    let results = import_chunk_source_with_helpers(unit, PINNED_FLUENT_BIT_COMMIT, helpers)?;
    let mut summaries = 0;
    let mut checked_contracts = 0;
    for result in &results {
        match result {
            ChunkSourceResult::Summary(summary) => {
                summaries += 1;
                say!(
                    out,
                    "  chunk-source {library}/{file}/{}: field-summary {:?}; {:?}",
                    summary.function(),
                    summary.field(),
                    summary.paths()
                );
                let checked_source = summary.source_checked();
                let certificate = match source_contracts {
                    SourceContractConfiguration::Process(registry) => {
                        // One provider execution at a time, as in the
                        // sequential loop: the process provider records its
                        // last execution in shared state and runs under a
                        // wall-clock limit (card W44-AUDIT-PARALLEL-UNITS).
                        let serial = provider.lock().unwrap_or_else(PoisonError::into_inner);
                        let verified = registry
                            .verify(checked_source, &format!("{library}/{file}"))
                            .map_err(|error| {
                                format!(
                                    "process source-contract evidence rejected for {}: {error}",
                                    summary.function()
                                )
                            })?;
                        drop(serial);
                        let checked = verified.certificate;
                        native_results.push(serde_json::json!({
                            "kind": "provider_source_effect", "library": library, "file": file,
                            "function": summary.function(), "status": "checked_function_contract",
                            "task": checked.task(), "witness": checked.witness(),
                            "evidence": checked.evidence(), "bounds": checked.obligation_bounds(),
                            "import_identity": checked_source.input_identity(),
                        }));
                        say!(
                            out,
                            "  qcore-provider-source {}: Checked (four Boolean-state combinations; explicit live/exclusive/domain assumptions)",
                            summary.function()
                        );
                        verified.evaluation
                    }
                    SourceContractConfiguration::Direct(policies) => {
                        let model_library = checked_source
                            .input_identity()
                            .model_library()
                            .ok_or("checked source lacks model library identity")?;
                        let contract_source = policies
                            .contract_for_library(model_library)
                            .map_err(|error| error.to_string())?;
                        (*check_chunk_source_contract(summary, contract_source)?).clone()
                    }
                };
                checked_contracts += 1;
                native_results.push(serde_json::json!({
                    "library": library, "file": file, "function": summary.function(),
                    "status": "checked", "summary": summary,
                    "contract_source": certificate.contract_source(),
                    "checked_states": certificate.checked_states(),
                }));
                say!(
                    out,
                    "  qcore-contract {}: Checked ({} Boolean pre-states; live/exclusive/domain preconditions assumed)",
                    certificate.function(),
                    certificate.checked_states()
                );
            }
            // Card REASON-CODES-B3: both sidecars carry the closed §L21.1
            // category beside the free text. It is the category the importer
            // classified (REASON-CODES-B2), not a third vocabulary re-derived
            // here, and it is fail-closed - `null` for a reason the closed
            // vocabulary does not recognise, never a defaulted category. The
            // key is emitted only on `unknown` rows, so `checked` rows keep
            // their exact bytes.
            ChunkSourceResult::Unknown {
                function,
                reason,
                reason_code,
            } => {
                let reason_code = reason_code.map(|category| category.as_str());
                if process_mode {
                    native_results.push(serde_json::json!({ "kind": "provider_source_effect", "library": library, "file": file,
                        "function": function, "status": "unknown", "reason": reason, "reason_code": reason_code }));
                }
                native_results.push(serde_json::json!({ "library": library, "file": file, "function": function, "status": "unknown", "reason": reason, "reason_code": reason_code }));
                say!(
                    out,
                    "  chunk-source {library}/{file}/{function}: Unknown: {reason}"
                )
            }
        }
    }
    say!(
        out,
        "chunk-source {library}/{file}: {} function(s), {summaries} field summary(s), {checked_contracts} QCore contract(s) checked, {} Unknown; no whole-unit lifecycle certificate",
        results.len(),
        results.len() - summaries
    );
    Ok(())
}

fn audit_monkey_callback_source(
    entries: &[CompileEntry],
    report: Option<&Path>,
    binding_directory: Option<&Path>,
) -> Result<(), String> {
    let entry = entries
        .iter()
        .find(|entry| entry.file.ends_with("mk_core/mk_event.c"))
        .ok_or("missing Monkey adapter translation unit")?;
    let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
    let options = compiler_options(entry)?;
    let unit = C11TranslationUnit {
        source: &source,
        include_paths: options.include_paths,
        definitions: options.definitions,
        preprocessor_options: options.preprocessor_options,
        dialect: options.dialect,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        input: options.input,
    };
    let results = import_monkey_callbacks(&unit, PINNED_FLUENT_BIT_COMMIT)?;
    for result in &results {
        match result {
            MonkeySourceResult::Summary(summary) => println!(
                "  monkey-libevent/{}: {} source access path(s); custody, failure paths, and runtime access refinement Unknown",
                summary.function(),
                summary.paths().len()
            ),
            MonkeySourceResult::Unknown { function, reason } => {
                println!("  monkey-libevent/{function}: callback source Unknown: {reason}")
            }
        }
    }
    let wrappers = quirin_source_analysis::libevent::monkey_source::import_monkey_wrappers(
        &unit,
        PINNED_FLUENT_BIT_COMMIT,
    )?;
    for result in &wrappers {
        match result {
            MonkeySourceResult::Summary(summary) if summary.has_scan_loop() => println!(
                "  monkey-wrapper/{}: {} structured source branch(es) with scan loop; loop bounds, arithmetic, termination and custody Unknown",
                summary.function(),
                summary.paths().len()
            ),
            MonkeySourceResult::Summary(summary) => println!(
                "  monkey-wrapper/{}: {} complete symbolic source path(s); opaque calls and custody Unknown",
                summary.function(),
                summary.paths().len()
            ),
            MonkeySourceResult::Unknown { function, reason } => {
                println!("  monkey-wrapper/{function}: source Unknown: {reason}")
            }
        }
    }
    let binding = if let Some(directory) = binding_directory {
        let read =
            |name| fs::read_to_string(directory.join(name)).map_err(|error| error.to_string());
        let bound = quirin_source_analysis::libevent::monkey_binding::check_monkey_bindings(
            &results,
            &read("monkey_observation.jsonl")?,
            &read("monkey_bindings.jsonl")?,
            include_str!(
                "../../../providers/qcore-provider-libevent/contracts/monkey/composition.qv"
            ),
        )?;
        for callback in &bound.callbacks {
            println!(
                "  monkey-libevent/{}: {} bound native execution(s), checked paths {:?}, Unknown paths {:?}; {:?}",
                callback.function,
                callback.checked_executions,
                callback.checked_paths,
                callback.unknown_paths,
                callback.unknown_reasons
            );
        }
        fs::write(
            directory.join("monkey_binding_result.json"),
            serde_json::to_string_pretty(&bound).map_err(|e| e.to_string())? + "\n",
        )
        .map_err(|e| e.to_string())?;
        Some(bound)
    } else {
        None
    };
    let callback_coverage =
        quirin_source_analysis::libevent::monkey_binding::account_monkey_callbacks(
            &results,
            binding.as_ref(),
        )?;
    for callback in &callback_coverage.callbacks {
        println!(
            "  monkey-libevent/{}: Cycle 18 callback composition {:?}",
            callback.function, callback.cycle18_composition
        );
    }
    if let Some(report) = report {
        let output = serde_json::json!({
            "schema_version": 3,
            "scan_loop_semantics": "Each iteration evaluates header, then matches; a true match executes early_return and exits the function. Otherwise store increment in the induction local and repeat. A false header continues outside the loop. Iteration symbols are scoped to one iteration. Bounds, arithmetic, termination and memory safety remain obligations.",
            "source_revision": PINNED_FLUENT_BIT_COMMIT,
            "compile_entry": { "directory": entry.directory, "command": entry.command, "file": entry.file },
            "source_projection_only": true,
            "source_policy_contract": include_str!("../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"),
            "wrapper_behavior_specification": include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/wrapper_paths.qv"),
            "wrapper_behavior_specification_status": "syntax-preserved source findings; custody and behavioral predicate replay not certified",
            "callbacks": results,
            "callback_coverage": callback_coverage,
            "wrappers": wrappers,
        });
        fs::write(
            report,
            serde_json::to_string_pretty(&output).map_err(|error| error.to_string())? + "\n",
        )
        .map_err(|error| format!("could not write {}: {error}", report.display()))?;
    }
    Ok(())
}

fn audit_monkey_libevent_contract(fluent_root: &Path) -> Result<(), String> {
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("audit crate must be inside the QCore-C repository")?;
    let reference_root = repository_root.join(LIBEVENT_SUBMODULE_PATH);
    let libevent = load_profile(LIBEVENT_SOURCE_SET)
        .map_err(|error| format!("Libevent source-set profile unavailable: {error}"))?;
    let libevent_library = libevent
        .library(LIBEVENT_SOURCE_SET)
        .ok_or("the libevent profile declares no `libevent` library")?;
    let libevent_pin = libevent
        .pin_for(LIBEVENT_SOURCE_SET)
        .ok_or("the libevent library binds no pin")?;
    verify_pinned_libevent_source(&reference_root, libevent_pin, libevent_library)
        .map_err(|error| format!("could not verify Cycle 18 Libevent source: {error}"))?;

    let bundled_root = fluent_root.join("lib/monkey/mk_core/deps/libevent");
    let trusted_files = [
        "include/event2/event.h",
        "event.c",
        "bufferevent.c",
        "include/event2/thread.h",
        "include/event2/util.h",
        "evutil.c",
        "epoll.c",
    ];
    for relative in trusted_files {
        let reference = fs::read(reference_root.join(relative))
            .map_err(|error| format!("could not read reference {relative}: {error}"))?;
        let bundled = fs::read(bundled_root.join(relative))
            .map_err(|error| format!("could not read bundled {relative}: {error}"))?;
        if bundled != reference {
            return Err(format!(
                "Monkey's bundled Libevent differs in Cycle 18 contract file {relative}"
            ));
        }
    }

    let adapter_path = fluent_root.join("lib/monkey/mk_core/mk_event_libevent.c");
    let adapter = fs::read_to_string(&adapter_path)
        .map_err(|error| format!("could not read {}: {error}", adapter_path.display()))?;
    let symbols = prefixed_call_symbols(&adapter);
    let mut accepted = Vec::new();
    let mut unknown = Vec::new();
    for symbol in symbols {
        match classify_libevent_symbol(&libevent_library.api_dispositions, &symbol) {
            quirin_source_analysis::libevent::ProfileDisposition::Accepted => accepted.push(symbol),
            quirin_source_analysis::libevent::ProfileDisposition::Unknown => unknown.push(symbol),
        }
    }
    println!(
        "monkey-libevent adapter: {} accepted Cycle 18 API symbol(s): {}",
        accepted.len(),
        accepted.join(", ")
    );
    println!(
        "monkey-libevent adapter: {} symbol(s) remain Unknown: {}",
        unknown.len(),
        unknown.join(", ")
    );
    println!(
        "monkey bundled Libevent: {} Cycle 18 contract file(s) byte-identical",
        trusted_files.len()
    );
    Ok(())
}

fn prefixed_call_symbols(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut symbols = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let start = cursor;
        if bytes[cursor].is_ascii_alphabetic() || bytes[cursor] == b'_' {
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            let symbol = &source[start..cursor];
            let mut following = cursor;
            while bytes.get(following).is_some_and(u8::is_ascii_whitespace) {
                following += 1;
            }
            if (symbol.starts_with("event_") || symbol.starts_with("evutil_"))
                && bytes.get(following) == Some(&b'(')
            {
                symbols.push(symbol.to_owned());
            }
        } else {
            cursor += 1;
        }
    }
    symbols.sort();
    symbols.dedup();
    symbols
}

/// Card AUDIT-CANONICAL-PATHS: the pinned route's unit selection over a
/// database spelled through a symlink to a moved inputs root (run 4 of
/// W48-BU-SINGLE-LEAF-BODIES selected no unit and failed before analysis).
#[cfg(all(test, unix))]
mod canonical_selection_tests {
    use super::*;
    use std::os::unix::fs::symlink;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "qcore-pinned-selection-{name}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(path.join("real/src/sub")).unwrap();
            for file in ["src/a.c", "src/b.c", "src/sub/c.c", "src/notes.txt"] {
                fs::write(path.join("real").join(file), "int x;\n").unwrap();
            }
            symlink(path.join("real"), path.join("old")).unwrap();
            Self(path.canonicalize().unwrap())
        }

        fn real(&self) -> PathBuf {
            self.0.join("real")
        }

        fn old(&self) -> PathBuf {
            self.0.join("old")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn entry(directory: &Path, file: &Path) -> CompileEntry {
        CompileEntry {
            directory: directory.to_path_buf(),
            command: format!("clang -c {}", file.display()),
            file: file.to_path_buf(),
        }
    }

    fn files(selected: &[CompileEntry]) -> Vec<PathBuf> {
        selected.iter().map(|entry| entry.file.clone()).collect()
    }

    #[test]
    fn the_old_spelling_selects_the_units_of_the_real_root_and_keeps_its_spelling() {
        let scratch = Scratch::new("old");
        let (real, old) = (scratch.real(), scratch.old());
        let spelled = [
            "src/b.c",
            "src/a.c",
            "src/sub/c.c",
            "src/notes.txt",
            "src/a.c",
        ]
        .map(|file| entry(&old, &old.join(file)));
        let root = real.join("src");
        assert_eq!(
            files(&select_top_level_units(&spelled, &root).unwrap()),
            [
                old.join("src/a.c"),
                old.join("src/a.c"),
                old.join("src/b.c")
            ]
        );
        assert_eq!(
            files(&select_recursive_units(&spelled, &root).unwrap()),
            [
                old.join("src/a.c"),
                old.join("src/b.c"),
                old.join("src/sub/c.c")
            ]
        );
        assert_eq!(
            files(&select_exact_unit(&spelled, &real, &real.join("src/sub/c.c")).unwrap()),
            [old.join("src/sub/c.c")]
        );
        // The published relative path is the one the real spelling gives.
        assert_eq!(
            relative_within(&old.join("src/sub/c.c"), &root),
            Some(PathBuf::from("sub/c.c"))
        );
    }

    #[test]
    fn escaping_dangling_and_aliased_records_are_refused() {
        let scratch = Scratch::new("refuse");
        let (real, old) = (scratch.real(), scratch.old());
        let root = real.join("src");
        fs::create_dir_all(scratch.0.join("outside")).unwrap();
        fs::write(scratch.0.join("outside/evil.c"), "int x;\n").unwrap();
        symlink(scratch.0.join("outside/evil.c"), real.join("src/evil.c")).unwrap();
        let escaping = [entry(&old, &old.join("src/evil.c"))];
        let error = select_top_level_units(&escaping, &root).unwrap_err();
        assert!(
            error.starts_with("source is outside audit root: "),
            "{error}"
        );
        assert!(select_recursive_units(&escaping, &root).is_err());
        assert!(select_exact_unit(&escaping, &real, &real.join("src/evil.c")).is_err());
        // A dangling record under the root is selected, so it stays in the
        // denominator; the unit is refused when it is read.
        let dangling = [entry(&old, &old.join("src/missing.c"))];
        assert_eq!(
            files(&select_recursive_units(&dangling, &root).unwrap()),
            [old.join("src/missing.c")]
        );
        assert_eq!(
            relative_within(&old.join("src/missing.c"), &root),
            Some(PathBuf::from("missing.c"))
        );
        // Two spellings of one unit: refused, never merged by the dedup.
        let aliased = [
            entry(&real, &real.join("src/a.c")),
            entry(&real, &old.join("src/a.c")),
        ];
        for error in [
            select_top_level_units(&aliased, &root).unwrap_err(),
            select_recursive_units(&aliased, &root).unwrap_err(),
            select_exact_unit(&aliased, &real, &real.join("src/a.c")).unwrap_err(),
        ] {
            assert!(
                error.starts_with(crate::input_paths::ALIAS_DIAGNOSTIC),
                "{error}"
            );
        }
    }
}
