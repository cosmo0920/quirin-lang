//! Per-configuration `quirin-audit current` checkpoints.
//!
//! Every planned compile configuration becomes one complete `.qo`. A worker
//! seals its file before the ordered consumer publishes console output, so a
//! slow earlier unit does not prevent later completed units surviving an
//! interruption. Composition checks the complete plan before publication and
//! folds the one-unit files in route order. A resumed unit is reused only after
//! its run identity and freshly prepared unit fingerprint match.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
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
    AnalyzerIdentity, BinaryValue, CategoryCount, CertificateOutcome, CertificateRecord,
    CertificatesSection, Checkpoint, CheckpointReader, CompilerIdentity, ContractSetIdentity,
    ContractsSection, CountsRecord, EnvironmentValue, GlobalSection, GroupIdentity, HeldCallRecord,
    HeldFunction, IdentitySection, LockDigest, ModuleDigest, PendingMarkerRecord, PlanSection,
    PlannedGroup, PreparationFacts, ProvenanceSection, ResultSection, RowMembers, RowStepSection,
    RunIdentity, UnitFingerprint, UnitKey, exact_json, json_sha256, split_row,
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
#[derive(Clone)]
pub(crate) struct RoutePlan<'a> {
    pub preamble: Vec<String>,
    pub headers: Vec<String>,
    /// Per group, the group-wide refusal its units inherit.
    pub unresolved: Vec<Option<&'a str>>,
    pub tasks: Vec<ConfiguredTask<'a>>,
}

/// One group of the route plan, or one expanded checkpoint unit.
struct Group<'a> {
    label: String,
    header: String,
    unresolved: Option<String>,
    tasks: Vec<ConfiguredTask<'a>>,
}

