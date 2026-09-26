//! Card **W45-AUDIT-CHECKPOINTS**: the checkpointed `quirin-audit current`
//! run — divide and conquer by library group.
//!
//! `--checkpoint-dir D` analyses the route's groups **one at a time**, in plan
//! order. Each group's units run on up to `--jobs` threads and are folded in
//! unit order exactly as the whole run folds them; the group is then sealed
//! into `D/<label>.qo` ([`crate::checkpoint`]) and every piece of its state is
//! dropped before the next group starts, so the run never holds more than one
//! group's intermediate results. The report is then composed from the
//! checkpoints by the streaming composer ([`crate::compose`]), which reads one
//! group's rows at a time.
//!
//! `--resume` reuses a group's checkpoint only when its whole `identity`
//! section equals the identity recomputed now: the analyzer build, the contract
//! set, the configuration, the environment, the revision and, per unit, the
//! compile configuration (compile-task identity, raw and preprocessed digests)
//! and the preparation facts, which needs each unit's Clang preparation but
//! none of its analysis. Anything else — a missing, refused or differing
//! checkpoint — recomputes the group, and the reason is reported. A reused
//! group's sections are the recomputed group's sections, byte for byte,
//! whenever the identity is equal (`docs/reports/w45-audit-checkpoints/`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, prepare_translation_unit,
};
use qcore_schemas::sha256_hex;
use serde_json::{Value, json};

use crate::checkpoint::{
    AnalyzerIdentity, CategoryCount, CertificateRecord, CertificatesSection, Checkpoint,
    CheckpointReader, CompilerIdentity, ContractSetIdentity, ContractsSection, CountsRecord,
    EnvironmentValue, GlobalSection, GroupIdentity, HeldCallRecord, HeldFunction,
    IdentitySection, ModuleDigest, PendingMarkerRecord, PlanSection, PlannedGroup,
    PreparationFacts, ProvenanceSection, ResultSection, RowMembers, RowStepSection, RunIdentity,
    UnitFingerprint, UnitKey, exact_json, json_sha256, split_row,
};
use crate::compile_db::{EffectiveInput, compile_task_identity, compiler_options};
use crate::current::CurrentAudit;
use crate::current_route::{ConfiguredTask, CurrentRoute, TaskInput, evaluate_task, task_dispatch};
use crate::parallel::run_ordered;
use crate::pending_review::HeldCalls;
use crate::projection::AuditCounts;
use crate::report::UnitRefusal;

/// The report's revision key, as [`CurrentAudit::finish`] writes it.
pub(crate) const REVISION_KEY: &str = "fluent_bit_revision";
/// How often a group's resident set size is sampled.
const RSS_SAMPLE_INTERVAL: Duration = Duration::from_millis(10);
/// The run record written beside the checkpoints (provenance only).
pub const RUN_RECORD: &str = "checkpoint-run.json";

/// Where checkpoints go, and whether matching ones are reused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointOptions {
    pub directory: PathBuf,
    pub resume: bool,
}

/// The route's plan, as `run_current_route` built it.
pub(crate) struct RoutePlan<'a> {
    pub preamble: Vec<String>,
    pub headers: Vec<String>,
    /// Per group, the group-wide refusal its units inherit.
    pub unresolved: Vec<Option<&'a str>>,
    pub tasks: Vec<ConfiguredTask<'a>>,
}

/// One group of the plan.
struct Group<'a> {
    label: String,
    header: String,
    unresolved: Option<String>,
    tasks: Vec<ConfiguredTask<'a>>,
}