/// Run units independently, then compose; `Ok(true)` when any hit a gateway
/// failure, as [`crate::current_route::run_current_route`].
pub(crate) fn run(
    route: &CurrentRoute<'_>,
    audit: &CurrentAudit,
    plan: RoutePlan<'_>,
    options: &CheckpointOptions,
) -> Result<(bool, qcore_schemas::contract_refutation::RefutationTable), String> {
    let census = qcore_c_gateway::active_census_sinks();
    if !census.is_empty() {
        return Err(format!(
            "checkpoints refuse a census run: census streams {census:?} are process-global and \
             record in analysis order, so a group's checkpoint could not stand for its share of them"
        ));
    }
    // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14j): a `.qo`
    // checkpoint carries no bounds rows, body evidence or bounds environment,
    // and its identity names neither the bounds environment nor its
    // refutation table, so a reused or composed run would omit the rows the
    // selection asks for. Refused, never silently partial.
    // Card NULL-SAFETY-PROPERTY: likewise for the null selection.
    if std::env::var_os(crate::projection::NULL_SELECTION).is_some() {
        return Err(format!(
            "checkpoints refuse the null selection ({}): a checkpoint carries no null rows and \
             its identity names no null environment, so a reused or composed run would omit the \
             requested rows; run without --checkpoints",
            crate::projection::NULL_SELECTION
        ));
    }
    // Card OUTER-CONTRACT (rule 31): likewise for the runtime projection.
    if crate::outer_selection::selected() {
        return Err(format!(
            "checkpoints refuse the runtime projection ({}): a checkpoint carries no outer \
             environment or reach and its identity names neither, so a reused or composed run \
             would omit the requested projection; run without --checkpoints",
            crate::outer_selection::PROJECTION
        ));
    }
    if std::env::var_os(crate::projection::BOUNDS_SELECTION).is_some() {
        return Err(format!(
            "checkpoints refuse the bounds selection ({}): a checkpoint carries no bounds rows, \
             body evidence or bounds environment and its identity names none of them, so a \
             reused or composed run would omit the requested rows; run without --checkpoints",
            crate::projection::BOUNDS_SELECTION
        ));
    }
    let groups = split_units(
        audit,
        split_groups(plan.headers, plan.unresolved, plan.tasks)?,
    )?;
    fs::create_dir_all(&options.directory)
        .map_err(|error| format!("{}: {error}", options.directory.display()))?;
    let revision_template = exact_json(
        &serde_json::to_value(crate::contract_revision::template(
            audit.contracts(),
            audit.pins(),
        ))
        .map_err(|error| error.to_string())?,
    )?;
    // Card MEMBER-PREPASS-OTHER-ROUTES: the run's member facts, digested once
    // for every unit's identity.
    let member_facts_sha256 = audit
        .member_prepass()
        .map(|prepass| crate::member_prepass::facts_sha256(&prepass.facts))
        .transpose()?;
    let run_identity = run_identity(
        route,
        audit,
        &revision_template,
        member_facts_sha256.clone(),
    )?;
    let global = global_section(
        route,
        audit,
        plan.preamble,
        revision_template,
        member_facts_sha256,
    )?;
    let planned = planned_groups(audit, &groups)?;
    warn_stale(&options.directory, &groups);
    let mut paths = Vec::with_capacity(groups.len());
    let mut records = Vec::with_capacity(groups.len());
    let dispatch = task_dispatch(
        &groups
            .iter()
            .flat_map(|group| group.tasks.iter().cloned())
            .collect::<Vec<_>>(),
    );
    run_ordered(
        groups.into_iter().enumerate().collect(),
        route.jobs,
        Some(dispatch),
        |(index, group)| {
            let path = options.directory.join(format!("{}.qo", group.label));
            let plan_section = PlanSection {
                group: index as u64,
                groups: planned.clone(),
            };
            let started = Instant::now();
            let sampler = RssSampler::start();
            let decision = if options.resume {
                reuse_decision(&path, &run_identity, &group, audit, 1)
            } else {
                Err("resume not requested".to_owned())
            };
            let (decision, reason, content) = match decision {
                Ok(()) => (
                    "reused",
                    "identity equal".to_owned(),
                    reuse(&path, &plan_section, &global)?,
                ),
                Err(reason) => {
                    let mut checkpoint = compute_group(
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
                    checkpoint.provenance.sampled_peak_rss_kib = peak;
                    checkpoint.provenance.process_peak_rss_kib = status_kib("VmHWM:");
                    checkpoint.provenance.wall_time_ms = started.elapsed().as_millis() as u64;
                    write_atomically(&path, &checkpoint.to_bytes()?)?;
                    eprintln!("checkpoint ready: {}", path.display());
                    let result = checkpoint.result;
                    return Ok::<_, String>((
                        path,
                        result,
                        json!({
                            "group": checkpoint.identity.group.label, "decision": "recomputed",
                            "reason": reason, "wall_time_ms": checkpoint.provenance.wall_time_ms,
                            "sampled_peak_rss_kib": peak, "content_sha256": content,
                        }),
                    ));
                }
            };
            let mut reader = CheckpointReader::open(&path)?;
            let result = reader.section::<ResultSection>("result")?;
            eprintln!("checkpoint ready: {}", path.display());
            let peak = sampler.finish();
            Ok((
                path,
                result,
                json!({
                    "group": group.label, "decision": decision, "reason": reason,
                    "wall_time_ms": started.elapsed().as_millis() as u64,
                    "sampled_peak_rss_kib": peak, "content_sha256": content,
                }),
            ))
        },
        |outcome: Result<(PathBuf, ResultSection, Value), String>| {
            let (path, result, record) = outcome?;
            if !result.header.is_empty() {
                println!("{}", result.header);
            }
            for output in &result.outputs {
                print!("{output}");
            }
            eprintln!(
                "checkpoint {}: {} ({})",
                record["group"].as_str().unwrap_or_default(),
                record["decision"].as_str().unwrap_or_default(),
                record["reason"].as_str().unwrap_or_default()
            );
            paths.push(path);
            records.push(record);
            Ok::<(), String>(())
        },
    )?;
    if crate::checkout_head(audit.checkout())? != audit.revision() {
        return Err("checkout HEAD changed during audit; rerun with stable inputs".into());
    }
    eprintln!("checkpoint linking {} file(s)", paths.len());
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
        "verdict": summary.verdict,
        "process_peak_rss_kib": status_kib("VmHWM:"),
        "groups": records,
    });
    let mut bytes = serde_json::to_vec_pretty(&record).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    let run_record = options.directory.join(RUN_RECORD);
    fs::write(&run_record, bytes).map_err(|error| format!("{}: {error}", run_record.display()))?;
    // Card W45-REFUTED-CONTRACT-WITHDRAWAL: the composed rows' refutations,
    // from which the route decides whether a second phase is due.
    Ok((summary.gateway_failures != 0, summary.refutations))
}