/// Run the route group by group; `Ok(true)` when any unit hit a gateway
/// failure, as [`crate::current_route::run_current_route`].
pub(crate) fn run(
    route: &CurrentRoute<'_>,
    audit: &CurrentAudit,
    plan: RoutePlan<'_>,
    options: &CheckpointOptions,
) -> Result<bool, String> {
    let census = qcore_c_gateway::active_census_sinks();
    if !census.is_empty() {
        return Err(format!(
            "checkpoints refuse a census run: census streams {census:?} are process-global and \
             record in analysis order, so a group's checkpoint could not stand for its share of them"
        ));
    }
    let groups = split_groups(plan.headers, plan.unresolved, plan.tasks)?;
    fs::create_dir_all(&options.directory)
        .map_err(|error| format!("{}: {error}", options.directory.display()))?;
    let run_identity = run_identity(route, audit)?;
    let global = global_section(route, audit, plan.preamble)?;
    let planned = planned_groups(audit, &groups)?;
    warn_stale(&options.directory, &groups);
    let mut paths = Vec::new();
    let mut records = Vec::new();
    for (index, group) in groups.into_iter().enumerate() {
        let path = options.directory.join(format!("{}.qo", group.label));
        let plan_section = PlanSection {
            group: index as u64,
            groups: planned.clone(),
        };
        let started = Instant::now();
        let sampler = RssSampler::start();
        let decision = if options.resume {
            reuse_decision(&path, &run_identity, &group, audit, route.jobs)
        } else {
            Err("resume not requested".to_owned())
        };
        let record = match decision {
            Ok(()) => {
                let content = reuse(&path, &plan_section, &global)?;
                let peak = sampler.finish();
                eprintln!("checkpoint {}: reused (identity equal)", group.label);
                json!({
                    "group": group.label, "decision": "reused", "reason": "identity equal",
                    "wall_time_ms": started.elapsed().as_millis() as u64,
                    "sampled_peak_rss_kib": peak, "content_sha256": content,
                })
            }
            Err(reason) => {
                let label = group.label.clone();
                let checkpoint = compute_group(
                    route,
                    audit,
                    group,
                    &run_identity,
                    plan_section,
                    &global,
                    started,
                )?;
                let content = checkpoint.content_sha256()?;
                let peak = sampler.finish();
                let mut checkpoint = checkpoint;
                checkpoint.provenance.sampled_peak_rss_kib = peak;
                checkpoint.provenance.process_peak_rss_kib = status_kib("VmHWM:");
                checkpoint.provenance.wall_time_ms = started.elapsed().as_millis() as u64;
                write_atomically(&path, &checkpoint.to_bytes()?)?;
                eprintln!("checkpoint {label}: recomputed ({reason})");
                json!({
                    "group": label, "decision": "recomputed", "reason": reason,
                    "wall_time_ms": checkpoint.provenance.wall_time_ms,
                    "sampled_peak_rss_kib": peak, "content_sha256": content,
                })
            }
        };
        records.push(record);
        paths.push(path);
    }
    if crate::checkout_head(audit.checkout())? != audit.revision() {
        return Err("checkout HEAD changed during audit; rerun with stable inputs".into());
    }
    let summary = crate::compose::compose(
        &paths,
        &crate::compose::ComposeOutputs {
            report: route.diagnostic_report,
            monkey_source_report: route.monkey_source_report,
            binding_directory: route.binding_directory,
        },
        crate::compose::Console::TotalsOnly,
    )?;
    let record = json!({
        "schema_version": 1,
        "scope": "provenance of one checkpointed run: which groups were reused or recomputed and why, with measured wall time and sampled peak memory; not an input to any verdict",
        "jobs": route.jobs,
        "process_peak_rss_kib": status_kib("VmHWM:"),
        "groups": records,
    });
    let mut bytes = serde_json::to_vec_pretty(&record).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    let run_record = options.directory.join(RUN_RECORD);
    fs::write(&run_record, bytes).map_err(|error| format!("{}: {error}", run_record.display()))?;
    Ok(summary.gateway_failures != 0)
}

/// Split the route's tasks into their groups; every label must be a plain,
/// distinct file name.
fn split_groups<'a>(
    headers: Vec<String>,
    unresolved: Vec<Option<&'a str>>,
    tasks: Vec<ConfiguredTask<'a>>,
) -> Result<Vec<Group<'a>>, String> {
    let mut groups = headers
        .into_iter()
        .zip(unresolved)
        .map(|(header, unresolved)| Group {
            label: String::new(),
            header,
            unresolved: unresolved.map(str::to_owned),
            tasks: Vec::new(),
        })
        .collect::<Vec<_>>();
    for task in tasks {
        let group = groups
            .get_mut(task.group)
            .ok_or("a task names a group the plan does not have")?;
        group.label = task.library.to_owned();
        group.tasks.push(task);
    }
    let mut seen = BTreeSet::new();
    for group in &groups {
        let label = &group.label;
        if group.tasks.is_empty() {
            return Err(format!(
                "group `{}` selects no unit; a checkpoint holds one or more units",
                group.header
            ));
        }
        let plain = !label.is_empty()
            && !label.starts_with('.')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
        if !plain {
            return Err(format!(
                "group label `{label}` is not a plain file name ([A-Za-z0-9._-], not starting with `.`); it names its checkpoint"
            ));
        }
        if !seen.insert(label.clone()) {
            return Err(format!("two groups are labelled `{label}`"));
        }
    }
    Ok(groups)
}

/// Each group's units, keyed; a unit in two groups is refused, because the
/// composed report could not be a partition of the run.
fn planned_groups(audit: &CurrentAudit, groups: &[Group<'_>]) -> Result<Vec<PlannedGroup>, String> {
    let mut owner = BTreeMap::<UnitKey, &str>::new();
    let mut planned = Vec::new();
    for group in groups {
        let mut units = Vec::new();
        for task in &group.tasks {
            let key = unit_key(audit, task)?;
            if let Some(other) = owner.insert(key.clone(), &group.label) {
                return Err(format!(
                    "unit {} is selected by groups `{other}` and `{}`; checkpointed composition needs disjoint groups",
                    key.expected_source, group.label
                ));
            }
            units.push(key);
        }
        planned.push(PlannedGroup {
            label: group.label.clone(),
            header: group.header.clone(),
            units,
        });
    }
    Ok(planned)
}

fn unit_key(audit: &CurrentAudit, task: &ConfiguredTask<'_>) -> Result<UnitKey, String> {
    let (expected_source, record) = match (&task.expected_source, task.input) {
        (_, TaskInput::Absent(file)) => (
            audit.checkout().join(file).display().to_string(),
            json!(["absent", file]),
        ),
        (Ok(expected), TaskInput::Configured(entry)) => (
            expected.display().to_string(),
            json!(["configured", entry.file, entry.directory, entry.command]),
        ),
        (Err(error), TaskInput::Configured(entry)) => (
            String::new(),
            json!(["unplaced", error, entry.file, entry.directory, entry.command]),
        ),
    };
    Ok(UnitKey {
        expected_source,
        record_sha256: json_sha256(&record)?,
    })
}

fn warn_stale(directory: &Path, groups: &[Group<'_>]) {
    let labels = groups
        .iter()
        .map(|group| format!("{}.qo", group.label))
        .collect::<BTreeSet<_>>();
    if let Ok(entries) = fs::read_dir(directory) {
        let mut stale = entries
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".qo") && !labels.contains(name))
            .collect::<Vec<_>>();
        stale.sort();
        for name in stale {
            eprintln!(
                "checkpoint {name}: not in this run's plan; left untouched (composing it with this run's checkpoints refuses)"
            );
        }
    }
}