/// Split route tasks into library groups; every label must be a plain,
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

/// Turn each planned compile configuration into one independently durable file.
/// The full unit key distinguishes multiple compile records of one C source.
fn split_units<'a>(audit: &CurrentAudit, groups: Vec<Group<'a>>) -> Result<Vec<Group<'a>>, String> {
    let mut units = Vec::new();
    for group in groups {
        for (index, task) in group.tasks.into_iter().enumerate() {
            let key = unit_key(audit, &task)?;
            let digest =
                json_sha256(&serde_json::to_value(&key).map_err(|error| error.to_string())?)?;
            units.push(Group {
                label: format!("{}-{digest}", group.label),
                header: if index == 0 {
                    group.header.clone()
                } else {
                    String::new()
                },
                unresolved: group.unresolved.clone(),
                tasks: vec![task],
            });
        }
    }
    Ok(units)
}

/// Each checkpoint unit, keyed; duplicates are refused because the composed
/// report could not be a partition of the run.
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
        (_, TaskInput::NotLinked(file, reason)) => (
            audit.checkout().join(file).display().to_string(),
            json!(["not-linked", file, reason]),
        ),
        (Ok(expected), TaskInput::Configured(entry)) => (
            expected.display().to_string(),
            json!(["configured", entry.file, entry.directory, entry.command]),
        ),
        (Err(error), TaskInput::Configured(entry)) => (
            String::new(),
            json!([
                "unplaced",
                error,
                entry.file,
                entry.directory,
                entry.command
            ]),
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

/// The analyzer build, configuration, contract set and environment shared
/// by every unit.
fn run_identity(
    route: &CurrentRoute<'_>,
    audit: &CurrentAudit,
    revision_template: &str,
    member_facts_sha256: Option<String>,
) -> Result<RunIdentity, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let binary = fs::read(&executable).map_err(|error| {
        format!(
            "could not read the analyzer {}: {error}",
            executable.display()
        )
    })?;
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
        let value = value.into_string().map_err(|_| {
            format!("environment variable {name} is not UTF-8; checkpoints need it exactly")
        })?;
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
            build_profile: if cfg!(debug_assertions) {
                "dev"
            } else {
                "release"
            }
            .into(),
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
            locks: lock_digests(audit),
            pin_evaluation_sha256: sha256_hex(revision_template.as_bytes()),
            plugin_checks_sha256: audit
                .assumptions()
                .get("plugin_checks")
                .map(json_sha256)
                .transpose()?,
            refuted_table_sha256: qcore_schemas::contract_refutation::table_digest(
                audit.refuted_table(),
            ),
        },
        compiler: CompilerIdentity {
            program: audit.compiler().program().to_owned(),
            version: audit.compiler().version().to_owned(),
            target: audit.compiler().target().to_owned(),
        },
        environment,
        member_facts_sha256,
    })
}

/// Every contract lock the pin evaluation read, with its digest.
fn lock_digests(audit: &CurrentAudit) -> Vec<LockDigest> {
    audit
        .pins()
        .lock_digests()
        .into_iter()
        .map(|(path, sha256)| LockDigest { path, sha256 })
        .collect()
}

fn global_section(
    route: &CurrentRoute<'_>,
    audit: &CurrentAudit,
    preamble: Vec<String>,
    revision_template_json: String,
    member_facts_sha256: Option<String>,
) -> Result<GlobalSection, String> {
    let markers = crate::pending_review::markers(audit.contracts());
    // Card MEMBER-PREPASS-OTHER-ROUTES: what the run's fold reads beyond the
    // units' own facts, so the composer folds as the run does.
    let environment = crate::callback_member_fold::FoldEnvironment::of(audit.contracts());
    let member_fold = (!environment.is_empty())
        .then(|| {
            let run = audit.member_prepass().map(|prepass| {
                json!({
                    "facts_sha256": member_facts_sha256,
                    "closures": crate::callback_member_fold::closures_value(
                        &environment.run_closures(&prepass.facts),
                    ),
                    "section": prepass.section,
                })
            });
            BinaryValue::from_json(&json!({
                "environment": environment,
                "run": run,
            }))
        })
        .transpose()?;
    Ok(GlobalSection {
        console_preamble: preamble,
        source_revision: audit.revision().to_owned(),
        revision_key: REVISION_KEY.into(),
        report_evidence: audit
            .report_evidence()
            .map(BinaryValue::from_json)
            .transpose()?,
        assumptions: BinaryValue::from_json(audit.assumptions())?,
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
                category: marker.category,
                module: marker.module,
                module_sha256: marker.module_sha256,
            })
            .collect(),
        revision_template: BinaryValue::from_json(
            &serde_json::from_str(&revision_template_json).map_err(|error| error.to_string())?,
        )?,
        historical_reference_revision: route.historical_reference_revision.to_owned(),
        member_fold,
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
    let mut reader =
        CheckpointReader::open(path).map_err(|error| format!("checkpoint refused: {error}"))?;
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
    } else if recorded.member_facts_sha256 != now.member_facts_sha256 {
        Some("member facts")
    } else {
        None
    }
}