/// The analyzer build, the configuration, the contract set and the
/// environment shared by every group.
fn run_identity(route: &CurrentRoute<'_>, audit: &CurrentAudit) -> Result<RunIdentity, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let binary = fs::read(&executable)
        .map_err(|error| format!("could not read the analyzer {}: {error}", executable.display()))?;
    let dependency = |name: &str| {
        audit
            .dependencies()
            .get(name)
            .cloned()
            .ok_or_else(|| format!("the audit has no `{name}` dependency digest"))
    };
    let mut environment = Vec::new();
    for (name, value) in std::env::vars_os() {
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with("QCORE_") || name == crate::parallel::JOBS_VARIABLE {
            continue;
        }
        let value = value
            .into_string()
            .map_err(|_| format!("environment variable {name} is not UTF-8; checkpoints need it exactly"))?;
        environment.push(EnvironmentValue {
            name: name.to_owned(),
            value,
        });
    }
    environment.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(RunIdentity {
        route: "current".into(),
        analyzer: AnalyzerIdentity {
            package: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
            build_profile: if cfg!(debug_assertions) { "dev" } else { "release" }.into(),
            build_commit: option_env!("QCORE_AUDIT_BUILD_COMMIT").map(str::to_owned),
            binary_sha256: sha256_hex(&binary),
        },
        mode: if route.mode.is_contracts_only() {
            "contracts-only"
        } else {
            "configured"
        }
        .into(),
        checkout: audit.checkout().display().to_string(),
        source_revision: audit.revision().to_owned(),
        source_profile_sha256: audit.source_profile_sha256().to_owned(),
        contracts: ContractSetIdentity {
            manifest_sha256: dependency("resource_contract_manifest")?,
            sources_sha256: dependency("resource_contract_sources")?,
            provider_selection_sha256: route
                .contracts
                .provider_record()
                .map(json_sha256)
                .transpose()?,
            evidence_sha256: audit.report_evidence().map(json_sha256).transpose()?,
            locks_sha256: None,
        },
        compiler: CompilerIdentity {
            program: audit.compiler().program().to_owned(),
            version: audit.compiler().version().to_owned(),
            target: audit.compiler().target().to_owned(),
        },
        environment,
    })
}

fn global_section(
    route: &CurrentRoute<'_>,
    audit: &CurrentAudit,
    preamble: Vec<String>,
) -> Result<GlobalSection, String> {
    let markers = crate::pending_review::markers(audit.contracts());
    Ok(GlobalSection {
        console_preamble: preamble,
        source_revision: audit.revision().to_owned(),
        revision_key: REVISION_KEY.into(),
        report_evidence_json: audit.report_evidence().map(exact_json).transpose()?,
        assumptions_json: exact_json(audit.assumptions())?,
        pending_declarations: markers.declarations as u64,
        pending_markers: markers
            .markers
            .into_iter()
            .map(|marker| PendingMarkerRecord {
                function: marker.function,
                scope: marker.scope,
                clause: marker.clause,
                reason: marker.reason,
                owner: marker.owner,
                module: marker.module,
                module_sha256: marker.module_sha256,
            })
            .collect(),
        historical_reference_revision: route.historical_reference_revision.to_owned(),
    })
}

/// `Ok` when the checkpoint at `path` may be reused; otherwise the reason it
/// may not.
fn reuse_decision(
    path: &Path,
    run: &RunIdentity,
    group: &Group<'_>,
    audit: &CurrentAudit,
    jobs: usize,
) -> Result<(), String> {
    if !path.exists() {
        return Err("no checkpoint".into());
    }
    let mut reader = CheckpointReader::open(path).map_err(|error| format!("checkpoint refused: {error}"))?;
    reader
        .verify()
        .map_err(|error| format!("checkpoint refused: {error}"))?;
    let stored = reader
        .section::<IdentitySection>("identity")
        .map_err(|error| format!("checkpoint refused: {error}"))?;
    drop(reader);
    if let Some(field) = run_difference(&stored.run, run) {
        return Err(format!("{field} differs"));
    }
    if stored.group.label != group.label {
        return Err("group label differs".into());
    }
    if stored.group.unresolved.as_deref() != group.unresolved.as_deref() {
        return Err("group-wide refusal differs".into());
    }
    if stored.group.units.len() != group.tasks.len() {
        return Err(format!(
            "unit count differs ({} recorded, {} now)",
            stored.group.units.len(),
            group.tasks.len()
        ));
    }
    let mut current = Vec::with_capacity(group.tasks.len());
    let tasks = group.tasks.iter().collect::<Vec<_>>();
    run_ordered(
        tasks,
        jobs,
        None,
        |task| fingerprint_task(audit, task),
        |fingerprint| {
            current.push(fingerprint);
            Ok::<(), String>(())
        },
    )?;
    for (index, (recorded, now)) in stored.group.units.iter().zip(current).enumerate() {
        let now = now.map_err(|error| format!("unit {index}: {error}"))?;
        if *recorded != now {
            let what = if recorded.file != now.file {
                "unit file"
            } else if recorded.configuration_sha256 != now.configuration_sha256 {
                "compile configuration"
            } else {
                "preparation"
            };
            return Err(format!("unit {index} ({}): {what} differs", now.file));
        }
    }
    Ok(())
}

/// The first run-identity field that differs, named for the report.
fn run_difference(recorded: &RunIdentity, now: &RunIdentity) -> Option<&'static str> {
    if recorded.route != now.route {
        Some("route")
    } else if recorded.analyzer != now.analyzer {
        Some("analyzer build")
    } else if recorded.mode != now.mode {
        Some("evidence mode")
    } else if recorded.checkout != now.checkout {
        Some("checkout")
    } else if recorded.source_revision != now.source_revision {
        Some("source revision")
    } else if recorded.source_profile_sha256 != now.source_profile_sha256 {
        Some("source profile")
    } else if recorded.contracts != now.contracts {
        Some("contract set")
    } else if recorded.compiler != now.compiler {
        Some("compiler")
    } else if recorded.environment != now.environment {
        Some("environment")
    } else {
        None
    }
}

/// A unit's fingerprint without its analysis: the compile configuration and
/// preparation facts [`CurrentAudit::evaluate_entry`] and
/// [`CurrentAudit::evaluate_absent`] record, computed the same way.
fn fingerprint_task(audit: &CurrentAudit, task: &ConfiguredTask<'_>) -> Result<UnitFingerprint, String> {
    let checkout = audit.checkout();
    let revision = audit.revision();
    match (&task.expected_source, task.input) {
        (_, TaskInput::Absent(relative)) => {
            let path = checkout.join(relative);
            let bytes = fs::read(&path);
            let refusal = match &bytes {
                Ok(bytes) => match crate::verify_pinned_bytes(checkout, revision, &path, bytes) {
                    Ok(()) => UnitRefusal::environment(format!(
                        "selected source has no compilation database record: {relative}"
                    )),
                    Err(refusal) => refusal,
                },
                Err(error) => UnitRefusal::environment(format!(
                    "selected source is absent from the checkout: {relative}: {error}"
                )),
            };
            let mut configuration = json!({
                "source_revision": revision, "file": Value::Null, "expected_source": path,
                "directory": Value::Null, "command": Value::Null, "compile_record": Value::Null,
                "raw_source_sha256": bytes.as_ref().ok().map(|bytes| sha256_hex(bytes)),
                "compiler": audit.compiler(), "dependencies": audit.dependencies(),
                "unresolved": refusal.reason(),
            });
            configuration["inventory_sha256"] = json!(json_sha256(&configuration)?);
            Ok(UnitFingerprint {
                file: (*relative).to_owned(),
                configuration_sha256: json_sha256(&configuration)?,
                preparation: PreparationFacts::NotPrepared,
            })
        }
        (Err(error), TaskInput::Configured(_)) => Err(error.clone()),
        (Ok(expected_source), TaskInput::Configured(entry)) => {
            let expected_source = expected_source
                .canonicalize()
                .map_err(|error| error.to_string())?;
            let relative = expected_source
                .strip_prefix(checkout)
                .map_err(|_| "source outside current checkout")?;
            let file = crate::repository_path(relative);
            let bytes =
                fs::read(&entry.file).map_err(|error| UnitRefusal::environment(error.to_string()));
            let mut configuration = json!({
                "source_revision": revision, "file": entry.file, "expected_source": expected_source,
                "directory": entry.directory, "command": entry.command,
                "raw_source_sha256": bytes.as_ref().ok().map(|bytes| sha256_hex(bytes)),
                "compiler": audit.compiler(), "dependencies": audit.dependencies(),
            });
            let source = bytes
                .as_ref()
                .ok()
                .and_then(|bytes| std::str::from_utf8(bytes).ok());
            let prepared = (|| -> Result<C11TranslationUnit<'_>, UnitRefusal> {
                let bytes = bytes.as_ref().map_err(Clone::clone)?;
                crate::verify_pinned_bytes(checkout, revision, &expected_source, bytes)?;
                let source = source.ok_or_else(|| UnitRefusal::classified("source is not UTF-8"))?;
                if let Some(reason) = task.unresolved {
                    return Err(UnitRefusal::classified(reason));
                }
                let options = compiler_options(entry)?;
                Ok(C11TranslationUnit {
                    source,
                    include_paths: options.include_paths,
                    definitions: options.definitions,
                    preprocessor_options: options.preprocessor_options,
                    dialect: options.dialect,
                    function_scope: LegacyFunctionScope::MainSourceDefinitions,
                    input: options.input,
                })
            })();
            let preparation = match prepared {
                Ok(unit) => {
                    let prepared = prepare_translation_unit(&unit);
                    let preparation_error = prepared.validation_error().map(ToString::to_string);
                    let identity = compile_task_identity(
                        entry,
                        &unit,
                        match prepared.preprocessed_source() {
                            Ok(source) => EffectiveInput::Accepted(source),
                            Err(_) => EffectiveInput::Rejected(
                                preparation_error
                                    .as_deref()
                                    .expect("rejected preparation has an error"),
                            ),
                        },
                        audit.compiler(),
                        audit.dependencies().clone(),
                    )?;
                    configuration["compile_task_identity"] =
                        serde_json::to_value(&identity).map_err(|error| error.to_string())?;
                    PreparationFacts::of(&prepared)
                }
                Err(refusal) => {
                    configuration["unresolved"] = json!(refusal.reason());
                    PreparationFacts::NotPrepared
                }
            };
            configuration["inventory_sha256"] = json!(json_sha256(&configuration)?);
            Ok(UnitFingerprint {
                file,
                configuration_sha256: json_sha256(&configuration)?,
                preparation,
            })
        }
    }
}