/// A unit's fingerprint without its analysis: the compile configuration and
/// preparation facts [`CurrentAudit::evaluate_entry`] and
/// [`CurrentAudit::evaluate_absent`] record, computed the same way.
fn fingerprint_task(
    audit: &CurrentAudit,
    task: &ConfiguredTask<'_>,
) -> Result<UnitFingerprint, String> {
    let checkout = audit.checkout();
    let revision = audit.revision();
    let unrecorded = match task.input {
        TaskInput::Absent(relative) => Some((relative, None)),
        TaskInput::NotLinked(relative, reason) => Some((relative, Some(reason))),
        TaskInput::Configured(_) => None,
    };
    match (&task.expected_source, task.input) {
        (_, TaskInput::Absent(_) | TaskInput::NotLinked(..)) => {
            let (relative, not_linked) = unrecorded.ok_or("unrecorded task")?;
            let path = checkout.join(relative);
            let bytes = fs::read(&path);
            let (refusal, _) =
                CurrentAudit::unrecorded_refusal(checkout, revision, relative, not_linked, &bytes);
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
                let source =
                    source.ok_or_else(|| UnitRefusal::classified("source is not UTF-8"))?;
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

/// Reuse the checkpoint at `path`. If the plan, global inputs or header
/// position changed, re-seal those sections with the current values while
/// retaining the analysis steps and provenance. Returns the content digest.
fn reuse(path: &Path, plan: &PlanSection, global: &GlobalSection) -> Result<String, String> {
    let mut reader = CheckpointReader::open(path)?;
    let stored_plan = reader.section::<PlanSection>("plan")?;
    let stored_global = reader.section::<GlobalSection>("global")?;
    let mut result = reader.section::<ResultSection>("result")?;
    let position = usize::try_from(plan.group).map_err(|_| "plan position overflows usize")?;
    let header = &plan
        .groups
        .get(position)
        .ok_or("plan position is outside the plan")?
        .header;
    let header_changed = result.header != *header;
    if stored_plan == *plan && stored_global == *global && !header_changed {
        return reader.content_sha256();
    }
    result.header.clone_from(header);
    let mut sections = Vec::new();
    for name in crate::checkpoint::SECTIONS {
        let bytes = match name {
            "plan" => postcard::to_stdvec(plan).map_err(|error| error.to_string())?,
            "global" => postcard::to_stdvec(global).map_err(|error| error.to_string())?,
            "result" if header_changed => {
                postcard::to_stdvec(&result).map_err(|error| error.to_string())?
            }
            other => reader.raw(other)?,
        };
        sections.push((name, bytes));
    }
    drop(reader);
    let bytes = crate::checkpoint::assemble(sections)?;
    write_atomically(path, &bytes)?;
    CheckpointReader::open(path)?.content_sha256()
}

/// Analyse one unit and build its complete checkpoint; publication is ordered later.
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
    let mut builder = GroupBuilder::default();
    let dispatch = task_dispatch(&tasks);
    run_ordered(
        tasks,
        route.jobs,
        Some(dispatch),
        |task| evaluate_task(audit, &task),
        |outcome| {
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
        facts,
        callback_members,
        unit_verdicts,
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
        pending_units,
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
            provider_selection: route
                .contracts
                .provider_record()
                .map(BinaryValue::from_json)
                .transpose()?,
            evidence_mode: run.mode.clone(),
            locks: run.contracts.locks.clone(),
            pending_declarations: markers.declarations as u64,
            pending_markers: markers.markers.len() as u64,
            held_calls: held
                .call_sites
                .iter()
                .map(|(callee, sites)| {
                    held_record(
                        callee,
                        *sites,
                        held.functions.get(callee).into_iter().flatten(),
                    )
                })
                .collect(),
            analysed_functions: facts.analysed.iter().cloned().collect(),
            pin_held_calls: facts
                .held
                .iter()
                .map(|(callee, (sites, rows))| held_record(callee, *sites, rows))
                .collect(),
            callback_members,
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
                pending_units: count(pending_units),
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
            verdict: qcore_schemas::verdict::Verdict::fold(unit_verdicts),
        },
        provenance: ProvenanceSection {
            wall_time_ms: started.elapsed().as_millis() as u64,
            sampled_peak_rss_kib: None,
            process_peak_rss_kib: None,
            rss_sample_interval_ms: RSS_SAMPLE_INTERVAL.as_millis() as u64,
            jobs: route.jobs as u64,
            plain_preprocessing_elapsed_us,
            syntax_validation_elapsed_us,
            line_marked_preprocessing_elapsed_us,
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
    facts: crate::contract_revision::ReportFacts,
    /// Card MEMBER-PREPASS-OTHER-ROUTES: the member and state facts the
    /// units recorded, in unit order.
    callback_members: Vec<BinaryValue>,
    diagnostic_counts: BTreeMap<String, usize>,
    counts: AuditCounts,
    sources: BTreeSet<String>,
    outputs: Vec<String>,
    /// Each unit's verdict, in unit order; the group's is their fold.
    unit_verdicts: Vec<qcore_schemas::verdict::Verdict>,
}

/// One callee's held calls as a checkpoint records them.
fn held_record<'a>(
    callee: &str,
    sites: usize,
    rows: impl IntoIterator<Item = &'a (String, String, String)>,
) -> HeldCallRecord {
    HeldCallRecord {
        callee: callee.to_owned(),
        call_sites: sites as u64,
        functions: rows
            .into_iter()
            .map(|(library, file, function)| HeldFunction {
                library: library.clone(),
                file: file.clone(),
                function: function.clone(),
            })
            .collect(),
    }
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
        let refused =
            reason.filter(|reason| reason.starts_with("certificate context binding refused"));
        self.certificates.push(CertificateRecord {
            file,
            outcome: if status == "Safe" {
                CertificateOutcome::PublishedSafe
            } else if refused.is_some() {
                CertificateOutcome::BindingRefused
            } else {
                CertificateOutcome::NotPublished
            },
            compile_task_identity_json_sha256: configuration
                .get("compile_task_identity")
                .map(json_sha256)
                .transpose()?,
            reason: refused.map(str::to_owned),
        });
        self.held.absorb(HeldCalls::from_rows(fragment.unit_rows()));
        self.facts
            .absorb(crate::contract_revision::ReportFacts::of_rows(
                fragment.unit_rows(),
            ));
        for unit in fragment.callback_member_facts() {
            self.callback_members.push(BinaryValue::from_json(
                &serde_json::to_value(unit).map_err(|error| error.to_string())?,
            )?);
        }
        for (category, count) in fragment.diagnostic_count_map() {
            *self.diagnostic_counts.entry(category.clone()).or_default() += count;
        }
        self.counts.absorb(counts);
        if let Some(source) = source {
            self.sources.insert(source.display().to_string());
        }
        self.outputs.push(output);
        self.unit_verdicts
            .push(crate::checkpoint::unit_verdict(status));
        Ok(())
    }
}

/// Flush a complete sibling temporary file before atomic publication.
/// On Unix, the parent directory is synced after rename. Completed units
/// survive process interruption on every supported host; host-crash durability
/// depends on the filesystem and is only requested through directory sync on Unix.
pub(crate) fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("qo.partial");
    let mut file = fs::File::create(&temporary)
        .map_err(|error| format!("{}: {error}", temporary.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("{}: {error}", temporary.display()))?;
    file.sync_all()
        .map_err(|error| format!("{}: {error}", temporary.display()))?;
    drop(file);
    fs::rename(&temporary, path).map_err(|error| format!("{}: {error}", path.display()))?;
    #[cfg(unix)]
    {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    Ok(())
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