/// Reuse the checkpoint at `path`: print its console lines and, when this
/// run's plan or run-wide inputs differ from those it records, re-seal it with
/// this run's (its step sections and provenance unchanged). Returns its
/// content digest.
fn reuse(path: &Path, plan: &PlanSection, global: &GlobalSection) -> Result<String, String> {
    let mut reader = CheckpointReader::open(path)?;
    let result = reader.section::<crate::checkpoint::ResultSection>("result")?;
    println!("{}", result.header);
    for output in &result.outputs {
        print!("{output}");
    }
    drop(result);
    let stored_plan = reader.section::<PlanSection>("plan")?;
    let stored_global = reader.section::<GlobalSection>("global")?;
    if stored_plan == *plan && stored_global == *global {
        return reader.content_sha256();
    }
    let mut sections = Vec::new();
    for name in crate::checkpoint::SECTIONS {
        let bytes = match name {
            "plan" => postcard::to_stdvec(plan).map_err(|error| error.to_string())?,
            "global" => postcard::to_stdvec(global).map_err(|error| error.to_string())?,
            other => reader.raw(other)?,
        };
        sections.push((name, bytes));
    }
    drop(reader);
    let bytes = crate::checkpoint::assemble(sections)?;
    write_atomically(path, &bytes)?;
    CheckpointReader::open(path)?.content_sha256()
}

/// Analyse one group, printing its console lines as the whole run does, and
/// build its checkpoint.
fn compute_group(
    route: &CurrentRoute<'_>,
    audit: &CurrentAudit,
    group: Group<'_>,
    run: &RunIdentity,
    plan: PlanSection,
    global: &GlobalSection,
    started: Instant,
) -> Result<Checkpoint, String> {
    let Group {
        label,
        header,
        unresolved,
        tasks,
    } = group;
    println!("{header}");
    let mut builder = GroupBuilder::default();
    let dispatch = task_dispatch(&tasks);
    run_ordered(
        tasks,
        route.jobs,
        Some(dispatch),
        |task| evaluate_task(audit, &task),
        |outcome| {
            print!("{}", outcome.output);
            let products = outcome.result?;
            builder.absorb(outcome.output, products)
        },
    )?;
    let GroupBuilder {
        units,
        preparation,
        lowering,
        verdicts,
        certificates,
        held,
        diagnostic_counts,
        counts,
        sources,
        outputs,
    } = builder;
    let contracts = audit.contracts();
    let markers = crate::pending_review::markers(contracts);
    let AuditCounts {
        units: unit_count,
        data_only_units,
        source_functions,
        projected_functions,
        safe_units,
        unsafe_units,
        unknown_units,
        gateway_failures,
        absent_from_raw_scan_definitions,
        unanalyzable_definitions,
        enumeration_refused_units,
        withheld_safe_units,
        prepared_units,
        preparation_clang_invocations,
        plain_preprocessing_elapsed_us,
        syntax_validation_elapsed_us,
        line_marked_preprocessing_elapsed_us,
    } = counts;
    let count = |value: usize| value as u64;
    Ok(Checkpoint {
        identity: IdentitySection {
            run: run.clone(),
            group: GroupIdentity {
                label: label.clone(),
                unresolved,
                units,
            },
        },
        plan,
        global: global.clone(),
        preparation: RowStepSection { units: preparation },
        lowering: RowStepSection { units: lowering },
        verdicts: RowStepSection { units: verdicts },
        certificates: CertificatesSection {
            units: certificates,
        },
        contracts: ContractsSection {
            manifest_sha256: run.contracts.manifest_sha256.clone(),
            sources_sha256: run.contracts.sources_sha256.clone(),
            modules: contracts
                .source_assumptions()
                .iter()
                .map(|(module, text)| ModuleDigest {
                    module: module.clone(),
                    sha256: sha256_hex(text.as_bytes()),
                })
                .collect(),
            provider_selection_json: route
                .contracts
                .provider_record()
                .map(exact_json)
                .transpose()?,
            evidence_mode: run.mode.clone(),
            locks_sha256: None,
            pending_declarations: markers.declarations as u64,
            pending_markers: markers.markers.len() as u64,
            held_calls: held
                .call_sites
                .iter()
                .map(|(callee, sites)| HeldCallRecord {
                    callee: callee.clone(),
                    call_sites: *sites as u64,
                    functions: held
                        .functions
                        .get(callee)
                        .into_iter()
                        .flatten()
                        .map(|(library, file, function)| HeldFunction {
                            library: library.clone(),
                            file: file.clone(),
                            function: function.clone(),
                        })
                        .collect(),
                })
                .collect(),
        },
        result: ResultSection {
            header,
            outputs,
            counts: CountsRecord {
                units: count(unit_count),
                data_only_units: count(data_only_units),
                source_functions: count(source_functions),
                projected_functions: count(projected_functions),
                safe_units: count(safe_units),
                unsafe_units: count(unsafe_units),
                unknown_units: count(unknown_units),
                gateway_failures: count(gateway_failures),
                absent_from_raw_scan_definitions: count(absent_from_raw_scan_definitions),
                unanalyzable_definitions: count(unanalyzable_definitions),
                enumeration_refused_units: count(enumeration_refused_units),
                withheld_safe_units: count(withheld_safe_units),
                prepared_units: count(prepared_units),
                preparation_clang_invocations: count(preparation_clang_invocations),
            },
            diagnostic_counts: diagnostic_counts
                .into_iter()
                .map(|(category, count)| CategoryCount {
                    category,
                    count: count as u64,
                })
                .collect(),
            sources: sources.into_iter().collect(),
        },
        provenance: ProvenanceSection {
            wall_time_ms: started.elapsed().as_millis() as u64,
            sampled_peak_rss_kib: None,
            process_peak_rss_kib: None,
            rss_sample_interval_ms: RSS_SAMPLE_INTERVAL.as_millis() as u64,
            jobs: route.jobs as u64,
            plain_preprocessing_elapsed_us: plain_preprocessing_elapsed_us as u64,
            syntax_validation_elapsed_us: syntax_validation_elapsed_us as u64,
            line_marked_preprocessing_elapsed_us: line_marked_preprocessing_elapsed_us as u64,
        },
    })
}

/// A group's step sections, accumulated unit by unit in unit order. Each
/// unit's report row is split into its step members as soon as it is
/// consumed, so only the members' text is retained.
#[derive(Default)]
struct GroupBuilder {
    units: Vec<UnitFingerprint>,
    preparation: Vec<RowMembers>,
    lowering: Vec<RowMembers>,
    verdicts: Vec<RowMembers>,
    certificates: Vec<CertificateRecord>,
    held: HeldCalls,
    diagnostic_counts: BTreeMap<String, usize>,
    counts: AuditCounts,
    sources: BTreeSet<String>,
    outputs: Vec<String>,
}

impl GroupBuilder {
    fn absorb(
        &mut self,
        output: String,
        products: crate::current::EntryProducts,
    ) -> Result<(), String> {
        let (source, fragment, counts, preparation) = products.into_parts();
        let [row] = fragment.unit_rows() else {
            return Err(format!(
                "a configuration recorded {} report rows; a checkpoint unit is exactly one",
                fragment.unit_rows().len()
            ));
        };
        let file = row["file"]
            .as_str()
            .ok_or("a unit row has no `file`")?
            .to_owned();
        let configuration = row
            .get("compile_configuration")
            .ok_or("a unit row has no `compile_configuration`")?;
        self.units.push(UnitFingerprint {
            file: file.clone(),
            configuration_sha256: json_sha256(configuration)?,
            preparation,
        });
        let [preparation, lowering, verdicts] = split_row(row)?;
        self.preparation.push(preparation);
        self.lowering.push(lowering);
        self.verdicts.push(verdicts);
        let status = row["status"].as_str().unwrap_or_default();
        let reason = row["reason"].as_str();
        let refused = reason.filter(|reason| reason.starts_with("certificate context binding refused"));
        self.certificates.push(CertificateRecord {
            file,
            outcome: if status == "Safe" {
                "published-safe"
            } else if refused.is_some() {
                "binding-refused"
            } else {
                "not-published"
            }
            .into(),
            compile_task_identity_json_sha256: configuration
                .get("compile_task_identity")
                .map(json_sha256)
                .transpose()?,
            reason: refused.map(str::to_owned),
        });
        self.held.absorb(HeldCalls::from_rows(fragment.unit_rows()));
        for (category, count) in fragment.diagnostic_count_map() {
            *self.diagnostic_counts.entry(category.clone()).or_default() += count;
        }
        self.counts.absorb(counts);
        if let Some(source) = source {
            self.sources.insert(source.display().to_string());
        }
        self.outputs.push(output);
        Ok(())
    }
}

/// Write `bytes` to `path` through a sibling temporary file and a rename, so
/// no reader ever sees a partial checkpoint at `path`.
pub(crate) fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("qo.partial");
    fs::write(&temporary, bytes).map_err(|error| format!("{}: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| format!("{}: {error}", path.display()))
}

/// One `/proc/self/status` field in KiB (`VmRSS:`, `VmHWM:`); `None` where the
/// platform has no such file.
pub(crate) fn status_kib(field: &str) -> Option<u64> {
    fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find(|line| line.starts_with(field))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

/// Samples the process's resident set size while a group runs. A sampler
/// never resets the kernel's high-water mark, so an outer `/usr/bin/time`
/// still measures the whole process.
struct RssSampler {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<Option<u64>>>,
}

impl RssSampler {
    fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = std::thread::Builder::new()
            .name("qo-rss-sampler".into())
            .spawn(move || {
                let mut peak = status_kib("VmRSS:");
                while !flag.load(Ordering::Relaxed) {
                    std::thread::sleep(RSS_SAMPLE_INTERVAL);
                    peak = peak.max(status_kib("VmRSS:"));
                }
                peak
            })
            .ok();
        Self { stop, handle }
    }

    fn finish(mut self) -> Option<u64> {
        self.stop.store(true, Ordering::Relaxed);
        self.handle.take()?.join().ok().flatten()
    }
}

impl Drop for RssSampler {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
